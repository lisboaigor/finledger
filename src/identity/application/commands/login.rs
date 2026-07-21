use pharos_app::{CURRENT_TENANT, CommandHandler, TenantContext};
use pharos_macros::Command;
use serde::Deserialize;

use crate::auth::jwt;
use crate::error::AppError;
use crate::identity::application::handler::{IdentityHandlers, verify_password};

#[derive(Command, Deserialize)]
pub struct Login {
    pub slug: String,
    #[trace]
    pub username: String,
    pub senha: String,
}

impl CommandHandler<Login> for IdentityHandlers {
    type Output = String;
    type Error = AppError;

    async fn handle(&self, cmd: Login) -> Result<String, AppError> {
        let tenant_row = self
            .tenants
            .buscar_por_slug(&cmd.slug)
            .await?
            .ok_or_else(|| {
                tracing::warn!(slug = %cmd.slug, "tentativa de login: tenant não encontrado");
                AppError::Unauthorized
            })?;

        if tenant_row.status != "ativo" {
            tracing::warn!(slug = %cmd.slug, "tentativa de login: tenant suspenso");
            return Err(AppError::Unauthorized);
        }

        let tenant_id_str = tenant_row.tenant_id.to_string();
        let tenant_slug = tenant_row.slug.clone();
        let tenant_ctx = TenantContext::new(tenant_row.tenant_id);
        let secret = self.auth.secret.clone();
        let username = cmd.username.clone();
        let senha = cmd.senha.clone();
        let repo = self.repo.clone();

        CURRENT_TENANT
            .scope(Some(tenant_ctx), async move {
                let row = repo.buscar_para_login(&username).await?;

                let (usuario_id, password_hash, roles_str, ativo) = row.ok_or_else(|| {
                    tracing::warn!(username = %username, "tentativa de login: usuário não encontrado");
                    AppError::Unauthorized
                })?;

                if !ativo {
                    tracing::warn!(username = %username, "tentativa de login: usuário inativo");
                    return Err(AppError::Unauthorized);
                }

                if !verify_password(&senha, &password_hash) {
                    tracing::warn!(username = %username, "tentativa de login: senha incorreta");
                    return Err(AppError::Unauthorized);
                }

                let roles: Vec<String> = roles_str
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();

                let token = jwt::encode_token(
                    usuario_id,
                    &username,
                    roles,
                    &tenant_id_str,
                    &tenant_slug,
                    &secret,
                    24,
                )?;
                tracing::info!(username = %username, %usuario_id, "login bem-sucedido");
                Ok(token)
            })
            .await
    }
}
