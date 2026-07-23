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
use crate::orcamentos::application::commands::{
    AceitarOrcamento, AdicionarItemOrcamento, AplicarDescontoOrcamento, AtualizarOrcamento,
    CancelarOrcamento, CriarOrcamento, EmitirOrcamento, RecusarOrcamento, RemoverItemOrcamento,
};
use crate::auth::Roles;
use crate::orcamentos::application::queries::{
    BuscarOrcamento, ListarOrcamentos, ListarOrcamentosArquivados,
};
use crate::web::routes::PaginacaoParams;
use crate::web::{error::ApiError, state::OrcamentosState};

#[derive(Deserialize)]
pub struct ListarOrcamentosParams {
    /// `?abertos=true` → só orçamentos Rascunho/Emitido (recuperação no PDV).
    #[serde(default)]
    abertos: bool,
    #[serde(default)]
    limite: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

pub async fn listar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Query(params): Query<ListarOrcamentosParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    let orcamentos = query_dispatch(
        &*s.orcamentos,
        ListarOrcamentos {
            apenas_abertos: params.abertos,
            limite: params.limite,
            offset: params.offset,
        },
    )
    .await?;
    Ok(Json(json!({ "orcamentos": orcamentos })))
}

/// Lixeira de orçamentos — só o gestor vê e restaura.
pub async fn listar_lixeira(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Query(p): Query<PaginacaoParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let orcamentos = query_dispatch(
        &*s.orcamentos,
        ListarOrcamentosArquivados {
            limite: p.limite,
            offset: p.offset,
        },
    )
    .await?;
    Ok(Json(json!({ "orcamentos": orcamentos })))
}

pub async fn restaurar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    s.orcamentos.restaurar_arquivado(orcamento_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn buscar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    let detalhes = query_dispatch(&*s.orcamentos, BuscarOrcamento { orcamento_id }).await?;
    detalhes
        .map(|d| Json(json!(d)))
        .ok_or_else(|| AppError::NotFound.into())
}

pub async fn criar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Json(mut cmd): Json<CriarOrcamento>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.vendedor_id = user.id;
    let id = dispatch(&*s.orcamentos, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "orcamento_id": id.to_string() })),
    ))
}

pub async fn atualizar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarOrcamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.orcamento_id = orcamento_id;
    dispatch(&*s.orcamentos, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancelar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
    Json(mut cmd): Json<CancelarOrcamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.orcamento_id = orcamento_id;
    dispatch(&*s.orcamentos, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn adicionar_item(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
    Json(mut cmd): Json<AdicionarItemOrcamento>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.orcamento_id = orcamento_id;
    let item_id = dispatch(&*s.orcamentos, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "item_id": item_id.to_string() })),
    ))
}

pub async fn remover_item(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path((orcamento_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    dispatch(
        &*s.orcamentos,
        RemoverItemOrcamento {
            orcamento_id,
            item_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn aplicar_desconto(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
    Json(mut cmd): Json<AplicarDescontoOrcamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.orcamento_id = orcamento_id;
    dispatch(&*s.orcamentos, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn emitir(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    dispatch(&*s.orcamentos, EmitirOrcamento { orcamento_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn aceitar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    dispatch(&*s.orcamentos, AceitarOrcamento { orcamento_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn recusar(
    State(s): State<OrcamentosState>,
    user: AuthUser,
    Path(orcamento_id): Path<Uuid>,
    Json(mut cmd): Json<RecusarOrcamento>,
) -> Result<StatusCode, ApiError> {
    user.exigir_qualquer_role(&[Role::Vendedor])?;
    cmd.orcamento_id = orcamento_id;
    dispatch(&*s.orcamentos, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
