use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::domain::ProdutoId;
use crate::error::AppError;
use crate::shared::Dinheiro;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarPrecos {
    #[external]
    pub produto_id: Uuid,
    pub preco_custo_centavos: i64,
    pub preco_venda_centavos: i64,
}

impl CommandHandler<AtualizarPrecos> for CatalogoHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarPrecos) -> Result<(), AppError> {
        let mut produto = self.load(ProdutoId::from_uuid(cmd.produto_id)).await?;
        produto.atualizar_precos(
            Dinheiro::from_centavos(cmd.preco_custo_centavos),
            Dinheiro::from_centavos(cmd.preco_venda_centavos),
        )?;
        self.salvar(&mut produto).await
    }
}
