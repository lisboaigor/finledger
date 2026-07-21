use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::domain::pedido_compra::PedidoCompraId;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct ItemRecebidoInput {
    pub produto_id: Uuid,
    pub quantidade: u32,
}

#[external_fields]
#[derive(Command, Deserialize)]
pub struct ReceberMercadoria {
    #[external]
    pub pedido_id: Uuid,
    pub itens_recebidos: Vec<ItemRecebidoInput>,
}

impl CommandHandler<ReceberMercadoria> for ComprasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: ReceberMercadoria) -> Result<(), AppError> {
        let mut pedido = self.load(PedidoCompraId::from_uuid(cmd.pedido_id)).await?;
        let itens = cmd
            .itens_recebidos
            .into_iter()
            .map(|i| (i.produto_id, i.quantidade))
            .collect();
        pedido.receber_mercadoria(itens)?;
        self.salvar(&mut pedido).await
    }
}
