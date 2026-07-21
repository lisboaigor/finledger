use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::application::queries::listar_orcamentos::OrcamentoResult;

#[derive(Serialize, sqlx::FromRow)]
pub struct OrcamentoItemResult {
    pub item_id: Uuid,
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub quantidade: i32,
    pub preco_unitario_centavos: i64,
}

#[derive(Serialize)]
pub struct OrcamentoDetalhes {
    pub orcamento: OrcamentoResult,
    pub itens: Vec<OrcamentoItemResult>,
}

#[derive(Query)]
#[query(result = Option<OrcamentoDetalhes>)]
pub struct BuscarOrcamento {
    #[trace(display)]
    pub orcamento_id: Uuid,
}

impl QueryHandler<BuscarOrcamento> for OrcamentosHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarOrcamento) -> Result<Option<OrcamentoDetalhes>, AppError> {
        self.repo.buscar(q.orcamento_id).await
    }
}
