use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::financeiro::application::queries::listar_contas_receber::ContaReceberResult;

#[derive(Query)]
#[query(result = Option<ContaReceberResult>)]
pub struct BuscarContaReceber {
    #[trace(display)]
    pub conta_id: Uuid,
}

impl QueryHandler<BuscarContaReceber> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarContaReceber) -> Result<Option<ContaReceberResult>, AppError> {
        self.repo_receber.buscar(q.conta_id).await
    }
}
