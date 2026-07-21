use pharos_app::EventBus;
use pharos_postgres::Pool;

use super::handlers::Handlers;
use crate::bi::outbox::BiOutboxHandler;
use crate::estoque::application::event_handlers::{
    EstoqueComprasEventHandler, EstoqueDevolucaoEventHandler, EstoqueVendaEventHandler,
};
use crate::financeiro::application::event_handlers::{
    FinanceiroComprasEventHandler, FinanceiroVendaEventHandler,
};
use crate::fiscal::application::event_handlers::FiscalVendaEventHandler;
use crate::vendas::application::event_handlers::VendaAPartirDeOrcamentoHandler;

pub fn register(bus: &EventBus, handlers: &Handlers, pool: Pool) {
    bus.register(FinanceiroVendaEventHandler {
        financeiro: handlers.financeiro.clone(),
    });

    bus.register(FinanceiroComprasEventHandler {
        financeiro: handlers.financeiro.clone(),
    });

    bus.register(EstoqueComprasEventHandler {
        estoque: handlers.estoque.clone(),
    });

    bus.register(EstoqueVendaEventHandler {
        estoque: handlers.estoque.clone(),
    });

    // Devolução: reentra itens devolvidos ao custo médio atual do produto.
    bus.register(EstoqueDevolucaoEventHandler {
        estoque: handlers.estoque.clone(),
        pool: pool.clone(),
    });

    bus.register(FiscalVendaEventHandler {
        fiscal: handlers.fiscal.clone(),
    });

    // Aceitar um orçamento gera uma venda EmAndamento (finalizada no PDV) e
    // marca o orçamento como convertido.
    bus.register(VendaAPartirDeOrcamentoHandler {
        vendas: handlers.vendas.clone(),
        orcamentos: handlers.orcamentos.clone(),
    });

    // Outbox analítico do BI: um handler genérico por enum de evento com
    // transições que interessam ao warehouse (ver src/bi/outbox.rs).
    let outbox = BiOutboxHandler::new(pool);
    bus.register::<crate::vendas::domain::events::VendaEvent, _>(outbox.clone());
    bus.register::<crate::orcamentos::domain::events::OrcamentoEvent, _>(outbox.clone());
    bus.register::<crate::compras::domain::events::ComprasEvent, _>(outbox.clone());
    bus.register::<crate::financeiro::domain::events::FinanceiroEvent, _>(outbox.clone());
    bus.register::<crate::fiscal::domain::events::NotaFiscalEvent, _>(outbox);
}
