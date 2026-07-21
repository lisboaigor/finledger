use pharos_app::QueryHandler;
use pharos_macros::Query;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::infrastructure::repository::ProdutoResult;
use crate::error::AppError;

#[derive(Query)]
#[query(result = Vec<ProdutoResult>)]
pub struct ListarProdutos;

impl QueryHandler<ListarProdutos> for CatalogoHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarProdutos) -> Result<Vec<ProdutoResult>, AppError> {
        self.repo.listar().await
    }
}
