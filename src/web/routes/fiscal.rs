use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::error::AppError;
use crate::fiscal::application::commands::{CancelarNotaFiscal, RetransmitirNotaFiscal};
use crate::fiscal::application::queries::{
    BuscarNotaFiscal, ListarAliquotaEfetivaProdutos, ListarClassesTributarias, ListarNotasFiscais,
};
use crate::web::routes::PaginacaoParams;
use crate::web::{error::ApiError, state::FiscalState};

pub async fn listar(
    State(s): State<FiscalState>,
    user: AuthUser,
    Query(p): Query<PaginacaoParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Fiscal, Role::Financeiro])?;
    let notas = query_dispatch(
        &*s.fiscal,
        ListarNotasFiscais {
            limite: p.limite,
            offset: p.offset,
        },
    )
    .await?;
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

/// Alíquota efetiva de imposto (bps) por produto, na fase vigente hoje e no
/// perfil do tenant — insumo da precificação assistida (substitui o imposto
/// manual único). Qualquer usuário autenticado, como o giro de produtos.
pub async fn aliquotas_efetivas(
    State(s): State<FiscalState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let aliquotas = query_dispatch(&*s.fiscal, ListarAliquotaEfetivaProdutos).await?;
    Ok(Json(json!({ "aliquotas": aliquotas })))
}

/// Classes tributárias de referência (dado global) — qualquer usuário
/// autenticado: o select do catálogo é preenchido por quem cadastra produto.
pub async fn listar_classes_tributarias(
    State(s): State<FiscalState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let classes = query_dispatch(&*s.fiscal, ListarClassesTributarias).await?;
    Ok(Json(json!({ "classes": classes })))
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
