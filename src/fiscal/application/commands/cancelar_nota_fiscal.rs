use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::sefaz::SefazClient;

/// Cancela uma NF já autorizada. Deve ser feito dentro de 24h da autorização.
#[external_fields]
#[derive(Command, Debug, Deserialize, Serialize)]
pub struct CancelarNotaFiscal {
    #[external]
    pub nf_id: Uuid,
    pub motivo: String,
}

impl<S: SefazClient, A: AliquotaProvider> CommandHandler<CancelarNotaFiscal> for FiscalHandlers<S, A> {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: CancelarNotaFiscal) -> Result<(), AppError> {
        self.cancelar(cmd).await
    }
}
