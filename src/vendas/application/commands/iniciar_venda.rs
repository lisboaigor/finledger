use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::{Venda, VendaId};

#[external_fields]
#[derive(Command, Deserialize)]
pub struct IniciarVenda {
    #[external]
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
}

impl CommandHandler<IniciarVenda> for VendasHandlers {
    type Output = VendaId;
    type Error = AppError;

    async fn handle(&self, cmd: IniciarVenda) -> Result<VendaId, AppError> {
        let mut venda = Venda::iniciar(cmd.vendedor_id, cmd.cliente_id);
        let id = *venda.id();
        self.salvar(&mut venda).await?;
        Ok(id)
    }
}
