use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AtualizarVenda {
    #[external]
    pub venda_id: Uuid,
    pub cliente_id: Option<Uuid>,
}

impl CommandHandler<AtualizarVenda> for VendasHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: AtualizarVenda) -> Result<(), AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;
        venda.atualizar(cmd.cliente_id)?;
        self.salvar(&mut venda).await
    }
}
