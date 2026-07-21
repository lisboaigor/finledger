use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct ContaReceberResult {
    pub conta_id: Uuid,
    pub venda_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub valor_original: i64,
    pub valor_recebido: i64,
    pub status: String,
}

#[derive(Query)]
#[query(result = Vec<ContaReceberResult>)]
pub struct ListarContasReceber;

impl QueryHandler<ListarContasReceber> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarContasReceber) -> Result<Vec<ContaReceberResult>, AppError> {
        self.repo_receber.listar().await
    }
}
