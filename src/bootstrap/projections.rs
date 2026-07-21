use pharos_app::EventBus;
use pharos_postgres::Pool;

use crate::projections::{
    catalogo::CatalogoProjection, compras::ComprasProjection, crm::CrmProjection,
    estoque::EstoqueProjection, financeiro::FinanceiroProjection, fiscal::FiscalProjection,
    fornecedores::FornecedoresProjection, identity::IdentityProjection,
    orcamentos::OrcamentosProjection, vendas::VendasProjection,
};

pub fn register(bus: &EventBus, pool: Pool) {
    bus.register(CatalogoProjection::new(pool.clone()));
    bus.register(CrmProjection::new(pool.clone()));
    bus.register(FornecedoresProjection::new(pool.clone()));
    bus.register(EstoqueProjection::new(pool.clone()));
    bus.register(VendasProjection::new(pool.clone()));
    bus.register(OrcamentosProjection::new(pool.clone()));
    bus.register(ComprasProjection::new(pool.clone()));
    bus.register(FinanceiroProjection::new(pool.clone()));
    bus.register(FiscalProjection::new(pool.clone()));
    bus.register(IdentityProjection::new(pool));
}
