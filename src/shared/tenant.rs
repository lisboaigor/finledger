//! Acesso ao tenant da requisição atual.
//!
//! O `TenantContext` é colocado no task-local `CURRENT_TENANT` pelo middleware de
//! autenticação (`require_auth`) e propaga, no mesmo task assíncrono, para command
//! handlers, repositórios, projeções e handlers cross-context — sem precisar carregar
//! o `tenant_id` no payload de cada evento.

use pharos_app::{CURRENT_TENANT, TenantContext};
use uuid::Uuid;

use crate::error::AppError;

/// Lê o `TenantContext` da requisição atual, se houver um em escopo.
pub fn current_tenant() -> Option<TenantContext> {
    CURRENT_TENANT.try_with(|t| *t).ok().flatten()
}

/// UUID do tenant da requisição atual.
///
/// Retorna `AppError::Unauthorized` (deny-by-default) quando não há tenant em escopo —
/// nenhuma query a dados de tenant deve rodar fora de um contexto de tenant.
/// Retorna `AppError::BadRequest` se o claim não for um UUID válido (nunca deve ocorrer
/// com tokens bem formados).
pub fn current_tenant_id() -> Result<Uuid, AppError> {
    let ctx = current_tenant().ok_or(AppError::Unauthorized)?;
    Ok(ctx.tenant_id().as_uuid())
}
