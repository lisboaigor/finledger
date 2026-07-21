use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::domain::ProdutoId;
use crate::error::AppError;

#[derive(Command, Deserialize)]
pub struct ReativarProduto {
    pub produto_id: Uuid,
}

impl CommandHandler<ReativarProduto> for CatalogoHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: ReativarProduto) -> Result<(), AppError> {
        let mut produto = self.load(ProdutoId::from_uuid(cmd.produto_id)).await?;
        produto.reativar()?;
        self.salvar(&mut produto).await
    }
}
