use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct ContaPagarResult {
    pub conta_id: Uuid,
    pub pedido_id: Uuid,
    pub fornecedor_id: Uuid,
    pub valor_original: i64,
    pub valor_pago: i64,
    pub status: String,
}

#[derive(Query)]
#[query(result = Vec<ContaPagarResult>)]
pub struct ListarContasPagar;

impl QueryHandler<ListarContasPagar> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarContasPagar) -> Result<Vec<ContaPagarResult>, AppError> {
        self.repo_pagar.listar().await
    }
}
