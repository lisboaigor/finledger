use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::handler::IdentityHandlers;
use crate::identity::domain::user::UsuarioId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct ReativarUsuario {
    #[external]
    pub usuario_id: Uuid,
}

impl CommandHandler<ReativarUsuario> for IdentityHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: ReativarUsuario) -> Result<(), AppError> {
        let mut usuario = self.carregar(UsuarioId::from_uuid(cmd.usuario_id)).await?;
        usuario.reativar()?;
        self.salvar(&mut usuario).await
    }
}
