use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct OrcamentoResult {
    pub orcamento_id: Uuid,
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub cliente_avulso: Option<String>,
    pub total_centavos: i64,
    pub desconto_centavos: i64,
    pub status: String,
    pub validade_dias: i32,
}

#[derive(Query)]
#[query(result = Vec<OrcamentoResult>)]
pub struct ListarOrcamentos {
    /// Só orçamentos em aberto (Rascunho/Emitido) — recuperáveis no PDV.
    /// Padrão: todos.
    pub apenas_abertos: bool,
    /// Paginação opcional (aditivo): sem os params, os 200 primeiros.
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}

impl QueryHandler<ListarOrcamentos> for OrcamentosHandlers {
    type Error = AppError;

    async fn handle(&self, query: ListarOrcamentos) -> Result<Vec<OrcamentoResult>, AppError> {
        self.repo
            .listar(query.apenas_abertos, query.limite, query.offset)
            .await
    }
}
