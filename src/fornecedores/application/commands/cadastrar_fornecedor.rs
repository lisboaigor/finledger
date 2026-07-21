use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::Command;
use serde::Deserialize;

use crate::error::AppError;
use crate::fornecedores::application::handler::FornecedoresHandlers;
use crate::fornecedores::domain::fornecedor::{Fornecedor, FornecedorId};

#[derive(Command, Deserialize)]
pub struct CadastrarFornecedor {
    pub razao_social: String,
    pub cnpj: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub prazo_pagamento_dias: u16,
}

impl CommandHandler<CadastrarFornecedor> for FornecedoresHandlers {
    type Output = FornecedorId;
    type Error = AppError;

    async fn handle(&self, cmd: CadastrarFornecedor) -> Result<FornecedorId, AppError> {
        let mut fornecedor = Fornecedor::cadastrar(
            cmd.razao_social,
            cmd.cnpj,
            cmd.telefone,
            cmd.email,
            cmd.prazo_pagamento_dias,
        )?;
        let id = *fornecedor.id();
        self.salvar(&mut fornecedor).await?;
        Ok(id)
    }
}
