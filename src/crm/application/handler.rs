use std::sync::Arc;

use pharos_app::EventBus;

use crate::crm::domain::cliente::{Cliente, ClienteId};
use crate::crm::infrastructure::repository::PostgresClienteRepository;
use crate::error::AppError;
use crate::shared::{load_aggregate, salvar_aggregate};

pub struct CrmHandlers {
    pub(crate) repo: Arc<PostgresClienteRepository>,
    pub(crate) bus: EventBus,
}

impl CrmHandlers {
    pub fn new(repo: Arc<PostgresClienteRepository>, bus: EventBus) -> Self {
        Self { repo, bus }
    }

    pub(crate) async fn load(&self, id: ClienteId) -> Result<Cliente, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, cliente: &mut Cliente) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, cliente).await
    }
}
