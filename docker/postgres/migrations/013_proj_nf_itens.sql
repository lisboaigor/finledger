-- 013 — Projeção de impostos POR ITEM da nota fiscal (reforma LC 214/2025).
-- proj_notas_fiscais só tem os totais por NF; o BI precisa da margem líquida
-- por item de venda, então materializamos o breakdown por item a partir do
-- evento NotaFiscalGerada (cada ItemNF já carrega seu ImpostoItem).
--
-- `ibs_cbs_informativo` é congelado na emissão (perfil vigente): no Simples
-- Nacional sem regime regular o IBS/CBS destacado é recolhido via DAS e NÃO é
-- custo por fora — o ETL do BI usa este flag para decidir se soma IBS/CBS.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

CREATE TABLE IF NOT EXISTS proj_nf_itens (
    tenant_id           UUID   NOT NULL,
    nf_id               UUID   NOT NULL,
    venda_id            UUID   NOT NULL,
    produto_id          UUID   NOT NULL,
    quantidade          INTEGER NOT NULL,
    total_centavos      BIGINT NOT NULL DEFAULT 0,
    icms_centavos       BIGINT NOT NULL DEFAULT 0,
    iss_centavos        BIGINT NOT NULL DEFAULT 0,
    pis_centavos        BIGINT NOT NULL DEFAULT 0,
    cofins_centavos     BIGINT NOT NULL DEFAULT 0,
    cbs_centavos        BIGINT NOT NULL DEFAULT 0,
    ibs_uf_centavos     BIGINT NOT NULL DEFAULT 0,
    ibs_mun_centavos    BIGINT NOT NULL DEFAULT 0,
    is_centavos         BIGINT NOT NULL DEFAULT 0,
    ibs_cbs_informativo BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (tenant_id, nf_id, produto_id)
);

CREATE INDEX IF NOT EXISTS idx_proj_nf_itens_venda
    ON proj_nf_itens (tenant_id, venda_id);

ALTER TABLE proj_nf_itens ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_nf_itens FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_proj_nf_itens ON proj_nf_itens;
CREATE POLICY rls_proj_nf_itens ON proj_nf_itens
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
