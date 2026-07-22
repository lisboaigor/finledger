use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::error::AppError;
use crate::financeiro::application::commands::{
    EfetuarPagamento, EstornarContaReceber, RegistrarAbatimentoContaReceber,
    RegistrarPagamentoRecebido,
};
use crate::financeiro::application::queries::{
    BuscarContaPagar, BuscarContaReceber, ListarContasPagar, ListarContasReceber,
};
use crate::web::{error::ApiError, state::FinanceiroState};

/// Paginação opcional das listagens (aditivo — sem os params o comportamento
/// é o histórico). Clamp de limites na camada de repositório.
#[derive(Deserialize)]
pub struct PaginacaoParams {
    limite: Option<i64>,
    offset: Option<i64>,
}

pub async fn listar_contas_receber(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Query(p): Query<PaginacaoParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    let contas = query_dispatch(
        &*s.financeiro,
        ListarContasReceber {
            limite: p.limite,
            offset: p.offset,
        },
    )
    .await?;
    Ok(Json(json!({ "contas": contas })))
}

pub async fn buscar_conta_receber(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    let conta = query_dispatch(&*s.financeiro, BuscarContaReceber { conta_id }).await?;
    conta
        .map(|c| Json(json!(c)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn listar_contas_pagar(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Query(p): Query<PaginacaoParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    let contas = query_dispatch(
        &*s.financeiro,
        ListarContasPagar {
            limite: p.limite,
            offset: p.offset,
        },
    )
    .await?;
    Ok(Json(json!({ "contas": contas })))
}

pub async fn buscar_conta_pagar(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    let conta = query_dispatch(&*s.financeiro, BuscarContaPagar { conta_id }).await?;
    conta
        .map(|c| Json(json!(c)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn receber_pagamento(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
    Json(mut cmd): Json<RegistrarPagamentoRecebido>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    cmd.conta_id = conta_id;
    dispatch(&*s.financeiro, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn registrar_abatimento(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
    Json(mut cmd): Json<RegistrarAbatimentoContaReceber>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    cmd.conta_id = conta_id;
    dispatch(&*s.financeiro, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn estornar_conta_receber(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
    Json(mut cmd): Json<EstornarContaReceber>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    cmd.conta_id = conta_id;
    dispatch(&*s.financeiro, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn efetuar_pagamento(
    State(s): State<FinanceiroState>,
    user: AuthUser,
    Path(conta_id): Path<Uuid>,
    Json(mut cmd): Json<EfetuarPagamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Financeiro])?;
    cmd.conta_id = conta_id;
    dispatch(&*s.financeiro, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
