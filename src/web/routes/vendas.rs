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
use crate::vendas::application::commands::{
    AdicionarItemVenda, AtualizarVenda, CancelarVenda, ConfirmarVenda, DefinirFormaPagamento,
    DevolverItensVenda, IniciarVenda, RemoverItemVenda,
};
use crate::auth::Roles;
use crate::vendas::application::queries::{BuscarVenda, ListarVendas, ListarVendasArquivadas};
use crate::web::{error::ApiError, state::VendasState};

#[derive(Deserialize)]
pub struct ListarVendasParams {
    #[serde(default)]
    produto: Option<String>,
    /// `?abertas=true` → só vendas EmAndamento (recuperação no PDV).
    #[serde(default)]
    abertas: bool,
}

pub async fn listar(
    State(s): State<VendasState>,
    user: AuthUser,
    Query(params): Query<ListarVendasParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor, Role::Financeiro])?;
    let vendas = query_dispatch(
        &*s.vendas,
        ListarVendas {
            produto_busca: params.produto,
            apenas_abertas: params.abertas,
        },
    )
    .await?;
    Ok(Json(json!({ "vendas": vendas })))
}

/// Lixeira de vendas — só o gestor vê e restaura.
pub async fn listar_lixeira(
    State(s): State<VendasState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let vendas = query_dispatch(&*s.vendas, ListarVendasArquivadas).await?;
    Ok(Json(json!({ "vendas": vendas })))
}

pub async fn restaurar(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.vendas.restaurar_arquivada(venda_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn buscar(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor, Role::Financeiro])?;
    let detalhes = query_dispatch(&*s.vendas, BuscarVenda { venda_id }).await?;
    detalhes
        .map(|d| Json(json!(d)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn iniciar(
    State(s): State<VendasState>,
    user: AuthUser,
    Json(mut cmd): Json<IniciarVenda>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.vendedor_id = user.id;
    let id = dispatch(&*s.vendas, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "venda_id": id.to_string() })),
    ))
}

pub async fn atualizar(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarVenda>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.venda_id = venda_id;
    dispatch(&*s.vendas, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn adicionar_item(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
    Json(mut cmd): Json<AdicionarItemVenda>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.venda_id = venda_id;
    let item_id = dispatch(&*s.vendas, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "item_id": item_id.to_string() })),
    ))
}

pub async fn remover_item(
    State(s): State<VendasState>,
    user: AuthUser,
    Path((venda_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    dispatch(&*s.vendas, RemoverItemVenda { venda_id, item_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn forma_pagamento(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
    Json(mut cmd): Json<DefinirFormaPagamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.venda_id = venda_id;
    dispatch(&*s.vendas, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn confirmar(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    dispatch(&*s.vendas, ConfirmarVenda { venda_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn devolver(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
    Json(mut cmd): Json<DevolverItensVenda>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor, Role::Financeiro])?;
    cmd.venda_id = venda_id;
    dispatch(&*s.vendas, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancelar(
    State(s): State<VendasState>,
    user: AuthUser,
    Path(venda_id): Path<Uuid>,
    Json(mut cmd): Json<CancelarVenda>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.venda_id = venda_id;
    dispatch(&*s.vendas, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
