use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::tenants::repository::Marca;
use crate::web::{error::ApiError, state::TenantLookupState};

/// Verificação pública usada pelo composer da landing page: 204 quando o slug
/// corresponde a um tenant ativo, 404 caso contrário. Expõe apenas a existência
/// do slug (mesma informação que a própria página de login do subdomínio) e
/// fica atrás do rate limit estrito das rotas públicas.
pub async fn existe(State(s): State<TenantLookupState>, Path(slug): Path<String>) -> StatusCode {
    let slug = slug.trim().to_ascii_lowercase();
    if slug.is_empty() {
        return StatusCode::NOT_FOUND;
    }
    match s.tenants.buscar_por_slug(&slug).await {
        Ok(Some(t)) if t.status == "ativo" => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(e) => {
            tracing::error!(error = ?e, slug, "falha ao verificar tenant público");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Marca whitelabel do tenant do subdomínio, sem autenticação — usada para
/// brandizar a tela de login (logo, nome e cores) antes de o usuário entrar.
/// Tenant inexistente/suspenso devolve a marca padrão (objeto vazio), sem
/// distinguir de um tenant sem personalização.
pub async fn marca(
    State(s): State<TenantLookupState>,
    Path(slug): Path<String>,
) -> Result<Json<Marca>, ApiError> {
    let slug = slug.trim().to_ascii_lowercase();
    let marca = s.tenants.obter_marca_por_slug(&slug).await?.unwrap_or_default();
    Ok(Json(marca))
}
