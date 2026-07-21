use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::Command;
use serde::Deserialize;

use crate::error::AppError;
use crate::identity::application::handler::{IdentityHandlers, hash_password};
use crate::identity::domain::user::{Usuario, UsuarioId};

#[derive(Command, Deserialize)]
pub struct RegistrarUsuario {
    pub username: String,
    pub senha: String,
    pub roles: Vec<String>,
}

impl CommandHandler<RegistrarUsuario> for IdentityHandlers {
    type Output = UsuarioId;
    type Error = AppError;

    async fn handle(&self, cmd: RegistrarUsuario) -> Result<UsuarioId, AppError> {
        if self.repo.username_existe(&cmd.username).await? {
            return Err(AppError::Conflict);
        }

        let hash = hash_password(&cmd.senha)?;
        let mut usuario = Usuario::registrar(cmd.username, hash, cmd.roles)?;
        let id = *usuario.id();
        self.salvar(&mut usuario).await?;
        Ok(id)
    }
}
