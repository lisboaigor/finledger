-- 017 — Numeração sequencial de NF por tenant e série (issue #16, parte de
-- numeração). Substitui o número derivado de UUID (uuid % 10⁹) por uma
-- sequência atômica por (tenant, série):
--   INSERT ... ON CONFLICT DO UPDATE SET proximo = proximo + 1 RETURNING proximo
-- Série fixa 1 por enquanto; CFOP/ICMS-ST seguem em aberto na issue #16.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

CREATE TABLE IF NOT EXISTS fiscal_numeracao (
    tenant_id UUID    NOT NULL,
    serie     INTEGER NOT NULL,
    proximo   BIGINT  NOT NULL DEFAULT 1,
    PRIMARY KEY (tenant_id, serie)
);

ALTER TABLE fiscal_numeracao ENABLE ROW LEVEL SECURITY;
ALTER TABLE fiscal_numeracao FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_fiscal_numeracao ON fiscal_numeracao;
CREATE POLICY rls_fiscal_numeracao ON fiscal_numeracao
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
