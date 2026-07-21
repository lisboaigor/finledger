use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::handler::{IdentityHandlers, hash_password, verify_password};
use crate::identity::domain::user::UsuarioId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AlterarSenha {
    #[external]
    pub usuario_id: Uuid,
    pub senha_atual: String,
    pub nova_senha: String,
}

impl CommandHandler<AlterarSenha> for IdentityHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AlterarSenha) -> Result<(), AppError> {
        let mut usuario = self.carregar(UsuarioId::from_uuid(cmd.usuario_id)).await?;

        if !verify_password(&cmd.senha_atual, &usuario.password_hash) {
            return Err(AppError::Unauthorized);
        }

        let nova_hash = hash_password(&cmd.nova_senha)?;
        usuario.alterar_senha(nova_hash)?;
        self.salvar(&mut usuario).await
    }
}
