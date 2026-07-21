pub mod error;
pub mod routes;
pub mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRef},
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER},
    },
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use governor::middleware::NoOpMiddleware;
use serde_json::json;
use tower_governor::{
    GovernorLayer,
    errors::GovernorError,
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::SmartIpKeyExtractor,
};

/// Alias da config de rate limit por IP usada em todo o roteador.
type IpGovernorConfig = GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware>;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    timeout::TimeoutLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::auth::middleware::{require_auth, require_backoffice_auth};
use state::{AppState, AuthMiddlewareState};

type ApiRouter = Router<AppState>;

/// Limite de tamanho do corpo de request (1 MiB) — barra payloads abusivos.
const BODY_LIMIT_BYTES: usize = 1024 * 1024;
/// Timeout por request — barra conexões lentas / handlers travados.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(configured_allow_origin())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    let auth_state = AuthMiddlewareState::from_ref(&state);

    // Rate limit ESTRITO no login (superfície de brute-force): rajada de 5, repõe
    // 1 tentativa a cada 2s por IP. `SmartIpKeyExtractor` usa X-Forwarded-For/Real-IP
    // (proxy) e cai para o IP do peer (ConnectInfo) em dev.
    let login_governor =
        GovernorLayer::new(governor_config(2, 5)).error_handler(rate_limit_response);
    // Rate limit GLOBAL, generoso (defesa em profundidade): rajada de 100, repõe 1 a
    // cada 50ms (~20 req/s sustentadas) por IP.
    let global_governor =
        GovernorLayer::new(governor_config_ms(50, 100)).error_handler(rate_limit_response);

    let public = public_routes().layer(login_governor);
    let protected =
        protected_routes().route_layer(from_fn_with_state(auth_state.clone(), require_auth));
    let backoffice =
        backoffice_routes().route_layer(from_fn_with_state(auth_state, require_backoffice_auth));

    Router::new()
        // Fora do rate limit de login (rajada 5): é chamado pelo Caddy (IP fixo do
        // container) a cada decisão de TLS on-demand — fica sob o limite global.
        .route("/tls/ask", get(routes::tls::ask))
        .merge(public)
        .merge(protected)
        .merge(backoffice)
        // Camadas do mais interno (primeiro) ao mais externo (último):
        // corpo limitado → timeout → rate limit global → trace → CORS (mais externo,
        // para que respostas de erro/429 também carreguem os headers CORS).
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(global_governor)
        .layer(
            TraceLayer::new_for_http()
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(cors)
        .with_state(state)
}

/// Constrói uma config de rate limit por IP: repõe 1 permissão a cada `per_second`
/// segundos, com rajada de `burst`. Encerra o processo se os parâmetros forem inválidos
/// (constantes de compilação — nunca deve acontecer em runtime).
fn governor_config(per_second: u64, burst: u32) -> Arc<IpGovernorConfig> {
    let mut builder = GovernorConfigBuilder::default().key_extractor(SmartIpKeyExtractor);
    builder.per_second(per_second).burst_size(burst);
    finish_governor(builder)
}

/// Como [`governor_config`], mas repõe 1 permissão a cada `per_ms` milissegundos.
fn governor_config_ms(per_ms: u64, burst: u32) -> Arc<IpGovernorConfig> {
    let mut builder = GovernorConfigBuilder::default().key_extractor(SmartIpKeyExtractor);
    builder.per_millisecond(per_ms).burst_size(burst);
    finish_governor(builder)
}

fn finish_governor(
    mut builder: GovernorConfigBuilder<SmartIpKeyExtractor, NoOpMiddleware>,
) -> Arc<IpGovernorConfig> {
    let Some(config) = builder.finish() else {
        panic!("parâmetros de rate limit inválidos");
    };
    let config = Arc::new(config);
    // Limpa periodicamente o estado de IPs inativos para não crescer sem limite.
    let limiter = config.limiter().clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            limiter.retain_recent();
        }
    });
    config
}

