use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::fornecedores::application::handler::FornecedoresHandlers;
use crate::fornecedores::application::queries::listar_fornecedores::FornecedorResult;

#[derive(Query)]
#[query(result = Option<FornecedorResult>)]
pub struct BuscarFornecedor {
    #[trace(display)]
    pub fornecedor_id: Uuid,
}

impl QueryHandler<BuscarFornecedor> for FornecedoresHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarFornecedor) -> Result<Option<FornecedorResult>, AppError> {
        self.repo.buscar(q.fornecedor_id).await
    }
}
