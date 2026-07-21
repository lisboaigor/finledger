use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::domain::pedido_compra::PedidoCompraId;
use crate::error::AppError;

#[derive(Command, Deserialize)]
pub struct EnviarPedidoCompra {
    pub pedido_id: Uuid,
}

impl CommandHandler<EnviarPedidoCompra> for ComprasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: EnviarPedidoCompra) -> Result<(), AppError> {
        let mut pedido = self.load(PedidoCompraId::from_uuid(cmd.pedido_id)).await?;
        pedido.enviar()?;
        self.salvar(&mut pedido).await
    }
}
