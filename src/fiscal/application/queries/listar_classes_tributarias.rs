use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::sefaz::SefazClient;

/// Classes tributárias de referência (cClassTrib, NT 2025.002) — alimenta o
/// select de classificação do produto no catálogo. Dado global, sem tenant.
#[derive(Serialize, sqlx::FromRow)]
pub struct ClasseTributariaResult {
    pub c_class_trib: String,
    pub descricao: String,
    pub cst_ibs_cbs: String,
    pub reducao_bps: i32,
}

#[derive(Query)]
#[query(result = Vec<ClasseTributariaResult>)]
pub struct ListarClassesTributarias;

impl<S: SefazClient, A: AliquotaProvider> QueryHandler<ListarClassesTributarias>
    for FiscalHandlers<S, A>
{
    type Error = AppError;

    async fn handle(
        &self,
        _query: ListarClassesTributarias,
    ) -> Result<Vec<ClasseTributariaResult>, AppError> {
        sqlx::query_as(
            "SELECT c_class_trib, descricao, cst_ibs_cbs, reducao_bps
             FROM ref_classes_tributarias ORDER BY c_class_trib",
        )
        .fetch_all(self.repo.pool())
        .await
        .map_err(AppError::infra)
    }
}
