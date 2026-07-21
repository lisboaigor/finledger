use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::infrastructure::precificacao_repository::ElasticidadeResultado;
use crate::error::AppError;

/// Variação de vendas observada no último reajuste de preço do produto —
/// None quando o histórico não tem dados suficientes para um número honesto.
#[derive(Query)]
#[query(result = Option<ElasticidadeResultado>)]
pub struct ObterElasticidade {
    #[trace(display)]
    pub produto_id: Uuid,
}

impl QueryHandler<ObterElasticidade> for CatalogoHandlers {
    type Error = AppError;

    async fn handle(
        &self,
        q: ObterElasticidade,
    ) -> Result<Option<ElasticidadeResultado>, AppError> {
        self.precificacao.elasticidade(q.produto_id).await
    }
}
