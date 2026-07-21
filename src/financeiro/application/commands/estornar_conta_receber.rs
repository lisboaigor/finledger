use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::handler::FinanceiroHandlers;
use crate::financeiro::domain::conta_receber::ContaReceberId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct EstornarContaReceber {
    #[external]
    pub conta_id: Uuid,
    pub motivo: String,
}

impl CommandHandler<EstornarContaReceber> for FinanceiroHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: EstornarContaReceber) -> Result<(), AppError> {
        let mut conta = self
            .load_receber(ContaReceberId::from_uuid(cmd.conta_id))
            .await?;
        conta.estornar(cmd.motivo)?;
        self.salvar_receber(&mut conta).await
    }
}
