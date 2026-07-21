use axum::{
    Json,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use pharos_app::{CURRENT_TENANT, TenantContext};
use serde_json::json;

use crate::{
    auth::jwt::{decode_backoffice_token, decode_token},
    web::state::AuthMiddlewareState,
};

pub async fn require_auth(
    State(state): State<AuthMiddlewareState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let Some(token) = token else {
        return unauthorized();
    };

    let Ok(claims) = decode_token(token, &state.auth.secret) else {
        return unauthorized();
    };

    // O tenant é uma claim assinada — autoridade do isolamento. Sem ela, nega.
    if claims.tenant_id.trim().is_empty() {
        return unauthorized();
    }

    // Defesa adicional: quando habilitado, o subdomínio do host deve casar com o tenant
    // do token (evita usar um token de um tenant em outro subdomínio).
    if subdomain_enforced()
        && let Some(sub) = request_subdomain(&req)
        && sub != claims.tenant_slug
    {
        return forbidden("subdomínio não corresponde ao tenant do token");
    }

    let Ok(tenant) = TenantContext::parse(&claims.tenant_id) else {
        return unauthorized();
    };

    // A suspended tenant loses access immediately — otherwise tokens issued
    // before the suspension would keep working until they expire.
    let Ok(tenant_uuid) = uuid::Uuid::parse_str(&claims.tenant_id) else {
        return unauthorized();
    };
    match state.tenants.buscar_por_id(tenant_uuid).await {
        Ok(Some(row)) if row.status == "ativo" => {}
        Ok(_) => return forbidden("tenant suspenso ou inexistente"),
        Err(e) => {
            tracing::error!(error = %e, "failed to check tenant status");
            return internal_error();
        }
    }

    req.extensions_mut().insert(claims);

    // Seta o tenant no task-local por toda a duração do request (mesmo task assíncrono).
    CURRENT_TENANT.scope(Some(tenant), next.run(req)).await
}

/// `true` quando `TENANT_SUBDOMAIN_ENFORCED=1|true` — desligado por padrão (dev/localhost).
fn subdomain_enforced() -> bool {
    matches!(
        std::env::var("TENANT_SUBDOMAIN_ENFORCED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Extrai o primeiro rótulo do host (`acme` em `acme.finledger.app`). Retorna `None` para
/// hosts sem subdomínio real (localhost, IP, domínio apex).
fn request_subdomain(req: &Request) -> Option<String> {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())?;

    let host = host.split(':').next().unwrap_or(host);

    if host == "localhost" || host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }

    let labels: Vec<&str> = host.split('.').collect();

    // Precisa de pelo menos sub.dominio.tld para haver um subdomínio.
    if labels.len() < 3 {
        return None;
    }

    Some(labels[0].to_string())
}

/// Middleware para rotas de backoffice (superadmin + admins de suporte).
/// Valida JWT com claims de backoffice e insere nas extensions.
pub async fn require_backoffice_auth(
    State(state): State<AuthMiddlewareState>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let Some(token) = token else {
        return unauthorized();
    };

    let Ok(claims) = decode_backoffice_token(token, &state.auth.secret) else {
        return unauthorized();
    };

    // A deactivated admin loses access immediately — mirrors the tenant
    // suspension check in `require_auth` below. Without this, a long-lived
    // (8h) backoffice token keeps working after the account is disabled.
    let Ok(user_id) = uuid::Uuid::parse_str(&claims.sub) else {
        return unauthorized();
    };
    match state.backoffice.repo().esta_ativo(user_id).await {
        Ok(Some(true)) => {}
        Ok(_) => return forbidden("admin inativo ou inexistente"),
        Err(e) => {
            tracing::error!(error = %e, "failed to check backoffice admin status");
            return internal_error();
        }
    }

    req.extensions_mut().insert(claims);
    next.run(req).await
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "token ausente ou inválido" })),
    )
        .into_response()
}

fn forbidden(msg: &str) -> Response {
    (StatusCode::FORBIDDEN, Json(json!({ "error": msg }))).into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "erro interno" })),
    )
        .into_response()
}
