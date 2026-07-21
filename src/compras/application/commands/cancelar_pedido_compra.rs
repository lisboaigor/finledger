use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::domain::pedido_compra::PedidoCompraId;
use crate::error::AppError;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct CancelarPedidoCompra {
    #[external]
    pub pedido_id: Uuid,
    pub motivo: String,
}

impl CommandHandler<CancelarPedidoCompra> for ComprasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: CancelarPedidoCompra) -> Result<(), AppError> {
        let mut pedido = self.load(PedidoCompraId::from_uuid(cmd.pedido_id)).await?;
        pedido.cancelar(cmd.motivo)?;
        self.salvar(&mut pedido).await
    }
}
