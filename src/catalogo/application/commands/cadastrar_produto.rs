use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;

use crate::catalogo::application::handler::CatalogoHandlers;
use crate::catalogo::domain::{Produto, ProdutoId};
use crate::error::AppError;
use crate::shared::Dinheiro;

fn default_true() -> bool {
    true
}

#[derive(Command, Deserialize)]
pub struct CadastrarProduto {
    pub sku: String,
    pub descricao: String,
    pub ncm: String,
    pub unidade: String,
    pub preco_custo_centavos: i64,
    pub preco_venda_centavos: i64,
    pub categoria: String,
    #[serde(default)]
    pub marca: Option<String>,
    /// FALSE para serviços/itens sem estoque físico (ex.: mão de obra).
    #[serde(default = "default_true")]
    pub controla_estoque: bool,
}

/// Manipuladores de comandos para o catálogo de produtos.
///
///
impl CommandHandler<CadastrarProduto> for CatalogoHandlers {
    type Output = ProdutoId;
    type Error = AppError;

    async fn handle(&self, cmd: CadastrarProduto) -> Result<ProdutoId, AppError> {
        let mut produto = Produto::cadastrar(
            cmd.sku,
            cmd.descricao,
            cmd.ncm,
            cmd.unidade,
            Dinheiro::from_centavos(cmd.preco_custo_centavos),
            Dinheiro::from_centavos(cmd.preco_venda_centavos),
            cmd.categoria,
            cmd.marca,
            cmd.controla_estoque,
        )?;
        let id = *produto.id();
        self.salvar(&mut produto).await?;
        Ok(id)
    }
}
