use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;
use crate::estoque::application::queries::listar_saldos::SaldoResult;

#[derive(Query)]
#[query(result = Option<SaldoResult>)]
pub struct BuscarSaldo {
    #[trace(display)]
    pub produto_id: Uuid,
}

impl QueryHandler<BuscarSaldo> for EstoqueHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarSaldo) -> Result<Option<SaldoResult>, AppError> {
        self.repo.buscar(q.produto_id).await
    }
}
