use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::crm::application::handler::CrmHandlers;
use crate::crm::domain::cliente::ClienteId;
use crate::error::AppError;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct DesbloquearCliente {
    #[external]
    pub cliente_id: Uuid,
}

impl CommandHandler<DesbloquearCliente> for CrmHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: DesbloquearCliente) -> Result<(), AppError> {
        let mut cliente = self.load(ClienteId::from_uuid(cmd.cliente_id)).await?;
        cliente.desbloquear()?;
        self.salvar(&mut cliente).await
    }
}
