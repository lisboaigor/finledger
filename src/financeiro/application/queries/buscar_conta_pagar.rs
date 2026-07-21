use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::financeiro::application::queries::listar_contas_pagar::ContaPagarResult;

#[derive(Query)]
#[query(result = Option<ContaPagarResult>)]
pub struct BuscarContaPagar {
    #[trace(display)]
    pub conta_id: Uuid,
}

impl QueryHandler<BuscarContaPagar> for FinanceiroHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarContaPagar) -> Result<Option<ContaPagarResult>, AppError> {
        self.repo_pagar.buscar(q.conta_id).await
    }
}
