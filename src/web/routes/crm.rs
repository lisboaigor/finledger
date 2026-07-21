use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role, Roles};
use crate::crm::application::commands::{
    AtualizarCliente, BloquearCliente, CadastrarCliente, DesativarCliente, DesbloquearCliente,
    ReativarCliente,
};
use crate::crm::application::queries::{BuscarCliente, ListarClientes};
use crate::error::AppError;
use crate::web::{error::ApiError, state::CrmState};

pub async fn listar(
    State(s): State<CrmState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    let clientes = query_dispatch(&*s.crm, ListarClientes).await?;
    Ok(Json(json!({ "clientes": clientes })))
}

pub async fn buscar(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    let cliente = query_dispatch(&*s.crm, BuscarCliente { cliente_id }).await?;
    cliente
        .map(|c| Json(json!(c)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn cadastrar(
    State(s): State<CrmState>,
    user: AuthUser,
    Json(cmd): Json<CadastrarCliente>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    let id = dispatch(&*s.crm, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "cliente_id": id.to_string() })),
    ))
}

pub async fn atualizar(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarCliente>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.cliente_id = cliente_id;
    dispatch(&*s.crm, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn bloquear(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
    Json(mut cmd): Json<BloquearCliente>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    cmd.cliente_id = cliente_id;
    dispatch(&*s.crm, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desbloquear(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.crm, DesbloquearCliente { cliente_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desativar(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.crm, DesativarCliente { cliente_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar(
    State(s): State<CrmState>,
    user: AuthUser,
    Path(cliente_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.crm, ReativarCliente { cliente_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}
