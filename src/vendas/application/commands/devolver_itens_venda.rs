use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[derive(Deserialize)]
pub struct DevolucaoItem {
    pub item_id: Uuid,
    pub quantidade: u32,
}

#[external_fields]
#[derive(Command, Deserialize)]
pub struct DevolverItensVenda {
    #[external]
    pub venda_id: Uuid,
    pub itens: Vec<DevolucaoItem>,
    pub motivo: String,
}

impl CommandHandler<DevolverItensVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: DevolverItensVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        let devolucoes: Vec<(uuid::Uuid, u32)> = cmd
            .itens
            .iter()
            .map(|i| (i.item_id, i.quantidade))
            .collect();
        venda.devolver_itens(&devolucoes, cmd.motivo)?;
        // Estoque (reentrada), financeiro (estorno em devolução total) e fiscal
        // (cancelamento/reemissão da NF) reagem aos eventos publicados abaixo.
        self.salvar(&mut venda).await
    }
}
