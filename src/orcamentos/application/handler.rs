use std::sync::Arc;

use pharos_app::EventBus;
use pharos_postgres::Pool;

use crate::error::AppError;
use crate::orcamentos::domain::orcamento::{Orcamento, OrcamentoId};
use crate::orcamentos::infrastructure::repository::PostgresOrcamentoRepository;
use crate::shared::{load_aggregate, salvar_aggregate};
use crate::tenants::repository::TenantRepository;

pub struct OrcamentosHandlers {
    pub(crate) repo: Arc<PostgresOrcamentoRepository>,
    pub(crate) bus: EventBus,
    pub(crate) pool: Pool,
    pub(crate) tenants: Arc<TenantRepository>,
}

impl OrcamentosHandlers {
    pub fn new(
        repo: Arc<PostgresOrcamentoRepository>,
        bus: EventBus,
        pool: Pool,
        tenants: Arc<TenantRepository>,
    ) -> Self {
        Self {
            repo,
            bus,
            pool,
            tenants,
        }
    }

    pub(crate) async fn load(&self, id: OrcamentoId) -> Result<Orcamento, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, orcamento: &mut Orcamento) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, orcamento).await
    }

    /// Restaura um orçamento da lixeira. Mexe só na visibilidade da projeção —
    /// não é comando de domínio (o agregado não muda).
    pub async fn restaurar_arquivado(&self, orcamento_id: uuid::Uuid) -> Result<(), AppError> {
        self.repo.restaurar(orcamento_id).await
    }
}
