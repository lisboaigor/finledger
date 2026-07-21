use std::sync::Arc;

use pharos_app::EventBus;
use pharos_core::Repository;

use crate::error::AppError;
use crate::estoque::domain::item_estoque::{ItemEstoque, ItemEstoqueId};
use crate::estoque::infrastructure::repository::PostgresEstoqueRepository;
use crate::shared::salvar_aggregate;

pub struct EstoqueHandlers {
    pub(crate) repo: Arc<PostgresEstoqueRepository>,
    pub(crate) bus: EventBus,
}

impl EstoqueHandlers {
    pub fn new(repo: Arc<PostgresEstoqueRepository>, bus: EventBus) -> Self {
        Self { repo, bus }
    }

    pub async fn load_ou_criar(&self, produto_id: uuid::Uuid) -> Result<ItemEstoque, AppError> {
        let id = ItemEstoqueId::from_uuid(produto_id);
        match self.repo.find_by_id(&id).await.map_err(AppError::infra)? {
            Some(item) => Ok(item),
            None => Ok(ItemEstoque::criar(produto_id, 0)),
        }
    }

    pub(crate) async fn salvar(&self, item: &mut ItemEstoque) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, item).await
    }
}
