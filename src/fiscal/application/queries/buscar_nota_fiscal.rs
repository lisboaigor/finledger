use pharos_app::QueryHandler;
use pharos_macros::Query;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::application::queries::listar_notas_fiscais::NotaFiscalResult;
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::sefaz::SefazClient;

#[derive(Query)]
#[query(result = Option<NotaFiscalResult>)]
pub struct BuscarNotaFiscal {
    #[trace(display)]
    pub nf_id: Uuid,
}

impl<S: SefazClient, A: AliquotaProvider> QueryHandler<BuscarNotaFiscal> for FiscalHandlers<S, A> {
    type Error = AppError;

    async fn handle(&self, q: BuscarNotaFiscal) -> Result<Option<NotaFiscalResult>, AppError> {
        self.repo.buscar(q.nf_id).await
    }
}
