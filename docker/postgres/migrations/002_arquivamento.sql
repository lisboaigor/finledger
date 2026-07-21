-- 002 — Arquivamento ("lixeira") de vendas e orçamentos não concretizados.
-- Nada é excluído: após N dias (tenants.arquivamento_dias, NULL = desligado),
-- vendas abandonadas/canceladas e orçamentos não convertidos ganham carimbo de
-- arquivamento e somem das listagens padrão. O gestor vê e restaura pela
-- lixeira; restaurados (restaurad?_em) não voltam a ser arquivados pelo job.
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS arquivamento_dias INTEGER
        CHECK (arquivamento_dias IS NULL OR arquivamento_dias >= 1);

ALTER TABLE proj_vendas
    ADD COLUMN IF NOT EXISTS arquivada_em   TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS restaurada_em  TIMESTAMPTZ;

ALTER TABLE proj_orcamentos
    ADD COLUMN IF NOT EXISTS arquivado_em   TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS restaurado_em  TIMESTAMPTZ;

-- SECURITY DEFINER (dono postgres): o job do backend roda fora de escopo de
-- tenant e varre todos — mesmo padrão do ETL do BI.
CREATE OR REPLACE FUNCTION executar_arquivamento() RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public AS $$
DECLARE v BIGINT; o BIGINT;
BEGIN
    -- Vendas nunca concretizadas (iniciadas e abandonadas) ou canceladas.
    UPDATE proj_vendas pv
       SET arquivada_em = NOW()
      FROM tenants t
     WHERE t.tenant_id = pv.tenant_id
       AND t.arquivamento_dias IS NOT NULL
       AND pv.arquivada_em IS NULL
       AND pv.restaurada_em IS NULL
       AND pv.status IN ('iniciada', 'cancelada')
       AND pv.atualizado_em < NOW() - make_interval(days => t.arquivamento_dias);
    GET DIAGNOSTICS v = ROW_COUNT;

    -- Orçamentos que não viraram venda (rascunhos velhos, recusados,
    -- expirados, cancelados). Emitidos ficam de fora: ainda podem converter.
    UPDATE proj_orcamentos po
       SET arquivado_em = NOW()
      FROM tenants t
     WHERE t.tenant_id = po.tenant_id
       AND t.arquivamento_dias IS NOT NULL
       AND po.arquivado_em IS NULL
       AND po.restaurado_em IS NULL
       AND po.status IN ('rascunho', 'recusado', 'expirado', 'cancelado')
       AND po.atualizado_em < NOW() - make_interval(days => t.arquivamento_dias);
    GET DIAGNOSTICS o = ROW_COUNT;

    RETURN jsonb_build_object('vendas', v, 'orcamentos', o);
END $$;
