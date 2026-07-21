-- 010 — Overrides de alíquota por tenant. Mesma forma de ref_aliquotas +
-- tenant_id + RLS: um tenant com benefício fiscal próprio (ex.: regime especial
-- estadual) sobrepõe a referência global sem afetar os demais. O provider
-- consulta o override primeiro e cai para ref_aliquotas.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

CREATE TABLE IF NOT EXISTS aliquotas_tenant (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id        UUID NOT NULL REFERENCES tenants(tenant_id),
    tributo          VARCHAR(10) NOT NULL CHECK (tributo IN
        ('icms', 'iss', 'pis', 'cofins', 'cbs', 'ibs_uf', 'ibs_mun', 'is')),
    uf               CHAR(2),
    codigo_municipio VARCHAR(7),
    regime           VARCHAR(20),
    c_class_trib     VARCHAR(6),
    ncm_prefixo      VARCHAR(8),
    aliquota_bps     INTEGER NOT NULL CHECK (aliquota_bps >= 0),
    vigencia_inicio  DATE NOT NULL,
    vigencia_fim     DATE,
    UNIQUE NULLS NOT DISTINCT (tenant_id, tributo, uf, codigo_municipio, regime,
                               c_class_trib, ncm_prefixo, vigencia_inicio)
);

CREATE INDEX IF NOT EXISTS idx_aliquotas_tenant_lookup
    ON aliquotas_tenant (tenant_id, tributo, vigencia_inicio, vigencia_fim);

ALTER TABLE aliquotas_tenant ENABLE ROW LEVEL SECURITY;
ALTER TABLE aliquotas_tenant FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_aliquotas_tenant ON aliquotas_tenant;
CREATE POLICY rls_aliquotas_tenant ON aliquotas_tenant
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
