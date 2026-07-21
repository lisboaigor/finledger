use pharos_app::CommandHandler;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;
use crate::shared::Dinheiro;

#[derive(Command, Deserialize)]
pub struct RegistrarEntradaEstoque {
    pub produto_id: Uuid,
    pub quantidade: u32,
    pub custo_unitario_centavos: i64,
    pub motivo: String,
    /// Supplier invoice number this entry came from, when applicable.
    #[serde(default)]
    pub nota_fiscal: Option<String>,
}

impl CommandHandler<RegistrarEntradaEstoque> for EstoqueHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: RegistrarEntradaEstoque) -> Result<(), AppError> {
        let mut item = self.load_ou_criar(cmd.produto_id).await?;
        item.registrar_entrada(
            cmd.quantidade,
            Dinheiro::from_centavos(cmd.custo_unitario_centavos),
            cmd.motivo,
            cmd.nota_fiscal,
        )?;
        self.salvar(&mut item).await
    }
}
