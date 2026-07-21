use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::BackofficeUser;
use crate::backoffice::domain::{BackofficePermission, TenantPlan};
use crate::backoffice::handlers::{
    AlterarPermissoesCmd, ChangeAdminPasswordCmd, CriarAdminCmd, LoginBackofficeCmd,
    ProvisionarTenantCmd,
};
use crate::web::error::ApiError;
use crate::web::state::BackofficeState;

// ── Auth ──────────────────────────────────────────────────────────────────────

pub async fn login(
    State(s): State<BackofficeState>,
    Json(cmd): Json<LoginBackofficeCmd>,
) -> Result<Json<Value>, ApiError> {
    let token = s.backoffice.login(cmd).await?;
    Ok(Json(json!({ "token": token })))
}

// ── Tenants ───────────────────────────────────────────────────────────────────

pub async fn listar_tenants(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
) -> Result<Json<Value>, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsRead)?;
    let tenants = s.tenants.listar().await?;
    Ok(Json(json!({ "tenants": tenants })))
}

pub async fn criar_tenant(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Json(payload): Json<ProvisionarTenantCmd>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsWrite)?;

    let slug = payload.slug.clone();
    let provisionado = s.backoffice.provisionar_tenant(payload).await?;

    tracing::info!(%slug, tenant_id = %provisionado.tenant_id, operador = %user.username, "tenant provisionado via backoffice");

    Ok((
        StatusCode::CREATED,
        Json(json!({ "tenant_id": provisionado.tenant_id, "slug": provisionado.slug })),
    ))
}

#[derive(Deserialize)]
pub struct AtualizarTenantPayload {
    pub nome: String,
}

pub async fn atualizar_tenant(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(tenant_id): Path<Uuid>,
    Json(payload): Json<AtualizarTenantPayload>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsWrite)?;
    s.backoffice
        .atualizar_tenant(tenant_id, payload.nome)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn suspender_tenant(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsWrite)?;
    s.tenants.suspender(tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar_tenant(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsWrite)?;
    s.tenants.reativar(tenant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn impersonar_tenant(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsImpersonate)?;
    let token = s
        .backoffice
        .impersonar_tenant(tenant_id, &user.username)
        .await?;
    Ok(Json(json!({ "token": token, "expira_em_horas": 1 })))
}

#[derive(Deserialize)]
pub struct AlterarPlanoPayload {
    pub plano: TenantPlan,
}

pub async fn alterar_plano(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(tenant_id): Path<Uuid>,
    Json(payload): Json<AlterarPlanoPayload>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsWrite)?;
    s.backoffice.alterar_plano(tenant_id, payload.plano).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Cross-tenant revenue overview for the backoffice dashboard.
pub async fn revenue_overview(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
) -> Result<Json<Value>, ApiError> {
    user.exigir_permissao(BackofficePermission::TenantsRead)?;
    let overview = s.backoffice.revenue_overview(12, 30).await?;
    Ok(Json(json!(overview)))
}

// ── Admins (requires the admins:manage permission; superadmin always has it) ──

pub async fn listar_admins(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
) -> Result<Json<Value>, ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    let admins = s.backoffice.listar_admins().await?;
    Ok(Json(json!({ "admins": admins })))
}

pub async fn criar_admin(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Json(cmd): Json<CriarAdminCmd>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    let id = s.backoffice.criar_admin(cmd).await?;
    Ok((StatusCode::CREATED, Json(json!({ "user_id": id }))))
}

pub async fn desativar_admin(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(admin_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    s.backoffice.desativar_admin(admin_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar_admin(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(admin_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    s.backoffice.reativar_admin(admin_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn alterar_permissoes(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(admin_id): Path<Uuid>,
    Json(cmd): Json<AlterarPermissoesCmd>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    s.backoffice.alterar_permissoes(admin_id, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_admin_password(
    State(s): State<BackofficeState>,
    user: BackofficeUser,
    Path(admin_id): Path<Uuid>,
    Json(cmd): Json<ChangeAdminPasswordCmd>,
) -> Result<StatusCode, ApiError> {
    user.exigir_permissao(BackofficePermission::AdminsManage)?;
    s.backoffice.change_admin_password(admin_id, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