/// Resposta para requisições barradas pelo rate limiter.
fn rate_limit_response(err: GovernorError) -> Response {
    match err {
        GovernorError::TooManyRequests { wait_time, .. } => {
            let mut resp = (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "error": "muitas requisições — tente novamente em instantes" })),
            )
                .into_response();
            if let Ok(v) = HeaderValue::from_str(&wait_time.to_string()) {
                resp.headers_mut().insert(RETRY_AFTER, v);
            }
            resp
        }
        GovernorError::UnableToExtractKey => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "não foi possível identificar a origem da requisição" })),
        )
            .into_response(),
        GovernorError::Other { code, msg, .. } => (
            code,
            Json(json!({ "error": msg.unwrap_or_else(|| "erro no limitador".to_string()) })),
        )
            .into_response(),
    }
}

/// Padrões de origem CORS permitidos, lidos de `CORS_ALLOWED_ORIGINS` (lista separada
/// por vírgula, com suporte a um curinga `*` por entrada — ex.: `https://*.finledger.app`).
/// Sem a variável, o padrão libera apenas localhost:3001 (subdomínios inclusos) para dev.
fn cors_allowed_patterns() -> Vec<String> {
    match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => vec![
            "http://localhost:3001".to_string(),
            "http://*.localhost:3001".to_string(),
        ],
    }
}

/// Constrói o `AllowOrigin` do CORS a partir da allowlist configurada. Reflete a origem
/// da requisição apenas quando ela casa com algum padrão — nunca `*`.
fn configured_allow_origin() -> AllowOrigin {
    let patterns = cors_allowed_patterns();
    AllowOrigin::predicate(move |origin: &HeaderValue, _req| {
        origin
            .to_str()
            .ok()
            .is_some_and(|o| patterns.iter().any(|p| origin_matches(o, p)))
    })
}

/// Casa uma origem contra um padrão que pode conter um único curinga `*`
/// (ex.: `https://*.finledger.app` casa `https://acme.finledger.app`).
fn origin_matches(origin: &str, pattern: &str) -> bool {
    match pattern.split_once('*') {
        None => origin == pattern,
        Some((prefix, suffix)) => {
            origin.len() >= prefix.len() + suffix.len()
                && origin.starts_with(prefix)
                && origin.ends_with(suffix)
        }
    }
}

fn public_routes() -> ApiRouter {
    Router::new()
        .route("/auth/login", post(routes::auth::login))
        .route("/backoffice/auth/login", post(routes::backoffice::login))
        // Consulta do composer de endereço da landing (fica sob o rate limit estrito).
        .route(
            "/tenants/{slug}/existe",
            get(routes::tenants_publico::existe),
        )
        // Marca whitelabel para brandizar o login do subdomínio antes do auth.
        .route(
            "/tenants/{slug}/marca",
            get(routes::tenants_publico::marca),
        )
}

fn protected_routes() -> ApiRouter {
    Router::new()
        .merge(auth_routes())
        .merge(catalogo_routes())
        .merge(crm_routes())
        .merge(estoque_routes())
        .merge(vendas_routes())
        .merge(fornecedores_routes())
        .merge(orcamentos_routes())
        .merge(compras_routes())
        .merge(financeiro_routes())
        .merge(fiscal_routes())
        .merge(bi_routes())
        .merge(configuracoes_routes())
}

fn bi_routes() -> ApiRouter {
    Router::new()
        .route("/bi/resumo", get(routes::bi::resumo))
        .route("/bi/financeiro", get(routes::bi::financeiro))
        .route("/bi/comercial", get(routes::bi::comercial))
        .route("/bi/estoque", get(routes::bi::estoque))
        .route("/bi/alertas", get(routes::bi::alertas))
        .route("/bi/alertas/{id}/feedback", post(routes::bi::feedback))
}

