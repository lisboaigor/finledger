use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::OrcamentoId;

/// Marca um orçamento aceito como convertido, ligando-o à venda gerada. Não
/// vem de requisição HTTP — é despachado internamente pelo assinante de
/// `OrcamentoAceito` após criar a venda (ver `VendaAPartirDeOrcamentoHandler`).
#[external_fields]
#[derive(Command, Deserialize)]
pub struct MarcarConvertidoOrcamento {
    #[external]
    pub orcamento_id: Uuid,
    #[external]
    pub venda_id: Uuid,
}

impl CommandHandler<MarcarConvertidoOrcamento> for OrcamentosHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: MarcarConvertidoOrcamento) -> Result<(), AppError> {
        let mut orcamento = self.load(OrcamentoId::from_uuid(cmd.orcamento_id)).await?;
        orcamento.marcar_convertido(cmd.venda_id)?;
        self.salvar(&mut orcamento).await
    }
}
