use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct RemoverItemVenda {
    #[external]
    pub venda_id: Uuid,
    pub item_id: Uuid,
}

impl CommandHandler<RemoverItemVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RemoverItemVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.remover_item(cmd.item_id)?;
        self.salvar(&mut venda).await
    }
}
