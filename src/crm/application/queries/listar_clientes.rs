use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::crm::application::handler::CrmHandlers;
use crate::error::AppError;

#[derive(Serialize, sqlx::FromRow)]
pub struct ClienteResult {
    pub cliente_id: Uuid,
    pub nome: String,
    pub cpf_cnpj: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
    #[sqlx(default)]
    pub uf: Option<String>,
    pub bloqueado: bool,
    pub ativo: bool,
}

#[derive(Query)]
#[query(result = Vec<ClienteResult>)]
pub struct ListarClientes;

impl QueryHandler<ListarClientes> for CrmHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarClientes) -> Result<Vec<ClienteResult>, AppError> {
        self.repo.listar().await
    }
}
