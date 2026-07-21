use std::sync::Arc;

use pharos_app::EventBus;

use crate::compras::domain::pedido_compra::{PedidoCompra, PedidoCompraId};
use crate::compras::infrastructure::repository::PostgresPedidoCompraRepository;
use crate::error::AppError;
use crate::shared::{load_aggregate, salvar_aggregate};

pub struct ComprasHandlers {
    pub(crate) repo: Arc<PostgresPedidoCompraRepository>,
    pub(crate) bus: EventBus,
}

impl ComprasHandlers {
    pub fn new(repo: Arc<PostgresPedidoCompraRepository>, bus: EventBus) -> Self {
        Self { repo, bus }
    }

    pub(crate) async fn load(&self, id: PedidoCompraId) -> Result<PedidoCompra, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, pedido: &mut PedidoCompra) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, pedido).await
    }
}
