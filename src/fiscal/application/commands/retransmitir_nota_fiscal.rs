use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::infrastructure::sefaz::SefazClient;

/// Retransmite uma NF que está no status Gerada (ex: após correção de rejeição).
#[derive(Command, Debug, Deserialize, Serialize)]
pub struct RetransmitirNotaFiscal {
    pub nf_id: Uuid,
}

impl<S: SefazClient> CommandHandler<RetransmitirNotaFiscal> for FiscalHandlers<S> {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RetransmitirNotaFiscal) -> Result<(), AppError> {
        self.retransmitir(cmd).await
    }
}
