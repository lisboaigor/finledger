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
    pub descricao: Option<String>,
    pub status: String,
}

/// `limite`/`offset` opcionais (aditivo): sem eles o comportamento é o
/// histórico (200 primeiras por vencimento). Clamp em `normalizar_paginacao`.
#[derive(Query, Default)]
#[query(result = Vec<ContaPagarResult>)]
pub struct ListarContasPagar {
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}

impl QueryHandler<ListarContasPagar> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, query: ListarContasPagar) -> Result<Vec<ContaPagarResult>, AppError> {
        self.repo_pagar.listar(query.limite, query.offset).await
    }
}
