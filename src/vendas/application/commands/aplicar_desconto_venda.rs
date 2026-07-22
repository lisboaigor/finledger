use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

/// Desconto global sobre a venda EmAndamento — usado pela conversão
/// orçamento→venda (herda o desconto do orçamento aceito) e disponível na API.
#[external_fields]
#[derive(Command, Deserialize)]
pub struct AplicarDescontoVenda {
    #[external]
    pub venda_id: Uuid,
    pub desconto_centavos: i64,
}

impl CommandHandler<AplicarDescontoVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AplicarDescontoVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.aplicar_desconto(cmd.desconto_centavos)?;
        self.salvar(&mut venda).await
    }
}
