//! Relay do outbox transacional (issue #3).
//!
//! Task de fundo que drena `pharos_outbox` e despacha cada evento de volta no
//! `EventBus` in-process (projeções + handlers cross-context), reconstruindo o
//! escopo de tenant a partir do header da mensagem. É a ÚNICA fonte de despacho
//! dos contextos produtores (vendas/orçamentos/compras): o request grava o
//! agregado + os eventos numa transação e retorna; os efeitos (conta a receber,
//! NF, baixa de estoque, projeções) saem aqui, logo depois — à prova de crash,
//! porque o evento é durável antes de qualquer efeito rodar.
//!
//! Trajetória microserviços: trocar o `RelayPublisher` in-process por um
//! publisher Kafka/NATS e separar o deploy não toca handler nem contrato — as
//! mensagens já saem com tópico por tipo de evento, `key = aggregate_id` e
//! header `tenant_id`.

use std::sync::Arc;
use std::time::Duration;

use pharos_app::{
    CURRENT_TENANT, EventBus, IdempotencyDecision, InboxStore, Message, MessagePublisher,
    MessagingError, OutboxDispatcher, TenantContext,
};
use pharos_postgres::{Pool, PostgresInboxStore, PostgresOutboxRepository};
use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

use crate::compras::domain::events::ComprasEvent;
use crate::orcamentos::domain::events::OrcamentoEvent;
use crate::vendas::domain::events::VendaEvent;

/// Nome do consumidor no inbox de idempotência. Um só relay in-process por ora;
/// num split por contexto cada serviço teria seu próprio consumidor.
const CONSUMER: &str = "in-process-relay";
const INTERVALO_PADRAO_MS: u64 = 250;
const LOTE: usize = 100;

/// Registra os decoders dos eventos que trafegam pelo outbox. O `topic` DEVE
/// casar com o passado a `salvar_aggregate_duravel` (constante por enum).
/// Chamado no bootstrap, junto do registro dos handlers.
pub fn registrar_decoders(bus: &EventBus) {
    bus.register_decoder::<VendaEvent>("VendaEvent");
    bus.register_decoder::<OrcamentoEvent>("OrcamentoEvent");
    bus.register_decoder::<ComprasEvent>("ComprasEvent");
}

/// Publisher in-process: decodifica a mensagem de outbox de volta ao evento
/// tipado e a despacha no `EventBus`, no escopo de tenant do header e com
/// deduplicação via inbox.
pub struct RelayPublisher {
    bus: EventBus,
    inbox: PostgresInboxStore,
}

impl RelayPublisher {
    pub fn new(bus: EventBus, pool: Pool) -> Self {
        Self {
            bus,
            inbox: PostgresInboxStore::new(pool),
        }
    }

    async fn despachar(&self, message: &Message) -> Result<(), MessagingError> {
        // Idempotência de consumidor: se já concluído, pula o dispatch (o
        // dispatcher marca a linha do outbox como publicada mesmo assim).
        // Handlers seguem idempotentes (#7) como segunda linha.
        if let IdempotencyDecision::AlreadyCompleted = self
            .inbox
            .begin_processing(message.message_id, CONSUMER)
            .await
            .map_err(MessagingError::publish)?
        {
            return Ok(());
        }

        match self.bus.publish_erased(&message.topic, &message.payload).await {
            Ok(()) => {
                self.inbox
                    .mark_completed(message.message_id, CONSUMER)
                    .await
                    .map_err(MessagingError::publish)?;
                Ok(())
            }
            Err(e) => {
                // Falha real: registra e propaga para o dispatcher aplicar
                // retry/backoff/dead-letter (a linha segue pendente no outbox).
                let _ = self
                    .inbox
                    .mark_failed(message.message_id, CONSUMER, e.to_string())
                    .await;
                Err(MessagingError::publish(e))
            }
        }
    }
}

impl MessagePublisher for RelayPublisher {
    async fn publish(&self, message: Message) -> Result<(), MessagingError> {
        // O relay roda fora de qualquer request: reidrata o CURRENT_TENANT a
        // partir do header para repositórios, projeções e RLS operarem no
        // tenant certo (mesmo mecanismo do `require_auth`/`in_tenant`).
        let tenant = message
            .headers
            .get("tenant_id")
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(TenantContext::new);

        match tenant {
            Some(ctx) => {
                CURRENT_TENANT
                    .scope(Some(ctx), self.despachar(&message))
                    .await
            }
            None => self.despachar(&message).await,
        }
    }
}

/// Agenda o relay: drena o outbox a cada `OUTBOX_RELAY_INTERVAL_MS` (default
/// 250ms) e imediatamente a cada "kick" pós-commit (`Notify`), mantendo a
/// leitura pós-escrita na casa dos milissegundos. `OUTBOX_RELAY_ATIVO=false`
/// desliga o relay (par do fallback síncrono em `salvar_aggregate_duravel`).
pub fn spawn(pool: Pool, bus: EventBus, kick: Arc<Notify>) {
    let ativo = std::env::var("OUTBOX_RELAY_ATIVO")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    if !ativo {
        tracing::info!("relay do outbox desativado (OUTBOX_RELAY_ATIVO=false)");
        return;
    }

    let intervalo = std::env::var("OUTBOX_RELAY_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(INTERVALO_PADRAO_MS);

    let dispatcher = OutboxDispatcher::new(
        PostgresOutboxRepository::new(pool.clone()),
        RelayPublisher::new(bus, pool),
    );

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(intervalo));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = tick.tick() => {}
                _ = kick.notified() => {}
            }
            // Drena até o lote esvaziar: um dispatch pode gerar novos eventos
            // duráveis (ex.: orçamento aceito → venda), que este loop recolhe
            // sem esperar o próximo tick. Para no primeiro lote sem publicações
            // (0 publicados = nada novo ou só falhas já reagendadas) para não
            // girar em falso.
            loop {
                let r = dispatcher.dispatch_pending(LOTE).await;
                if !r.errors.is_empty() {
                    tracing::error!(
                        publicados = r.published,
                        erros = r.errors.len(),
                        "relay do outbox: falhas no lote (reagendadas para retry/dead-letter)"
                    );
                }
                if r.published == 0 {
                    break;
                }
            }
        }
    });
}
