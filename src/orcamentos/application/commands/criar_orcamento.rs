use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::{Orcamento, OrcamentoId};

#[external_fields]
#[derive(Command, Deserialize)]
pub struct CriarOrcamento {
    #[external]
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
    /// Nome informal do cliente quando não há cadastro completo no CRM
    /// (atendimento de balcão). Ignorado se `cliente_id` também vier
    /// preenchido — ver `Orcamento::criar`.
    #[serde(default)]
    pub cliente_avulso: Option<String>,
    pub validade_dias: u16,
}

impl CommandHandler<CriarOrcamento> for OrcamentosHandlers {
    type Output = OrcamentoId;
    type Error = AppError;

    async fn handle(&self, cmd: CriarOrcamento) -> Result<OrcamentoId, AppError> {
        let mut orcamento = Orcamento::criar(
            cmd.vendedor_id,
            cmd.cliente_id,
            cmd.cliente_avulso,
            cmd.validade_dias,
        )?;
        let id = *orcamento.id();
        self.salvar(&mut orcamento).await?;
        Ok(id)
    }
}
