use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::handler::IdentityHandlers;
use crate::identity::application::queries::listar_usuarios::UsuarioResult;

#[derive(Query)]
#[query(result = Option<UsuarioResult>)]
pub struct BuscarUsuario {
    #[trace(display)]
    pub usuario_id: Uuid,
}

impl QueryHandler<BuscarUsuario> for IdentityHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarUsuario) -> Result<Option<UsuarioResult>, AppError> {
        self.repo.buscar(q.usuario_id).await
    }
}
