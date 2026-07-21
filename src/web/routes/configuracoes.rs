use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{AuthUser, Roles};
use crate::tenants::repository::{ConfigPrecificacao, DadosEmpresa, Marca};
use crate::web::{error::ApiError, state::TenantLookupState};

#[derive(Serialize)]
pub struct ConfiguracoesResponse {
    permite_orcamento_sem_estoque: bool,
    /// Dias até vendas/orçamentos não concretizados irem para a lixeira
    /// (None = arquivamento automático desligado).
    arquivamento_dias: Option<i32>,
    #[serde(flatten)]
    dados_empresa: DadosEmpresa,
    #[serde(flatten)]
    precificacao: ConfigPrecificacao,
}

/// Configurações do tenant atual (self-service — qualquer usuário autenticado
/// pode consultar; só admin pode alterar, ver `atualizar`).
pub async fn obter(
    State(s): State<TenantLookupState>,
    _user: AuthUser,
) -> Result<Json<ConfiguracoesResponse>, ApiError> {
    let permite_orcamento_sem_estoque = s.tenants.permite_orcamento_sem_estoque().await?;
    let arquivamento_dias = s.tenants.obter_arquivamento_dias().await?;
    let dados_empresa = s.tenants.obter_dados_empresa().await?;
    let precificacao = s.tenants.obter_config_precificacao().await?;
    Ok(Json(ConfiguracoesResponse {
        permite_orcamento_sem_estoque,
        arquivamento_dias,
        dados_empresa,
        precificacao,
    }))
}

#[derive(Deserialize)]
pub struct AtualizarConfiguracoes {
    pub permite_orcamento_sem_estoque: bool,
    #[serde(default)]
    pub arquivamento_dias: Option<i32>,
    #[serde(flatten)]
    pub dados_empresa: DadosEmpresa,
    #[serde(flatten)]
    pub precificacao: ConfigPrecificacao,
}

pub async fn atualizar(
    State(s): State<TenantLookupState>,
    user: AuthUser,
    Json(body): Json<AtualizarConfiguracoes>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.tenants
        .atualizar_configuracoes(body.permite_orcamento_sem_estoque)
        .await?;
    s.tenants
        .atualizar_arquivamento_dias(body.arquivamento_dias)
        .await?;
    s.tenants.atualizar_dados_empresa(body.dados_empresa).await?;
    s.tenants
        .atualizar_config_precificacao(body.precificacao)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Marca whitelabel ─────────────────────────────────────────────────────────
// Identidade visual self-service: nome, logo (data URI), cor de destaque, fonte
// do nome e seu tamanho/cor. Leitura pública por slug (ver
// `tenants_publico::marca`, usada antes do login); escrita só admin do tenant.

fn validar_hex(campo: &str, valor: &Option<String>) -> Result<(), ApiError> {
    if let Some(v) = valor {
        let ok = v.len() == 7
            && v.starts_with('#')
            && v[1..].bytes().all(|b| b.is_ascii_hexdigit());
        if !ok {
            return Err(pharos_core::DomainError::Validation(format!(
                "{campo} deve ser uma cor hexadecimal no formato #RRGGBB"
            ))
            .into());
        }
    }
    Ok(())
}

pub async fn atualizar_marca(
    State(s): State<TenantLookupState>,
    user: AuthUser,
    Json(mut marca): Json<Marca>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;

    // Normaliza o nome: espaços aparados, vazio vira ausência (cai no padrão).
    marca.marca_nome = marca
        .marca_nome
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    if marca.marca_nome.as_ref().is_some_and(|n| n.chars().count() > 40) {
        return Err(pharos_core::DomainError::Validation(
            "O nome da marca deve ter no máximo 40 caracteres".into(),
        )
        .into());
    }

    validar_hex("A cor de destaque", &marca.marca_cor_primaria)?;
    validar_hex("A cor do nome", &marca.marca_fonte_cor)?;

    // Tamanho do wordmark: percentual entre 50% e 200% (nulo = padrão 100%).
    if let Some(t) = marca.marca_fonte_tamanho
        && !(50..=200).contains(&t)
    {
        return Err(pharos_core::DomainError::Validation(
            "O tamanho do nome deve ficar entre 50% e 200%".into(),
        )
        .into());
    }

    // Fonte: chave curta com charset restrito (o frontend mapeia para a
    // font-family; chave desconhecida cai no padrão sem quebrar).
    if let Some(f) = &marca.marca_fonte {
        let ok = !f.is_empty()
            && f.len() <= 32
            && f.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !ok {
            return Err(pharos_core::DomainError::Validation(
                "Fonte inválida".into(),
            )
            .into());
        }
    }

    if let Some(logo) = &marca.marca_logo_data_uri {
        if !logo.starts_with("data:image/") {
            return Err(pharos_core::DomainError::Validation(
                "O logo deve ser uma imagem".into(),
            )
            .into());
        }
        // ~700 KB de texto ≈ 512 KB de imagem; deixa folga no corpo de 1 MB.
        if logo.len() > 700_000 {
            return Err(pharos_core::DomainError::Validation(
                "A imagem do logo é muito grande — use uma menor".into(),
            )
            .into());
        }
    }

    s.tenants.atualizar_marca(marca).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Custos fixos discriminados ───────────────────────────────────────────────
// Itens nomeados (aluguel, salário, DAS…) cuja soma vira o total mensal usado
// na sugestão de preço. Leitura livre (o painel de precificação mostra a
// composição a qualquer usuário); escrita só admin, como o restante da tela.

pub async fn listar_custos_fixos(
    State(s): State<TenantLookupState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let custos = s.tenants.listar_custos_fixos().await?;
    Ok(Json(json!({ "custos": custos })))
}

#[derive(Deserialize)]
pub struct DefinirCustoFixoBody {
    pub nome: String,
    pub valor_centavos: i64,
}

pub async fn definir_custo_fixo(
    State(s): State<TenantLookupState>,
    user: AuthUser,
    Json(body): Json<DefinirCustoFixoBody>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let nome = body.nome.trim();
    if nome.is_empty() {
        return Err(pharos_core::DomainError::Validation("Informe o nome do custo fixo".into()).into());
    }
    if body.valor_centavos < 0 {
        return Err(pharos_core::DomainError::Validation(
            "O valor do custo fixo não pode ser negativo".into(),
        )
        .into());
    }
    s.tenants.definir_custo_fixo(nome, body.valor_centavos).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remover_custo_fixo(
    State(s): State<TenantLookupState>,
    user: AuthUser,
    Path(nome): Path<String>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.tenants.remover_custo_fixo(&nome).await?;
    Ok(StatusCode::NO_CONTENT)
}
