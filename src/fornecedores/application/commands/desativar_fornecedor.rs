use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fornecedores::application::handler::FornecedoresHandlers;
use crate::fornecedores::domain::fornecedor::FornecedorId;

#[derive(Command, Deserialize)]
pub struct DesativarFornecedor {
    pub fornecedor_id: Uuid,
}

impl CommandHandler<DesativarFornecedor> for FornecedoresHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: DesativarFornecedor) -> Result<(), AppError> {
        let mut fornecedor = self
            .load(FornecedorId::from_uuid(cmd.fornecedor_id))
            .await?;
        fornecedor.desativar()?;
        self.salvar(&mut fornecedor).await
    }
}
