use std::sync::Arc;

use pharos_app::EventBus;

use crate::error::AppError;
use crate::fornecedores::domain::fornecedor::{Fornecedor, FornecedorId};
use crate::fornecedores::infrastructure::repository::PostgresFornecedorRepository;
use crate::shared::{load_aggregate, salvar_aggregate};

pub struct FornecedoresHandlers {
    pub(crate) repo: Arc<PostgresFornecedorRepository>,
    pub(crate) bus: EventBus,
}

impl FornecedoresHandlers {
    pub fn new(repo: Arc<PostgresFornecedorRepository>, bus: EventBus) -> Self {
        Self { repo, bus }
    }

    pub(crate) async fn load(&self, id: FornecedorId) -> Result<Fornecedor, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, fornecedor: &mut Fornecedor) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, fornecedor).await
    }
}
