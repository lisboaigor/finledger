use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::web::state::TenantLookupState;

/// Hostnames de 1 nível sempre autorizados, independentemente de tenant.
const SUBDOMINIOS_RESERVADOS: &[&str] = &["www", "admin", "backoffice"];

#[derive(Deserialize)]
pub struct AskParams {
    domain: String,
}

/// Endpoint `ask` do TLS on-demand do Caddy (`on_demand_tls` no Caddyfile).
/// O Caddy chama `GET /tls/ask?domain=<hostname>` antes de emitir um certificado:
/// 200 autoriza a emissão, qualquer outro status nega. Autorizamos o apex, os
/// subdomínios reservados e `<slug>.<base>` quando o slug é um tenant ativo —
/// assim ninguém consome emissões do Let's Encrypt apontando nomes inválidos.
///
/// Só ativo quando `TLS_BASE_DOMAIN` está definido (ex.: `finledger.com.br`);
/// sem a env, responde 503 e o on-demand fica efetivamente desligado.
pub async fn ask(State(s): State<TenantLookupState>, Query(p): Query<AskParams>) -> StatusCode {
    let Ok(base) = std::env::var("TLS_BASE_DOMAIN") else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };
    let base = base.trim().trim_end_matches('.').to_ascii_lowercase();
    if base.is_empty() {
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    let dominio = p.domain.trim().trim_end_matches('.').to_ascii_lowercase();

    if dominio == base {
        return StatusCode::OK;
    }

    let Some(sub) = dominio.strip_suffix(&format!(".{base}")) else {
        return StatusCode::FORBIDDEN;
    };
    // Apenas um nível de subdomínio (a.b.finledger.com.br não é tenant válido).
    if sub.is_empty() || sub.contains('.') {
        return StatusCode::FORBIDDEN;
    }

    if SUBDOMINIOS_RESERVADOS.contains(&sub) {
        return StatusCode::OK;
    }

    match s.tenants.buscar_por_slug(sub).await {
        Ok(Some(t)) if t.status == "ativo" => StatusCode::OK,
        Ok(_) => StatusCode::FORBIDDEN,
        Err(e) => {
            tracing::error!(error = ?e, dominio, "falha ao consultar tenant no /tls/ask");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