fn configuracoes_routes() -> ApiRouter {
    Router::new()
        .route("/configuracoes", get(routes::configuracoes::obter))
        .route("/configuracoes", put(routes::configuracoes::atualizar))
        // Marca whitelabel self-service (escrita só admin, ver handler)
        .route(
            "/configuracoes/marca",
            put(routes::configuracoes::atualizar_marca),
        )
        // Custos fixos discriminados (soma vira o total mensal da precificação)
        .route(
            "/configuracoes/custos-fixos",
            get(routes::configuracoes::listar_custos_fixos)
                .put(routes::configuracoes::definir_custo_fixo),
        )
        .route(
            "/configuracoes/custos-fixos/{nome}",
            delete(routes::configuracoes::remover_custo_fixo),
        )
}

fn auth_routes() -> ApiRouter {
    Router::new()
        .route("/auth/registrar", post(routes::auth::registrar))
        .route("/auth/alterar-senha", post(routes::auth::alterar_senha))
        .route("/auth/usuarios", get(routes::auth::listar_usuarios))
        .route("/auth/usuarios/{id}", get(routes::auth::buscar_usuario))
        .route(
            "/auth/usuarios/{id}/desativar",
            post(routes::auth::desativar_usuario),
        )
        .route(
            "/auth/usuarios/{id}/reativar",
            post(routes::auth::reativar_usuario),
        )
        .route("/auth/usuarios/{id}", put(routes::auth::atualizar_usuario))
}

fn catalogo_routes() -> ApiRouter {
    Router::new()
        .route("/catalogo/produtos", get(routes::catalogo::listar))
        .route("/catalogo/produtos", post(routes::catalogo::cadastrar))
        .route("/catalogo/produtos/{id}", get(routes::catalogo::buscar))
        .route("/catalogo/produtos/{id}", put(routes::catalogo::atualizar))
        .route(
            "/catalogo/produtos/{id}/precos",
            put(routes::catalogo::atualizar_precos),
        )
        .route(
            "/catalogo/produtos/{id}/desativar",
            post(routes::catalogo::desativar),
        )
        .route(
            "/catalogo/produtos/{id}/reativar",
            post(routes::catalogo::reativar),
        )
        // Precificação assistida: margens por categoria, custo fixo por
        // produto, preços da concorrência e elasticidade.
        .route(
            "/catalogo/margens-categoria",
            get(routes::catalogo::listar_margens).put(routes::catalogo::definir_margem),
        )
        .route(
            "/catalogo/margens-categoria/{categoria}",
            delete(routes::catalogo::remover_margem),
        )
        .route(
            "/catalogo/categorias",
            get(routes::catalogo::listar_categorias),
        )
        .route(
            "/catalogo/precificacao-produtos",
            get(routes::catalogo::listar_precificacao_produtos),
        )
        .route(
            "/catalogo/produtos/{id}/precificacao",
            put(routes::catalogo::definir_precificacao_produto),
        )
        .route(
            "/catalogo/giro-produtos",
            get(routes::catalogo::listar_giro_produtos),
        )
        .route(
            "/catalogo/mix-pagamento",
            get(routes::catalogo::mix_pagamento),
        )
        .route(
            "/catalogo/maquinas-cartao",
            get(routes::catalogo::listar_maquinas).put(routes::catalogo::definir_maquina),
        )
        .route(
            "/catalogo/maquinas-cartao/{nome}",
            delete(routes::catalogo::remover_maquina),
        )
        .route(
            "/catalogo/fretes-fornecedor",
            get(routes::catalogo::listar_fretes_fornecedor),
        )
        .route(
            "/catalogo/fornecedores/{id}/frete",
            put(routes::catalogo::definir_frete_fornecedor),
        )
        .route(
            "/catalogo/produtos/{id}/precos-concorrencia",
            get(routes::catalogo::listar_precos_concorrencia)
                .post(routes::catalogo::registrar_preco_concorrencia),
        )
        .route(
            "/catalogo/produtos/{id}/precos-concorrencia/{preco_id}",
            delete(routes::catalogo::remover_preco_concorrencia),
        )
        .route(
            "/catalogo/produtos/{id}/elasticidade",
            get(routes::catalogo::obter_elasticidade),
        )
}

