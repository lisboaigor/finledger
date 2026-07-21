use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::OrcamentoId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct RemoverItemOrcamento {
    #[external]
    pub orcamento_id: Uuid,
    pub item_id: Uuid,
}

impl CommandHandler<RemoverItemOrcamento> for OrcamentosHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RemoverItemOrcamento) -> Result<(), AppError> {
        let mut orcamento = self.load(OrcamentoId::from_uuid(cmd.orcamento_id)).await?;
        orcamento.remover_item(cmd.item_id)?;
        self.salvar(&mut orcamento).await
    }
}
