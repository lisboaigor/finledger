use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_core::DomainEvent;
use pharos_postgres::Pool;

use crate::shared::tenant::current_tenant_id;

/// Outbox analítico: grava (tipo, aggregate_id, ocorrido_em) de todo evento de
/// domínio em `bi.eventos_outbox`, dando ao ETL os timestamps exatos de
/// transição (data de decisão de orçamento, liquidação de conta etc.) que as
/// projeções não preservam. Um único handler genérico atende qualquer enum de
/// evento — o trait `DomainEvent` já expõe os três campos necessários.
#[derive(Clone)]
pub struct BiOutboxHandler {
    pool: Pool,
}

impl BiOutboxHandler {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

impl<E: DomainEvent> EventHandler<E> for BiOutboxHandler {
    type Error = Infallible;

    async fn handle(&self, event: &E) -> Result<(), Infallible> {
        // Sem tenant em escopo (login, backoffice) não há o que registrar.
        let Ok(tenant_id) = current_tenant_id() else {
            return Ok(());
        };
        if let Err(e) = sqlx::query(
            "INSERT INTO bi.eventos_outbox (tenant_id, tipo_evento, aggregate_id, ocorrido_em)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(tenant_id)
        .bind(event.event_type())
        .bind(event.aggregate_id())
        .bind(event.occurred_at())
        .execute(&self.pool)
        .await
        {
            tracing::warn!(error = %e, "outbox do BI falhou (schema bi aplicado?)");
        }
        Ok(())
    }
}
