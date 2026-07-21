use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::sefaz::SefazClient;

#[derive(Serialize, sqlx::FromRow)]
pub struct NotaFiscalResult {
    pub nf_id: Uuid,
    pub venda_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub modelo: String,
    pub serie: String,
    pub numero: i32,
    pub chave: Option<String>,
    pub status: String,
    pub total_centavos: i64,
    pub cancelamento_pendente: bool,
}

#[derive(Query)]
#[query(result = Vec<NotaFiscalResult>)]
pub struct ListarNotasFiscais;

impl<S: SefazClient> QueryHandler<ListarNotasFiscais> for FiscalHandlers<S> {
    type Error = AppError;

    async fn handle(&self, _query: ListarNotasFiscais) -> Result<Vec<NotaFiscalResult>, AppError> {
        self.repo.listar().await
    }
}
