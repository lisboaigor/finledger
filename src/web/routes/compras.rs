use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Role};
use crate::compras::application::commands::{
    AprovarPedidoCompra, CancelarPedidoCompra, EnviarPedidoCompra, GerarPedidoCompra,
    ReceberMercadoria,
};
use crate::compras::application::queries::{BuscarPedidoCompra, ListarPedidosCompra};
use crate::error::AppError;
use crate::web::{error::ApiError, state::ComprasState};

pub async fn listar(
    State(s): State<ComprasState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    let pedidos = query_dispatch(&*s.compras, ListarPedidosCompra).await?;
    Ok(Json(json!({ "pedidos": pedidos })))
}

pub async fn buscar(
    State(s): State<ComprasState>,
    user: AuthUser,
    Path(pedido_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    let detalhes = query_dispatch(&*s.compras, BuscarPedidoCompra { pedido_id }).await?;
    detalhes
        .map(|d| Json(json!(d)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn gerar(
    State(s): State<ComprasState>,
    user: AuthUser,
    Json(mut cmd): Json<GerarPedidoCompra>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    cmd.comprador_id = user.id;
    let id = dispatch(&*s.compras, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "pedido_id": id.to_string() })),
    ))
}

pub async fn aprovar(
    State(s): State<ComprasState>,
    user: AuthUser,
    Path(pedido_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    dispatch(
        &*s.compras,
        AprovarPedidoCompra {
            pedido_id,
            aprovador_id: user.id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn enviar(
    State(s): State<ComprasState>,
    user: AuthUser,
    Path(pedido_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    dispatch(&*s.compras, EnviarPedidoCompra { pedido_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn receber(
    State(s): State<ComprasState>,
    user: AuthUser,
    Path(pedido_id): Path<Uuid>,
    Json(mut cmd): Json<ReceberMercadoria>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador, Role::Estoquista])?;
    cmd.pedido_id = pedido_id;
    dispatch(&*s.compras, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancelar(
    State(s): State<ComprasState>,
    user: AuthUser,
    Path(pedido_id): Path<Uuid>,
    Json(mut cmd): Json<CancelarPedidoCompra>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Comprador])?;
    cmd.pedido_id = pedido_id;
    dispatch(&*s.compras, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
