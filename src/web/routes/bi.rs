use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::bi::application::commands::RegistrarFeedbackAlerta;
use crate::bi::application::queries::{
    ListarAlertasBi, ObterComercialBi, ObterEstoqueBi, ObterFinanceiroBi, ObterResumoBi,
};
use crate::web::{error::ApiError, state::BiState};

/// KPIs do dia + série de receita dos últimos 30 dias (dashboard "Hoje").
pub async fn resumo(
    State(s): State<BiState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let resumo = query_dispatch(&*s.bi, ObterResumoBi).await?;
    Ok(Json(json!(resumo)))
}

/// CCC, aging de recebíveis, projeção semanal de fluxo e top devedores.
pub async fn financeiro(
    State(s): State<BiState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dados = query_dispatch(&*s.bi, ObterFinanceiroBi).await?;
    Ok(Json(json!(dados)))
}

/// Funil de orçamentos, expirando, desempenho por vendedor e RFM.
pub async fn comercial(
    State(s): State<BiState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dados = query_dispatch(&*s.bi, ObterComercialBi).await?;
    Ok(Json(json!(dados)))
}

/// Matriz ABC×XYZ, rupturas, estoque morto, giro por categoria e pedidos parados.
pub async fn estoque(
    State(s): State<BiState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dados = query_dispatch(&*s.bi, ObterEstoqueBi).await?;
    Ok(Json(json!(dados)))
}

#[derive(Deserialize)]
pub struct AlertasParams {
    limite: Option<i64>,
}

pub async fn alertas(
    State(s): State<BiState>,
    _user: AuthUser,
    Query(params): Query<AlertasParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let alertas = query_dispatch(
        &*s.bi,
        ListarAlertasBi {
            limite: params.limite.unwrap_or(5),
        },
    )
    .await?;
    Ok(Json(json!({ "alertas": alertas })))
}

pub async fn feedback(
    State(s): State<BiState>,
    _user: AuthUser,
    Path(alerta_id): Path<Uuid>,
    Json(mut cmd): Json<RegistrarFeedbackAlerta>,
) -> Result<StatusCode, ApiError> {
    cmd.alerta_id = alerta_id;
    dispatch(&*s.bi, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
