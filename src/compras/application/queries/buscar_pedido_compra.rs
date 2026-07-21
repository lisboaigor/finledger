use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::application::queries::listar_pedidos_compra::PedidoCompraResult;
use crate::error::AppError;

#[derive(Serialize, sqlx::FromRow)]
pub struct PedidoCompraItemResult {
    pub produto_id: Uuid,
    pub quantidade: i32,
    pub custo_unitario_centavos: i64,
}

#[derive(Serialize)]
pub struct PedidoCompraDetalhes {
    pub pedido: PedidoCompraResult,
    pub itens: Vec<PedidoCompraItemResult>,
}

#[derive(Query)]
#[query(result = Option<PedidoCompraDetalhes>)]
pub struct BuscarPedidoCompra {
    #[trace(display)]
    pub pedido_id: Uuid,
}

impl QueryHandler<BuscarPedidoCompra> for ComprasHandlers {
    type Error = AppError;

    async fn handle(
        &self,
        q: BuscarPedidoCompra,
    ) -> Result<Option<PedidoCompraDetalhes>, AppError> {
        self.repo.buscar(q.pedido_id).await
    }
}
