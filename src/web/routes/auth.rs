use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use pharos_app::{dispatch, query_dispatch};
use serde_json::json;
use uuid::Uuid;

use crate::auth::{AuthUser, Roles};
use crate::error::AppError;
use crate::identity::application::commands::{
    AlterarSenha, AtualizarUsuario, DesativarUsuario, Login, ReativarUsuario, RegistrarUsuario,
};
use crate::identity::application::queries::{BuscarUsuario, ListarUsuarios};
use crate::web::{error::ApiError, state::IdentityState};

pub async fn listar_usuarios(
    State(s): State<IdentityState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let usuarios = query_dispatch(&*s.identity, ListarUsuarios).await?;
    Ok(Json(json!({ "usuarios": usuarios })))
}

pub async fn buscar_usuario(
    State(s): State<IdentityState>,
    user: AuthUser,
    Path(usuario_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let usuario = query_dispatch(&*s.identity, BuscarUsuario { usuario_id }).await?;
    usuario
        .map(|u| Json(json!(u)))
        .ok_or(AppError::NotFound.into())
}

pub async fn registrar(
    State(s): State<IdentityState>,
    user: AuthUser,
    Json(cmd): Json<RegistrarUsuario>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    let id = dispatch(&*s.identity, cmd).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "usuario_id": id.to_string() })),
    ))
}

pub async fn login(
    State(s): State<IdentityState>,
    Json(cmd): Json<Login>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let token = dispatch(&*s.identity, cmd).await?;
    Ok(Json(json!({ "token": token })))
}

pub async fn alterar_senha(
    State(s): State<IdentityState>,
    user: AuthUser,
    Json(mut cmd): Json<AlterarSenha>,
) -> Result<StatusCode, ApiError> {
    cmd.usuario_id = user.id;
    dispatch(&*s.identity, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desativar_usuario(
    State(s): State<IdentityState>,
    user: AuthUser,
    Path(usuario_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.identity, DesativarUsuario { usuario_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reativar_usuario(
    State(s): State<IdentityState>,
    user: AuthUser,
    Path(usuario_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    dispatch(&*s.identity, ReativarUsuario { usuario_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn atualizar_usuario(
    State(s): State<IdentityState>,
    user: AuthUser,
    Path(usuario_id): Path<Uuid>,
    Json(mut cmd): Json<AtualizarUsuario>,
) -> Result<StatusCode, ApiError> {
    user.exigir_role(Roles::ADMIN)?;
    cmd.usuario_id = usuario_id;
    dispatch(&*s.identity, cmd).await?;
    Ok(StatusCode::NO_CONTENT)
}
