use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::financeiro::domain::conta_receber::ContaReceberId;
use crate::shared::Dinheiro;

/// Abate parte (ou todo) do saldo em aberto de uma conta a receber —
/// devolução parcial negociada, desconto concedido etc.
#[external_fields]
#[derive(Command, Deserialize)]
pub struct RegistrarAbatimentoContaReceber {
    #[external]
    pub conta_id: Uuid,
    pub valor_centavos: i64,
    #[serde(default)]
    pub motivo: String,
}

impl CommandHandler<RegistrarAbatimentoContaReceber> for FinanceiroHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RegistrarAbatimentoContaReceber) -> Result<(), AppError> {
        let mut conta = self
            .load_receber(ContaReceberId::from_uuid(cmd.conta_id))
            .await?;
        conta.abater(Dinheiro::from_centavos(cmd.valor_centavos), cmd.motivo)?;
        self.salvar_receber(&mut conta).await
    }
}
