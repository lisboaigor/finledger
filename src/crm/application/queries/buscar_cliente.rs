use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::crm::application::handler::CrmHandlers;
use crate::crm::application::queries::listar_clientes::ClienteResult;
use crate::error::AppError;

#[derive(Query)]
#[query(result = Option<ClienteResult>)]
pub struct BuscarCliente {
    #[trace(display)]
    pub cliente_id: Uuid,
}

impl QueryHandler<BuscarCliente> for CrmHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarCliente) -> Result<Option<ClienteResult>, AppError> {
        self.repo.buscar(q.cliente_id).await
    }
}
