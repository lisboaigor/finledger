use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct CancelarVenda {
    #[external]
    pub venda_id: Uuid,
    pub motivo: String,
}

impl CommandHandler<CancelarVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: CancelarVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.cancelar(cmd.motivo)?;
        self.salvar(&mut venda).await
    }
}
