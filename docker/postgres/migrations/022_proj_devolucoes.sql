-- 022 — Read-model de devoluções datadas.
-- O evento ItensDevolvidos carrega a DATA da devolução (occurred_at), mas a
-- projeção de vendas só reduzia proj_vendas_itens — a data se perdia. Sem ela,
-- o BI reduzia a venda na data ORIGINAL, mudando meses já fechados
-- retroativamente. Este read-model persiste cada devolução com sua data, para
-- o BI lançar um fato NEGATIVO datado no período em que a devolução ocorreu
-- (mês original estável) em vez de reescrever a venda.
--
-- Idempotente: reaplicável (psql -U postgres -d finledger -f ...).

CREATE TABLE IF NOT EXISTS proj_devolucoes (
    tenant_id        UUID        NOT NULL,
    venda_id         UUID        NOT NULL,
    item_id          UUID        NOT NULL,
    produto_id       UUID        NOT NULL,
    quantidade       INTEGER     NOT NULL,
    receita_centavos BIGINT      NOT NULL,
    devolvida_em     TIMESTAMPTZ NOT NULL,
    -- Uma devolução é única por (venda, item, instante) — reentrega do mesmo
    -- evento não duplica.
    PRIMARY KEY (tenant_id, venda_id, item_id, devolvida_em)
);

CREATE INDEX IF NOT EXISTS idx_proj_devolucoes_produto
    ON proj_devolucoes (tenant_id, produto_id);

ALTER TABLE proj_devolucoes ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_devolucoes FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rls_proj_devolucoes ON proj_devolucoes;
CREATE POLICY rls_proj_devolucoes ON proj_devolucoes
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
