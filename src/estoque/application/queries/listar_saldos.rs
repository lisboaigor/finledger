use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct SaldoResult {
    pub produto_id: Uuid,
    pub quantidade: i32,
    pub custo_medio: i64,
    pub estoque_minimo: i32,
}

#[derive(Query)]
#[query(result = Vec<SaldoResult>)]
pub struct ListarSaldos;

impl QueryHandler<ListarSaldos> for EstoqueHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarSaldos) -> Result<Vec<SaldoResult>, AppError> {
        self.repo.listar().await
    }
}
