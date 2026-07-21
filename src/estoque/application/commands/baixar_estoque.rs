use pharos_app::CommandHandler;
use pharos_core::Repository;
use pharos_macros::Command;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::handler::EstoqueHandlers;
use crate::estoque::domain::item_estoque::ItemEstoqueId;

/// Baixa de estoque disparada pela confirmação de venda.
#[derive(Command, Deserialize)]
pub struct BaixarEstoque {
    pub produto_id: Uuid,
    pub quantidade: u32,
    pub referencia_id: Option<String>,
}

impl CommandHandler<BaixarEstoque> for EstoqueHandlers {
    type Output = ();
    type Error = AppError;

    async fn handle(&self, cmd: BaixarEstoque) -> Result<(), AppError> {
        let id = ItemEstoqueId::from_uuid(cmd.produto_id);
        let mut item = self
            .repo
            .find_by_id(&id)
            .await
            .map_err(AppError::infra)?
            .ok_or_else(|| {
                AppError::Domain(pharos_core::DomainError::NotFound(format!(
                    "Estoque do produto {} não encontrado",
                    cmd.produto_id
                )))
            })?;
        item.baixar(cmd.quantidade, "venda".to_string(), cmd.referencia_id)?;
        self.salvar(&mut item).await
    }
}
