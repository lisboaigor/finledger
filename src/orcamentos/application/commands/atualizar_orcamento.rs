use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::OrcamentoId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarOrcamento {
    #[external]
    pub orcamento_id: Uuid,
    pub cliente_id: Option<Uuid>,
    #[serde(default)]
    pub cliente_avulso: Option<String>,
    pub validade_dias: u16,
}

impl CommandHandler<AtualizarOrcamento> for OrcamentosHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarOrcamento) -> Result<(), AppError> {
        let mut orcamento = self.load(OrcamentoId::from_uuid(cmd.orcamento_id)).await?;
        orcamento.atualizar(cmd.cliente_id, cmd.cliente_avulso, cmd.validade_dias)?;
        self.salvar(&mut orcamento).await
    }
}
