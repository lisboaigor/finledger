use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::error::AppError;
use crate::estoque::application::commands::{
    AjustarEstoque, DefinirEstoqueMinimo, RegistrarEntradaEstoque,
};
use crate::estoque::application::queries::{BuscarSaldo, ListarSaldos};
use crate::web::{error::ApiError, state::EstoqueState};

pub async fn listar(
    State(s): State<EstoqueState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Estoquista, Role::Comprador, Role::Vendedor])?;
    let saldos = query_dispatch(&*s.estoque, ListarSaldos).await?;
    Ok(Json(json!({ "saldos": saldos })))
}

pub async fn buscar(
    State(s): State<EstoqueState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Estoquista, Role::Comprador, Role::Vendedor])?;
    let saldo = query_dispatch(&*s.estoque, BuscarSaldo { produto_id }).await?;
    saldo
        .map(|s| Json(json!(s)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn entrada(
    State(s): State<EstoqueState>,
    user: AuthUser,
    Json(cmd): Json<RegistrarEntradaEstoque>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Estoquista])?;
    dispatch(&*s.estoque, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn ajustar(
    State(s): State<EstoqueState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(mut cmd): Json<AjustarEstoque>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Estoquista])?;
    cmd.produto_id = produto_id;
    dispatch(&*s.estoque, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn definir_minimo(
    State(s): State<EstoqueState>,
    user: AuthUser,
    Path(produto_id): Path<Uuid>,
    Json(mut cmd): Json<DefinirEstoqueMinimo>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Estoquista])?;
    cmd.produto_id = produto_id;
    dispatch(&*s.estoque, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
