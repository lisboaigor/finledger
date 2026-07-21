use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::handler::IdentityHandlers;
use crate::identity::domain::user::UsuarioId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarUsuario {
    #[external]
    pub usuario_id: Uuid,
    pub roles: Vec<String>,
}

impl CommandHandler<AtualizarUsuario> for IdentityHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarUsuario) -> Result<(), AppError> {
        let mut usuario = self.carregar(UsuarioId::from_uuid(cmd.usuario_id)).await?;
        usuario.alterar_roles(cmd.roles)?;
        self.salvar(&mut usuario).await
    }
}
