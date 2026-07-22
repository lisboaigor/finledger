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
    pub valor_abatido: i64,
    pub descricao: Option<String>,
    pub status: String,
}

/// `limite`/`offset` opcionais (aditivo): sem eles o comportamento é o
/// histórico (200 primeiras por vencimento). Clamp em `normalizar_paginacao`.
#[derive(Query, Default)]
#[query(result = Vec<ContaReceberResult>)]
pub struct ListarContasReceber {
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}

impl QueryHandler<ListarContasReceber> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, query: ListarContasReceber) -> Result<Vec<ContaReceberResult>, AppError> {
        self.repo_receber.listar(query.limite, query.offset).await
    }
}
