-- 001 — Custos fixos discriminados (item a item) na precificação assistida.
-- O tenant cadastra cada custo (aluguel, salário, DAS…) e o backend mantém a
-- soma em tenants.custos_fixos_mensais_centavos, que continua sendo o campo
-- lido pelo cálculo de sugestão de preço.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

CREATE TABLE IF NOT EXISTS custos_fixos (
    tenant_id      UUID   NOT NULL REFERENCES tenants(tenant_id),
    nome           TEXT   NOT NULL,
    valor_centavos BIGINT NOT NULL CHECK (valor_centavos >= 0),
    PRIMARY KEY (tenant_id, nome)
);

ALTER TABLE custos_fixos ENABLE ROW LEVEL SECURITY;
ALTER TABLE custos_fixos FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_custos_fixos ON custos_fixos;
CREATE POLICY rls_custos_fixos ON custos_fixos
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