fn crm_routes() -> ApiRouter {
    Router::new()
        .route("/crm/clientes", get(routes::crm::listar))
        .route("/crm/clientes", post(routes::crm::cadastrar))
        .route("/crm/clientes/{id}", get(routes::crm::buscar))
        .route("/crm/clientes/{id}", put(routes::crm::atualizar))
        .route("/crm/clientes/{id}/bloquear", post(routes::crm::bloquear))
        .route(
            "/crm/clientes/{id}/desbloquear",
            post(routes::crm::desbloquear),
        )
        .route("/crm/clientes/{id}/desativar", post(routes::crm::desativar))
        .route("/crm/clientes/{id}/reativar", post(routes::crm::reativar))
}

fn estoque_routes() -> ApiRouter {
    Router::new()
        .route("/estoque", get(routes::estoque::listar))
        .route("/estoque/{produto_id}", get(routes::estoque::buscar))
        .route("/estoque/entradas", post(routes::estoque::entrada))
        .route(
            "/estoque/{produto_id}/ajuste",
            post(routes::estoque::ajustar),
        )
        .route(
            "/estoque/{produto_id}/minimo",
            put(routes::estoque::definir_minimo),
        )
}

fn vendas_routes() -> ApiRouter {
    Router::new()
        .route("/vendas", get(routes::vendas::listar))
        .route("/vendas", post(routes::vendas::iniciar))
        // Lixeira (rotas estáticas antes de /vendas/{id})
        .route("/vendas/lixeira", get(routes::vendas::listar_lixeira))
        .route("/vendas/{id}/restaurar", post(routes::vendas::restaurar))
        .route("/vendas/{id}", get(routes::vendas::buscar))
        .route("/vendas/{id}", put(routes::vendas::atualizar))
        .route("/vendas/{id}/itens", post(routes::vendas::adicionar_item))
        .route(
            "/vendas/{id}/itens/{item_id}",
            delete(routes::vendas::remover_item),
        )
        .route(
            "/vendas/{id}/forma-pagamento",
            post(routes::vendas::forma_pagamento),
        )
        .route("/vendas/{id}/confirmar", post(routes::vendas::confirmar))
        .route("/vendas/{id}/cancelar", post(routes::vendas::cancelar))
        .route("/vendas/{id}/devolver", post(routes::vendas::devolver))
}

fn fornecedores_routes() -> ApiRouter {
    Router::new()
        .route("/fornecedores", get(routes::fornecedores::listar))
        .route("/fornecedores", post(routes::fornecedores::cadastrar))
        .route("/fornecedores/{id}", get(routes::fornecedores::buscar))
        .route("/fornecedores/{id}", put(routes::fornecedores::atualizar))
        .route(
            "/fornecedores/{id}/desativar",
            post(routes::fornecedores::desativar),
        )
        .route(
            "/fornecedores/{id}/reativar",
            post(routes::fornecedores::reativar),
        )
}

fn orcamentos_routes() -> ApiRouter {
    Router::new()
        .route("/orcamentos", get(routes::orcamentos::listar))
        .route("/orcamentos", post(routes::orcamentos::criar))
        // Lixeira (rotas estáticas antes de /orcamentos/{id})
        .route("/orcamentos/lixeira", get(routes::orcamentos::listar_lixeira))
        .route(
            "/orcamentos/{id}/restaurar",
            post(routes::orcamentos::restaurar),
        )
        .route("/orcamentos/{id}", get(routes::orcamentos::buscar))
        .route("/orcamentos/{id}", put(routes::orcamentos::atualizar))
        .route(
            "/orcamentos/{id}/itens",
            post(routes::orcamentos::adicionar_item),
        )
        .route(
            "/orcamentos/{id}/itens/{item_id}",
            delete(routes::orcamentos::remover_item),
        )
        .route(
            "/orcamentos/{id}/desconto",
            post(routes::orcamentos::aplicar_desconto),
        )
        .route("/orcamentos/{id}/emitir", post(routes::orcamentos::emitir))
        .route(
            "/orcamentos/{id}/aceitar",
            post(routes::orcamentos::aceitar),
        )
        .route(
            "/orcamentos/{id}/recusar",
            post(routes::orcamentos::recusar),
        )
        .route(
            "/orcamentos/{id}/cancelar",
            post(routes::orcamentos::cancelar),
        )
}

