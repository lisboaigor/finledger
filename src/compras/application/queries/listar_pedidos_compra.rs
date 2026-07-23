use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::error::AppError;

#[derive(Serialize, sqlx::FromRow)]
pub struct PedidoCompraResult {
    pub pedido_id: Uuid,
    pub comprador_id: Uuid,
    pub fornecedor_id: Uuid,
    pub total_centavos: i64,
    pub prazo_pagamento_dias: i32,
    pub status: String,
}

#[derive(Query, Default)]
#[query(result = Vec<PedidoCompraResult>)]
pub struct ListarPedidosCompra {
    /// Paginação opcional (aditivo): sem os params, os 200 primeiros.
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}

impl QueryHandler<ListarPedidosCompra> for ComprasHandlers {
    type Error = AppError;

    async fn handle(&self, query: ListarPedidosCompra) -> Result<Vec<PedidoCompraResult>, AppError> {
        self.repo.listar(query.limite, query.offset).await
    }
}
