use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::handler::IdentityHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct UsuarioResult {
    pub usuario_id: Uuid,
    pub username: String,
    pub roles: String,
    pub ativo: bool,
}

#[derive(Query)]
#[query(result = Vec<UsuarioResult>)]
pub struct ListarUsuarios;

impl QueryHandler<ListarUsuarios> for IdentityHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarUsuarios) -> Result<Vec<UsuarioResult>, AppError> {
        self.repo.listar().await
    }
}
