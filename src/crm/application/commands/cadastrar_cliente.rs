use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::Command;
use serde::Deserialize;

use crate::crm::application::handler::CrmHandlers;
use crate::crm::domain::cliente::{Cliente, ClienteId};
use crate::error::AppError;

#[derive(Command, Deserialize)]
pub struct CadastrarCliente {
    pub nome: String,
    pub cpf_cnpj: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
}

impl CommandHandler<CadastrarCliente> for CrmHandlers {
    type Output = ClienteId;
    type Error = AppError;

    async fn handle(&self, cmd: CadastrarCliente) -> Result<ClienteId, AppError> {
        let mut cliente = Cliente::cadastrar(cmd.nome, cmd.cpf_cnpj, cmd.telefone, cmd.email)?;
        let id = *cliente.id();
        self.salvar(&mut cliente).await?;
        Ok(id)
    }
}
