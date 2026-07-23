use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;
use crate::shared::Dinheiro;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AjustarEstoque {
    #[external]
    pub produto_id: Uuid,
    pub quantidade_nova: u32,
    // Obrigatório quando o ajuste aumenta o saldo (validado no domínio).
    #[serde(default)]
    pub custo_unitario_centavos: Option<i64>,
    pub justificativa: String,
}

impl CommandHandler<AjustarEstoque> for EstoqueHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AjustarEstoque) -> Result<(), AppError> {
        let mut item = self.load_ou_criar(cmd.produto_id).await?;
        let custo = cmd.custo_unitario_centavos.map(Dinheiro::from_centavos);
        item.ajustar(cmd.quantidade_nova, custo, cmd.justificativa)?;
        self.salvar(&mut item).await
    }
}
