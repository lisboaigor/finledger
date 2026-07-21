use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct ConfirmarVenda {
    #[external]
    pub venda_id: Uuid,
}

impl CommandHandler<ConfirmarVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: ConfirmarVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.confirmar()?;
        // Baixa de estoque acontece de forma assíncrona: EstoqueVendaEventHandler
        // reage ao VendaConfirmada publicado abaixo (ver bootstrap/events.rs).
        self.salvar(&mut venda).await
    }
}
