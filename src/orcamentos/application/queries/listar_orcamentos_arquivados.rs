use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;

/// Lixeira: orçamentos arquivados pela rotina de limpeza (visão do gestor).
#[derive(Query)]
#[query(result = Vec<OrcamentoArquivadoResult>)]
pub struct ListarOrcamentosArquivados;

/// Linha da lixeira: mesma leitura da listagem + quando foi criado/arquivado.
#[derive(Serialize, sqlx::FromRow)]
pub struct OrcamentoArquivadoResult {
    pub orcamento_id: Uuid,
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub cliente_avulso: Option<String>,
    pub total_centavos: i64,
    pub desconto_centavos: i64,
    pub status: String,
    pub validade_dias: i32,
    pub criado_em: chrono::DateTime<chrono::Utc>,
    pub arquivado_em: chrono::DateTime<chrono::Utc>,
}

impl QueryHandler<ListarOrcamentosArquivados> for OrcamentosHandlers {
    type Error = AppError;

    async fn handle(
        &self,
        _q: ListarOrcamentosArquivados,
    ) -> Result<Vec<OrcamentoArquivadoResult>, AppError> {
        self.repo.listar_lixeira().await
    }
}
