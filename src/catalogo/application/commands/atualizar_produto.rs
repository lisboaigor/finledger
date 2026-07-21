use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::domain::ProdutoId;
use crate::error::AppError;

fn default_true() -> bool {
    true
}

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarProduto {
    #[external]
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub ncm: String,
    pub unidade: String,
    pub categoria: String,
    #[serde(default)]
    pub marca: Option<String>,
    #[serde(default = "default_true")]
    pub controla_estoque: bool,
    #[serde(default)]
    pub classe_trib: Option<String>,
}

impl CommandHandler<AtualizarProduto> for CatalogoHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarProduto) -> Result<(), AppError> {
        let mut produto = self.load(ProdutoId::from_uuid(cmd.produto_id)).await?;
        produto.atualizar(
            cmd.sku,
            cmd.descricao,
            cmd.ncm,
            cmd.unidade,
            cmd.categoria,
            cmd.marca,
            cmd.controla_estoque,
            cmd.classe_trib,
        )?;
        self.salvar(&mut produto).await
    }
}
