-- 019_pharos_outbox.sql — Outbox transacional + inbox de idempotência (issue #3).
--
-- Tabelas de INFRA do pharos (não são read-model nem tenant-scoped): o
-- `save_and_enqueue_in` grava aqui o evento na MESMA transação do snapshot do
-- agregado, e o relay (src/bootstrap/outbox_relay.rs) despacha depois. NÃO levam
-- RLS de propósito — o relay lê o conjunto pendente de todos os tenants e escopa
-- cada mensagem pelo header `tenant_id`. O papel `finledger` recebe privilégio
-- via ALTER DEFAULT PRIVILEGES do init.sql (migração roda como postgres).
--
-- DDL idêntica à de pharos-postgres/src/eventing.rs (POSTGRES_EVENTING_SCHEMA),
-- mantida em migração para os testes (tests/helpers.rs aplica migrations/*.sql)
-- e o initdb a materializarem junto do restante do schema. Idempotente.

CREATE TABLE IF NOT EXISTS pharos_outbox (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL,
    topic TEXT NOT NULL,
    message_key TEXT NULL,
    headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    payload BYTEA NOT NULL,
    content_type TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'published', 'failed', 'dead_lettered')),
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT NULL
);
ALTER TABLE pharos_outbox
    ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now();
CREATE INDEX IF NOT EXISTS idx_pharos_outbox_pending_created_at
    ON pharos_outbox (created_at)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_pharos_outbox_pending_next_attempt_at
    ON pharos_outbox (next_attempt_at)
    WHERE status = 'pending';

CREATE TABLE IF NOT EXISTS pharos_inbox (
    message_id UUID NOT NULL,
    consumer TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('processing', 'completed', 'failed')),
    received_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    last_error TEXT NULL,
    PRIMARY KEY (message_id, consumer)
);
CREATE INDEX IF NOT EXISTS idx_pharos_inbox_status_updated_at
    ON pharos_inbox (status, updated_at);
