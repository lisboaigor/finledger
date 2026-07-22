use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
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
    /// Total da nota (produtos − desconto).
    pub total_centavos: i64,
    /// Desconto global destacado na NF — campo aditivo (0 em notas antigas).
    #[sqlx(default)]
    #[serde(default)]
    pub desconto_centavos: i64,
    pub cancelamento_pendente: bool,
    // Breakdown de impostos (reforma tributária) — campos aditivos: notas
    // anteriores ao motor têm 0 na projeção.
    #[sqlx(default)]
    #[serde(default)]
    pub icms_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub pis_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub cofins_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub iss_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub cbs_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub ibs_uf_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub ibs_mun_centavos: i64,
    #[sqlx(default)]
    #[serde(default)]
    pub is_centavos: i64,
}

#[derive(Query)]
#[query(result = Vec<NotaFiscalResult>)]
pub struct ListarNotasFiscais;

impl<S: SefazClient, A: AliquotaProvider> QueryHandler<ListarNotasFiscais> for FiscalHandlers<S, A> {
    type Error = AppError;

    async fn handle(&self, _query: ListarNotasFiscais) -> Result<Vec<NotaFiscalResult>, AppError> {
        self.repo.listar().await
    }
}
