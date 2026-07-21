use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role, Roles};
use crate::error::AppError;
use crate::fornecedores::application::commands::{
    AtualizarFornecedor, CadastrarFornecedor, DesativarFornecedor, ReativarFornecedor,
};
use crate::fornecedores::application::queries::{BuscarFornecedor, ListarFornecedores};
use crate::web::{error::ApiError, state::FornecedoresState};

pub async fn listar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    let fornecedores = query_dispatch(&*s.fornecedores, ListarFornecedores).await?;
    Ok(Json(json!({ "fornecedores": fornecedores })))
}

pub async fn buscar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
    Path(fornecedor_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    let fornecedor = query_dispatch(&*s.fornecedores, BuscarFornecedor { fornecedor_id }).await?;
    fornecedor
        .map(|f| Json(json!(f)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn cadastrar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
    Json(cmd): Json<CadastrarFornecedor>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    let id = dispatch(&*s.fornecedores, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "fornecedor_id": id.to_string() })),
    ))
}

pub async fn atualizar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
    Path(fornecedor_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarFornecedor>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    cmd.fornecedor_id = fornecedor_id;
    dispatch(&*s.fornecedores, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desativar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
    Path(fornecedor_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.fornecedores, DesativarFornecedor { fornecedor_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar(
    State(s): State<FornecedoresState>,
    user: AuthUser,
    Path(fornecedor_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.fornecedores, ReativarFornecedor { fornecedor_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}
