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
use crate::fiscal::application::commands::{CancelarNotaFiscal, RetransmitirNotaFiscal};
use crate::fiscal::application::queries::{BuscarNotaFiscal, ListarNotasFiscais};
use crate::web::{error::ApiError, state::FiscalState};

pub async fn listar(
    State(s): State<FiscalState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Fiscal, Role::Financeiro])?;
    let notas = query_dispatch(&*s.fiscal, ListarNotasFiscais).await?;
    Ok(Json(json!({ "notas": notas })))
}

pub async fn buscar(
    State(s): State<FiscalState>,
    user: AuthUser,
    Path(nf_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Fiscal, Role::Financeiro])?;
    let nota = query_dispatch(&*s.fiscal, BuscarNotaFiscal { nf_id }).await?;
    nota.map(|n| Json(json!(n)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn cancelar(
    State(s): State<FiscalState>,
    user: AuthUser,
    Path(nf_id): Path<Uuid>,
    Json(mut cmd): Json<CancelarNotaFiscal>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Fiscal])?;
    cmd.nf_id = nf_id;
    dispatch(&*s.fiscal, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn retransmitir(
    State(s): State<FiscalState>,
    user: AuthUser,
    Path(nf_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Fiscal])?;
    dispatch(&*s.fiscal, RetransmitirNotaFiscal { nf_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}
