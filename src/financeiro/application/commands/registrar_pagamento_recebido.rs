use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::financeiro::domain::conta_receber::ContaReceberId;
use crate::shared::Dinheiro;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct RegistrarPagamentoRecebido {
    #[external]
    pub conta_id: Uuid,
    pub valor_centavos: i64,
}

impl CommandHandler<RegistrarPagamentoRecebido> for FinanceiroHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RegistrarPagamentoRecebido) -> Result<(), AppError> {
        let mut conta = self
            .load_receber(ContaReceberId::from_uuid(cmd.conta_id))
            .await?;
        conta.registrar_pagamento(Dinheiro::from_centavos(cmd.valor_centavos))?;
        self.salvar_receber(&mut conta).await
    }
}
