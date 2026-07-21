use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::identity::domain::events::IdentityEvent;
use crate::shared::tenant::current_tenant_id;

pub struct IdentityProjection {
    pool: Pool,
}

impl IdentityProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &IdentityEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            IdentityEvent::UsuarioCriado {
                usuario_id,
                username,
                password_hash,
                roles,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("usuario_id", usuario_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_usuarios
                        (usuario_id, username, password_hash, roles, ativo, criado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, TRUE, $5, $6)
                     ON CONFLICT (tenant_id, usuario_id) DO NOTHING",
                )
                .bind(id)
                .bind(username.as_str())
                .bind(password_hash.as_str())
                .bind(roles.as_str())
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            IdentityEvent::SenhaAlterada {
                usuario_id,
                password_hash,
                ..
            } => {
                let Some(id) = crate::projections::parse_uuid("usuario_id", usuario_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_usuarios SET password_hash = $2 WHERE usuario_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(password_hash.as_str())
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            IdentityEvent::UsuarioDesativado { usuario_id, .. } => {
                let Some(id) = crate::projections::parse_uuid("usuario_id", usuario_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_usuarios SET ativo = FALSE WHERE usuario_id = $1 AND tenant_id = $2",
                )
                .bind(id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            IdentityEvent::UsuarioReativado { usuario_id, .. } => {
                let Some(id) = crate::projections::parse_uuid("usuario_id", usuario_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_usuarios SET ativo = TRUE WHERE usuario_id = $1 AND tenant_id = $2",
                )
                .bind(id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            IdentityEvent::RolesAlteradas {
                usuario_id, roles, ..
            } => {
                let Some(id) = crate::projections::parse_uuid("usuario_id", usuario_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_usuarios SET roles = $2 WHERE usuario_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(roles.as_str())
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<IdentityEvent> for IdentityProjection {
    type Error = Infallible;

    async fn handle(&self, event: &IdentityEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("identity projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "identity projection failed");
        }
        Ok(())
    }
}
