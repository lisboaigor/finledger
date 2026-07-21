use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AjustarEstoque {
    #[external]
    pub produto_id: Uuid,
    pub quantidade_nova: u32,
    pub justificativa: String,
}

impl CommandHandler<AjustarEstoque> for EstoqueHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AjustarEstoque) -> Result<(), AppError> {
        let mut item = self.load_ou_criar(cmd.produto_id).await?;
        item.ajustar(cmd.quantidade_nova, cmd.justificativa)?;
        self.salvar(&mut item).await
    }
}
