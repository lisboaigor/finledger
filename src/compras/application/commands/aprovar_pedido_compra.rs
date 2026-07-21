use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::domain::pedido_compra::PedidoCompraId;
use crate::error::AppError;

#[derive(Command, Deserialize)]
pub struct AprovarPedidoCompra {
    pub pedido_id: Uuid,
    pub aprovador_id: Uuid,
}

impl CommandHandler<AprovarPedidoCompra> for ComprasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AprovarPedidoCompra) -> Result<(), AppError> {
        let mut pedido = self.load(PedidoCompraId::from_uuid(cmd.pedido_id)).await?;
        pedido.aprovar(cmd.aprovador_id)?;
        self.salvar(&mut pedido).await
    }
}
