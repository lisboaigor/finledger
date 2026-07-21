use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::infrastructure::repository::ProdutoResult;
use crate::error::AppError;

#[derive(Query)]
#[query(result = Option<ProdutoResult>)]
pub struct BuscarProduto {
    #[trace(display)]
    pub produto_id: Uuid,
}

impl QueryHandler<BuscarProduto> for CatalogoHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarProduto) -> Result<Option<ProdutoResult>, AppError> {
        self.repo.buscar(q.produto_id).await
    }
}
