use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fornecedores::application::handler::FornecedoresHandlers;
use crate::fornecedores::domain::fornecedor::FornecedorId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarFornecedor {
    #[external]
    pub fornecedor_id: Uuid,
    pub razao_social: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub prazo_pagamento_dias: u16,
}

impl CommandHandler<AtualizarFornecedor> for FornecedoresHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarFornecedor) -> Result<(), AppError> {
        let mut fornecedor = self
            .load(FornecedorId::from_uuid(cmd.fornecedor_id))
            .await?;
        fornecedor.atualizar(
            cmd.razao_social,
            cmd.telefone,
            cmd.email,
            cmd.prazo_pagamento_dias,
        )?;
        self.salvar(&mut fornecedor).await
    }
}
