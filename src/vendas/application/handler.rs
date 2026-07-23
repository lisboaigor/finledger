use std::sync::Arc;

use pharos_app::EventBus;
use pharos_postgres::Pool;

use crate::error::AppError;
use crate::shared::{load_aggregate, salvar_aggregate_duravel};
use crate::vendas::domain::venda::{Venda, VendaId};
use crate::vendas::infrastructure::repository::PostgresVendaRepository;

pub struct VendasHandlers {
    pub(crate) repo: Arc<PostgresVendaRepository>,
    pub(crate) bus: EventBus,
    pub(crate) pool: Pool,
}

impl VendasHandlers {
    pub fn new(repo: Arc<PostgresVendaRepository>, bus: EventBus, pool: Pool) -> Self {
        Self { repo, bus, pool }
    }

    pub(crate) async fn load(&self, id: VendaId) -> Result<Venda, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, venda: &mut Venda) -> Result<(), AppError> {
        salvar_aggregate_duravel(&self.pool, &*self.repo, &self.bus, venda, "VendaEvent").await
    }

    /// Restaura uma venda da lixeira. Mexe só na visibilidade da projeção —
    /// não é comando de domínio (o agregado não muda).
    pub async fn restaurar_arquivada(&self, venda_id: uuid::Uuid) -> Result<(), AppError> {
        self.repo.restaurar(venda_id).await
    }
}
