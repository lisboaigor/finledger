use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct DefinirEstoqueMinimo {
    #[external]
    pub produto_id: Uuid,
    pub estoque_minimo: u32,
}

impl CommandHandler<DefinirEstoqueMinimo> for EstoqueHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: DefinirEstoqueMinimo) -> Result<(), AppError> {
        let mut item = self.load_ou_criar(cmd.produto_id).await?;
        item.definir_estoque_minimo(cmd.estoque_minimo)?;
        self.salvar(&mut item).await
    }
}