fn compras_routes() -> ApiRouter {
    Router::new()
        .route("/compras/pedidos", get(routes::compras::listar))
        .route("/compras/pedidos", post(routes::compras::gerar))
        .route("/compras/pedidos/{id}", get(routes::compras::buscar))
        .route(
            "/compras/pedidos/{id}/aprovar",
            post(routes::compras::aprovar),
        )
        .route(
            "/compras/pedidos/{id}/enviar",
            post(routes::compras::enviar),
        )
        .route(
            "/compras/pedidos/{id}/receber",
            post(routes::compras::receber),
        )
        .route(
            "/compras/pedidos/{id}/cancelar",
            post(routes::compras::cancelar),
        )
}

fn financeiro_routes() -> ApiRouter {
    Router::new()
        .route(
            "/financeiro/contas-receber",
            get(routes::financeiro::listar_contas_receber),
        )
        .route(
            "/financeiro/contas-receber/{id}",
            get(routes::financeiro::buscar_conta_receber),
        )
        .route(
            "/financeiro/contas-receber/{id}/pagamento",
            post(routes::financeiro::receber_pagamento),
        )
        .route(
            "/financeiro/contas-receber/{id}/estornar",
            post(routes::financeiro::estornar_conta_receber),
        )
        .route(
            "/financeiro/contas-pagar",
            get(routes::financeiro::listar_contas_pagar),
        )
        .route(
            "/financeiro/contas-pagar/{id}",
            get(routes::financeiro::buscar_conta_pagar),
        )
        .route(
            "/financeiro/contas-pagar/{id}/pagamento",
            post(routes::financeiro::efetuar_pagamento),
        )
}

fn fiscal_routes() -> ApiRouter {
    Router::new()
        .route("/fiscal/notas", get(routes::fiscal::listar))
        .route("/fiscal/notas/{id}", get(routes::fiscal::buscar))
        .route(
            "/fiscal/notas/{id}/cancelar",
            post(routes::fiscal::cancelar),
        )
        .route(
            "/fiscal/notas/{id}/retransmitir",
            post(routes::fiscal::retransmitir),
        )
}

fn backoffice_routes() -> ApiRouter {
    Router::new()
        // Tenants
        .route(
            "/backoffice/tenants",
            get(routes::backoffice::listar_tenants),
        )
        .route(
            "/backoffice/tenants",
            post(routes::backoffice::criar_tenant),
        )
        .route(
            "/backoffice/tenants/{id}",
            put(routes::backoffice::atualizar_tenant),
        )
        .route(
            "/backoffice/tenants/{id}/suspender",
            post(routes::backoffice::suspender_tenant),
        )
        .route(
            "/backoffice/tenants/{id}/reativar",
            post(routes::backoffice::reativar_tenant),
        )
        .route(
            "/backoffice/tenants/{id}/impersonar",
            post(routes::backoffice::impersonar_tenant),
        )
        .route(
            "/backoffice/tenants/{id}/plano",
            post(routes::backoffice::alterar_plano),
        )
        // Admins (superadmin only)
        .route(
            "/backoffice/revenue",
            get(routes::backoffice::revenue_overview),
        )
        .route("/backoffice/admins", get(routes::backoffice::listar_admins))
        .route("/backoffice/admins", post(routes::backoffice::criar_admin))
        .route(
            "/backoffice/admins/{id}/desativar",
            post(routes::backoffice::desativar_admin),
        )
        .route(
            "/backoffice/admins/{id}/reativar",
            post(routes::backoffice::reativar_admin),
        )
        .route(
            "/backoffice/admins/{id}/permissoes",
            post(routes::backoffice::alterar_permissoes),
        )
        .route(
            "/backoffice/admins/{id}/password",
            post(routes::backoffice::change_admin_password),
        )
}
