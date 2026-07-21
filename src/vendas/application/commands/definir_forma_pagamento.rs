use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::value_objects::FormaPagamento;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct DefinirFormaPagamento {
    #[external]
    pub venda_id: Uuid,
    pub forma: FormaPagamento,
}

impl CommandHandler<DefinirFormaPagamento> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: DefinirFormaPagamento) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.definir_forma_pagamento(cmd.forma)?;
        self.salvar(&mut venda).await
    }
}
