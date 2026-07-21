use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::OrcamentoId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct CancelarOrcamento {
    #[external]
    pub orcamento_id: Uuid,
    pub motivo: String,
}

impl CommandHandler<CancelarOrcamento> for OrcamentosHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: CancelarOrcamento) -> Result<(), AppError> {
        let mut orcamento = self.load(OrcamentoId::from_uuid(cmd.orcamento_id)).await?;
        orcamento.cancelar(cmd.motivo)?;
        self.salvar(&mut orcamento).await
    }
}
