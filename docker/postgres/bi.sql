-- ── Módulo de BI prescritivo ──────────────────────────────────────────────────
-- Warehouse dimensional (schema `bi`) + ETL incremental + motor de alertas.
--
-- Idempotente: pode ser (re)aplicado a qualquer momento com um superusuário:
--   psql -U postgres -d finledger -f bi.sql
-- Em instalações novas roda no initdb (montado como zz-bi.sql, após init.sql).
--
-- Fonte dos fatos são as projeções `proj_*` (pharos_tenant_aggregates guarda só
-- o último snapshot por agregado — não é um event log; ver docs/plano de BI).
-- As funções de ETL/alertas são SECURITY DEFINER (donas = superusuário) porque
-- processam todos os tenants de uma vez; a leitura pelo app continua sob RLS.

\c finledger

CREATE SCHEMA IF NOT EXISTS bi;

-- ── Dimensões ─────────────────────────────────────────────────────────────────

-- Calendário compartilhado (sem tenant_id / sem RLS).
CREATE TABLE IF NOT EXISTS bi.dim_tempo (
    data      DATE PRIMARY KEY,
    ano       INTEGER  NOT NULL,
    mes       INTEGER  NOT NULL,
    dia       INTEGER  NOT NULL,
    trimestre INTEGER  NOT NULL,
    semana    INTEGER  NOT NULL,
    dia_semana INTEGER NOT NULL, -- 0=domingo … 6=sábado
    dia_util  BOOLEAN  NOT NULL
);

INSERT INTO bi.dim_tempo (data, ano, mes, dia, trimestre, semana, dia_semana, dia_util)
SELECT d::date,
       EXTRACT(year    FROM d)::int,
       EXTRACT(month   FROM d)::int,
       EXTRACT(day     FROM d)::int,
       EXTRACT(quarter FROM d)::int,
       EXTRACT(week    FROM d)::int,
       EXTRACT(dow     FROM d)::int,
       EXTRACT(dow     FROM d)::int NOT IN (0, 6)
FROM generate_series('2020-01-01'::date, '2035-12-31'::date, '1 day') AS d
ON CONFLICT (data) DO NOTHING;

-- Produto com histórico SCD2: margem histórica usa o custo vigente na data da
-- venda (o item de venda não congela custo — gap de instrumentação conhecido).
CREATE TABLE IF NOT EXISTS bi.dim_produto (
    sk          BIGSERIAL   PRIMARY KEY,
    tenant_id   UUID        NOT NULL,
    produto_id  UUID        NOT NULL,
    sku         VARCHAR(50) NOT NULL,
    descricao   TEXT        NOT NULL,
    ncm         VARCHAR(10) NOT NULL,
    categoria   TEXT        NOT NULL,
    marca       TEXT,
    preco_custo BIGINT      NOT NULL,
    preco_venda BIGINT      NOT NULL,
    ativo       BOOLEAN     NOT NULL,
    valido_de   TIMESTAMPTZ NOT NULL,
    valido_ate  TIMESTAMPTZ,
    atual       BOOLEAN     NOT NULL DEFAULT TRUE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_bi_dim_produto_atual
    ON bi.dim_produto (tenant_id, produto_id) WHERE atual;

-- ── Fatos ─────────────────────────────────────────────────────────────────────

-- Grão: 1 linha por item de venda confirmada/cancelada. Custo congelado na carga.
CREATE TABLE IF NOT EXISTS bi.fato_vendas_item (
    tenant_id               UUID        NOT NULL,
    item_id                 UUID        NOT NULL,
    venda_id                UUID        NOT NULL,
    produto_sk              BIGINT,
    produto_id              UUID        NOT NULL,
    cliente_id              UUID,
    vendedor_id             UUID        NOT NULL,
    forma_pagamento         TEXT,
    status                  VARCHAR(16) NOT NULL,
    data_venda              DATE        NOT NULL,
    quantidade              INTEGER     NOT NULL,
    receita_centavos        BIGINT      NOT NULL,
    custo_unitario_centavos BIGINT      NOT NULL,
    custo_centavos          BIGINT      NOT NULL,
    margem_centavos         BIGINT      NOT NULL,
    PRIMARY KEY (tenant_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_bi_fvi_data ON bi.fato_vendas_item (tenant_id, data_venda);

-- Grão: 1 linha por orçamento (accumulating snapshot simplificado; data_decisao
-- é proxy de atualizado_em até o outbox de eventos existir).
CREATE TABLE IF NOT EXISTS bi.fato_orcamentos (
    tenant_id         UUID        NOT NULL,
    orcamento_id      UUID        NOT NULL,
    vendedor_id       UUID        NOT NULL,
    cliente_id        UUID,
    total_centavos    BIGINT      NOT NULL,
    desconto_centavos BIGINT      NOT NULL,
    status            VARCHAR(16) NOT NULL,
    validade_dias     INTEGER     NOT NULL,
    venda_id          UUID,
    criado_em         TIMESTAMPTZ NOT NULL,
    data_decisao      TIMESTAMPTZ,
    PRIMARY KEY (tenant_id, orcamento_id)
);

-- Grão: 1 linha por conta (accumulating snapshot; data_liquidacao é proxy).
CREATE TABLE IF NOT EXISTS bi.fato_contas_receber (
    tenant_id       UUID        NOT NULL,
    conta_id        UUID        NOT NULL,
    venda_id        UUID        NOT NULL,
    cliente_id      UUID,
    valor_original  BIGINT      NOT NULL,
    valor_recebido  BIGINT      NOT NULL,
    status          VARCHAR(16) NOT NULL,
    criada_em       TIMESTAMPTZ NOT NULL,
    vencimento      TIMESTAMPTZ NOT NULL,
    data_liquidacao TIMESTAMPTZ,
    PRIMARY KEY (tenant_id, conta_id)
);

CREATE TABLE IF NOT EXISTS bi.fato_contas_pagar (
    tenant_id       UUID        NOT NULL,
    conta_id        UUID        NOT NULL,
    pedido_id       UUID        NOT NULL,
    fornecedor_id   UUID        NOT NULL,
    valor_original  BIGINT      NOT NULL,
    valor_pago      BIGINT      NOT NULL,
    status          VARCHAR(16) NOT NULL,
    criada_em       TIMESTAMPTZ NOT NULL,
    vencimento      TIMESTAMPTZ NOT NULL,
    data_liquidacao TIMESTAMPTZ,
    PRIMARY KEY (tenant_id, conta_id)
);

-- Grão: 1 linha por produto por dia (periodic snapshot — única fonte possível
-- de histórico de estoque; começa a existir a partir da primeira execução).
CREATE TABLE IF NOT EXISTS bi.fato_estoque_snapshot (
    tenant_id      UUID    NOT NULL,
    produto_id     UUID    NOT NULL,
    data           DATE    NOT NULL,
    quantidade     INTEGER NOT NULL,
    custo_medio    BIGINT  NOT NULL,
    valor_centavos BIGINT  NOT NULL,
    estoque_minimo INTEGER NOT NULL,
    em_ruptura     BOOLEAN NOT NULL,
    PRIMARY KEY (tenant_id, produto_id, data)
);

-- ── Análises derivadas (recalculadas a cada ciclo) ────────────────────────────

-- Curva ABC (Pareto por receita 12m), classe XYZ (variabilidade da demanda
-- semanal, 26 semanas), cobertura e estoque morto — grão tenant/produto.
CREATE TABLE IF NOT EXISTS bi.analise_produtos (
    tenant_id         UUID        NOT NULL,
    produto_id        UUID        NOT NULL,
    sku               VARCHAR(50) NOT NULL,
    descricao         TEXT        NOT NULL,
    categoria         TEXT        NOT NULL,
    receita_12m       BIGINT      NOT NULL DEFAULT 0,
    margem_12m        BIGINT      NOT NULL DEFAULT 0,
    qtd_12m           BIGINT      NOT NULL DEFAULT 0,
    demanda_dia_90d   NUMERIC     NOT NULL DEFAULT 0,
    cobertura_dias    INTEGER,             -- NULL = sem demanda
    classe_abc        CHAR(1)     NOT NULL DEFAULT 'C',
    classe_xyz        CHAR(1),             -- NULL = sem demanda no período
    dias_sem_venda    INTEGER,             -- NULL = nunca vendeu
    quantidade        INTEGER     NOT NULL DEFAULT 0,
    estoque_minimo    INTEGER     NOT NULL DEFAULT 0,
    valor_imobilizado BIGINT      NOT NULL DEFAULT 0,
    atualizado_em     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, produto_id)
);

-- RFM (recência/frequência/valor, quintis dentro do tenant) + inadimplência —
-- grão tenant/cliente (apenas clientes com compra identificada em 12m).
CREATE TABLE IF NOT EXISTS bi.analise_clientes (
    tenant_id       UUID        NOT NULL,
    cliente_id      UUID        NOT NULL,
    nome            TEXT        NOT NULL,
    recencia_dias   INTEGER     NOT NULL,
    frequencia_12m  INTEGER     NOT NULL,
    valor_12m       BIGINT      NOT NULL,
    score_r         SMALLINT    NOT NULL,
    score_f         SMALLINT    NOT NULL,
    score_m         SMALLINT    NOT NULL,
    segmento        TEXT        NOT NULL,
    saldo_vencido   BIGINT      NOT NULL DEFAULT 0,
    atualizado_em   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, cliente_id)
);

-- ── Controle do pipeline ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS bi.watermarks (
    tabela TEXT        PRIMARY KEY,
    ultimo TIMESTAMPTZ NOT NULL DEFAULT '-infinity'
);

-- Outbox analítico: alimentado por um subscriber do EventBus (V2) para capturar
-- timestamps exatos de transição de status. Criado já para o contrato existir.
CREATE TABLE IF NOT EXISTS bi.eventos_outbox (
    id           BIGSERIAL   PRIMARY KEY,
    tenant_id    UUID        NOT NULL,
    tipo_evento  TEXT        NOT NULL,
    aggregate_id TEXT        NOT NULL,
    payload      JSONB,
    ocorrido_em  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Motor de alertas ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS bi.regras_alerta (
    codigo       VARCHAR(8)  PRIMARY KEY,
    area         TEXT        NOT NULL,
    titulo       TEXT        NOT NULL,
    tela_destino TEXT        NOT NULL,
    peso_area    NUMERIC     NOT NULL DEFAULT 1.0,
    ativo        BOOLEAN     NOT NULL DEFAULT TRUE
);

INSERT INTO bi.regras_alerta (codigo, area, titulo, tela_destino, peso_area, ativo) VALUES
    ('A1',  'caixa',     'Caixa projetado negativo',            '/financeiro', 1.5, TRUE),
    ('A2',  'caixa',     'Contas vencidas aguardando cobrança', '/financeiro', 1.5, TRUE),
    ('A3',  'estoque',   'Risco de ruptura de estoque',         '/compras',    1.2, TRUE),
    ('A4',  'estoque',   'Estoque parado sem giro',             '/estoque',    1.2, TRUE),
    ('A5',  'comercial', 'Orçamento prestes a expirar',         '/orcamentos', 1.2, TRUE),
    ('A6',  'fiscal',    'Nota fiscal rejeitada pendente',      '/fiscal',     1.5, TRUE),
    ('A7',  'margem',    'Produto vendido abaixo do custo',     '/catalogo',   1.2, TRUE),
    ('A8',  'clientes',  'Cliente devedor sem bloqueio',        '/clientes',   1.0, TRUE),
    ('A9',  'vendas',    'Venda aberta no PDV há mais de 24h',  '/terminal',   1.0, TRUE),
    ('A10', 'vendas',    'Queda de receita na semana',          '/vendas',     1.0, TRUE),
    ('A11', 'caixa',     'Aperto de caixa com estoque parado',  '/analises',   1.5, TRUE),
    ('A12', 'margem',    'Rateio de custos fixos desatualizado', '/configuracoes', 1.0, TRUE)
ON CONFLICT (codigo) DO UPDATE
    SET area = EXCLUDED.area, titulo = EXCLUDED.titulo,
        tela_destino = EXCLUDED.tela_destino, peso_area = EXCLUDED.peso_area;

CREATE TABLE IF NOT EXISTS bi.alertas (
    alerta_id        UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL,
    codigo           VARCHAR(8)  NOT NULL REFERENCES bi.regras_alerta (codigo),
    entidade_id      TEXT        NOT NULL,
    impacto_centavos BIGINT      NOT NULL DEFAULT 0,
    urgencia_dias    INTEGER     NOT NULL DEFAULT 0,
    score            NUMERIC     NOT NULL DEFAULT 0,
    titulo           TEXT        NOT NULL,
    mensagem         TEXT        NOT NULL,
    link             TEXT        NOT NULL,
    status           VARCHAR(10) NOT NULL DEFAULT 'novo'
                         CHECK (status IN ('novo', 'visto', 'resolvido', 'ignorado')),
    criado_em        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    atualizado_em    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolvido_em     TIMESTAMPTZ,
    snooze_ate       TIMESTAMPTZ
);
-- Deduplicação: no máximo 1 alerta aberto por (tenant, regra, entidade).
CREATE UNIQUE INDEX IF NOT EXISTS idx_bi_alertas_aberto
    ON bi.alertas (tenant_id, codigo, entidade_id) WHERE status IN ('novo', 'visto');
CREATE INDEX IF NOT EXISTS idx_bi_alertas_fila
    ON bi.alertas (tenant_id, status, score DESC);

-- ── RLS: leitura/feedback pelo app sob o mesmo isolamento das proj_* ─────────

DO $$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'dim_produto', 'fato_vendas_item', 'fato_orcamentos', 'fato_contas_receber',
        'fato_contas_pagar', 'fato_estoque_snapshot', 'eventos_outbox', 'alertas',
        'analise_produtos', 'analise_clientes'
    ] LOOP
        EXECUTE format('ALTER TABLE bi.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE bi.%I FORCE  ROW LEVEL SECURITY', t);
        EXECUTE format('DROP POLICY IF EXISTS rls_bi_%s ON bi.%I', t, t);
        EXECUTE format(
            'CREATE POLICY rls_bi_%s ON bi.%I
                USING      (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)
                WITH CHECK (tenant_id = NULLIF(current_setting(''app.tenant_id'', true), '''')::uuid)',
            t, t
        );
    END LOOP;
END $$;

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- NUMERIC para aceitar tanto BIGINT quanto agregados SUM() sem cast explícito.
CREATE OR REPLACE FUNCTION bi.fmt_reais(v NUMERIC) RETURNS TEXT
LANGUAGE sql IMMUTABLE AS $$
    SELECT 'R$ ' || replace(replace(replace(
        to_char(v / 100.0, 'FM999,999,999,990.00'), ',', '#'), '.', ','), '#', '.')
$$;

-- Lê/inicializa o watermark de uma fonte e devolve o corte (com sobreposição de
-- 5s para não perder linhas gravadas no mesmo instante da última carga).
CREATE OR REPLACE FUNCTION bi.watermark_corte(fonte TEXT) RETURNS TIMESTAMPTZ
LANGUAGE plpgsql AS $$
DECLARE wm TIMESTAMPTZ;
BEGIN
    INSERT INTO bi.watermarks (tabela) VALUES (fonte) ON CONFLICT (tabela) DO NOTHING;
    SELECT ultimo INTO wm FROM bi.watermarks WHERE tabela = fonte;
    RETURN wm - INTERVAL '5 seconds';
END $$;

-- ── ETL ───────────────────────────────────────────────────────────────────────

-- SCD2 de produto: fecha a versão vigente quando algum atributo rastreado muda
-- e abre uma nova; produtos novos entram com valido_de = criado_em.
CREATE OR REPLACE FUNCTION bi.etl_dim_produto() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE n BIGINT;
BEGIN
    UPDATE bi.dim_produto d
       SET atual = FALSE, valido_ate = NOW()
      FROM proj_produtos p
     WHERE d.atual AND d.tenant_id = p.tenant_id AND d.produto_id = p.produto_id
       AND (d.sku, d.descricao, d.ncm, d.categoria, d.marca, d.preco_custo, d.preco_venda, d.ativo)
           IS DISTINCT FROM
           (p.sku, p.descricao, p.ncm, p.categoria, p.marca, p.preco_custo, p.preco_venda, p.ativo);

    INSERT INTO bi.dim_produto
        (tenant_id, produto_id, sku, descricao, ncm, categoria, marca,
         preco_custo, preco_venda, ativo, valido_de, valido_ate, atual)
    SELECT p.tenant_id, p.produto_id, p.sku, p.descricao, p.ncm, p.categoria, p.marca,
           p.preco_custo, p.preco_venda, p.ativo,
           CASE WHEN EXISTS (SELECT 1 FROM bi.dim_produto d
                              WHERE d.tenant_id = p.tenant_id AND d.produto_id = p.produto_id)
                THEN NOW() ELSE p.criado_em END,
           NULL, TRUE
      FROM proj_produtos p
     WHERE NOT EXISTS (SELECT 1 FROM bi.dim_produto d
                        WHERE d.tenant_id = p.tenant_id AND d.produto_id = p.produto_id AND d.atual);
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;

-- Itens de venda: custo congelado na carga a partir da versão vigente do produto.
-- Reprocessamentos só atualizam status/forma/data — nunca o custo congelado.
CREATE OR REPLACE FUNCTION bi.etl_vendas() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE corte TIMESTAMPTZ := bi.watermark_corte('proj_vendas'); n BIGINT;
BEGIN
    INSERT INTO bi.fato_vendas_item AS f
        (tenant_id, item_id, venda_id, produto_sk, produto_id, cliente_id, vendedor_id,
         forma_pagamento, status, data_venda, quantidade, receita_centavos,
         custo_unitario_centavos, custo_centavos, margem_centavos)
    SELECT v.tenant_id, vi.item_id, v.venda_id, dp.sk, vi.produto_id, v.cliente_id, v.vendedor_id,
           v.forma_pagamento, v.status,
           COALESCE(v.confirmada_em, v.atualizado_em)::date,
           vi.quantidade,
           vi.quantidade * vi.preco_unitario_centavos,
           COALESCE(dp.preco_custo, 0),
           vi.quantidade * COALESCE(dp.preco_custo, 0),
           vi.quantidade * (vi.preco_unitario_centavos - COALESCE(dp.preco_custo, 0))
      FROM proj_vendas v
      JOIN proj_vendas_itens vi ON vi.tenant_id = v.tenant_id AND vi.venda_id = v.venda_id
      LEFT JOIN bi.dim_produto dp
             ON dp.tenant_id = v.tenant_id AND dp.produto_id = vi.produto_id AND dp.atual
     WHERE v.status IN ('confirmada', 'cancelada') AND v.atualizado_em > corte
    ON CONFLICT (tenant_id, item_id) DO UPDATE
       SET status = EXCLUDED.status,
           forma_pagamento = EXCLUDED.forma_pagamento,
           data_venda = EXCLUDED.data_venda,
           -- Devolução parcial reduz a quantidade; receita/custo/margem são
           -- recalculados mantendo o custo unitário CONGELADO na 1ª carga.
           quantidade = EXCLUDED.quantidade,
           receita_centavos = EXCLUDED.receita_centavos,
           custo_centavos = EXCLUDED.quantidade * f.custo_unitario_centavos,
           margem_centavos = EXCLUDED.receita_centavos
                             - EXCLUDED.quantidade * f.custo_unitario_centavos;
    GET DIAGNOSTICS n = ROW_COUNT;

    -- Itens integralmente devolvidos somem da projeção — remove do fato também.
    DELETE FROM bi.fato_vendas_item f
     USING proj_vendas v
     WHERE v.tenant_id = f.tenant_id AND v.venda_id = f.venda_id
       AND v.atualizado_em > corte
       AND NOT EXISTS (SELECT 1 FROM proj_vendas_itens vi
                        WHERE vi.tenant_id = f.tenant_id AND vi.item_id = f.item_id);

    UPDATE bi.watermarks
       SET ultimo = COALESCE((SELECT MAX(atualizado_em) FROM proj_vendas), ultimo)
     WHERE tabela = 'proj_vendas';
    RETURN n;
END $$;

CREATE OR REPLACE FUNCTION bi.etl_financeiro() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE corte_cr TIMESTAMPTZ := bi.watermark_corte('proj_contas_receber');
        corte_cp TIMESTAMPTZ := bi.watermark_corte('proj_contas_pagar');
        n BIGINT; m BIGINT;
BEGIN
    INSERT INTO bi.fato_contas_receber AS f
        (tenant_id, conta_id, venda_id, cliente_id, valor_original, valor_recebido,
         status, criada_em, vencimento, data_liquidacao)
    SELECT c.tenant_id, c.conta_id, c.venda_id, c.cliente_id, c.valor_original, c.valor_recebido,
           c.status, c.criada_em, c.vencimento,
           CASE WHEN c.status = 'liquidada' THEN c.atualizado_em END
      FROM proj_contas_receber c
     WHERE c.atualizado_em > corte_cr
    ON CONFLICT (tenant_id, conta_id) DO UPDATE
       SET valor_recebido = EXCLUDED.valor_recebido,
           status = EXCLUDED.status,
           vencimento = EXCLUDED.vencimento,
           data_liquidacao = COALESCE(f.data_liquidacao, EXCLUDED.data_liquidacao);
    GET DIAGNOSTICS n = ROW_COUNT;

    INSERT INTO bi.fato_contas_pagar AS f
        (tenant_id, conta_id, pedido_id, fornecedor_id, valor_original, valor_pago,
         status, criada_em, vencimento, data_liquidacao)
    SELECT c.tenant_id, c.conta_id, c.pedido_id, c.fornecedor_id, c.valor_original, c.valor_pago,
           c.status, c.criada_em, c.vencimento,
           CASE WHEN c.status = 'liquidada' THEN c.atualizado_em END
      FROM proj_contas_pagar c
     WHERE c.atualizado_em > corte_cp
    ON CONFLICT (tenant_id, conta_id) DO UPDATE
       SET valor_pago = EXCLUDED.valor_pago,
           status = EXCLUDED.status,
           vencimento = EXCLUDED.vencimento,
           data_liquidacao = COALESCE(f.data_liquidacao, EXCLUDED.data_liquidacao);
    GET DIAGNOSTICS m = ROW_COUNT;

    -- Datas de liquidação exatas a partir do outbox de eventos.
    UPDATE bi.fato_contas_receber f
       SET data_liquidacao = ob.ts
      FROM (SELECT tenant_id, aggregate_id, MIN(ocorrido_em) AS ts
              FROM bi.eventos_outbox WHERE tipo_evento = 'ContaReceberLiquidada'
             GROUP BY 1, 2) ob
     WHERE f.tenant_id = ob.tenant_id AND f.conta_id::text = ob.aggregate_id
       AND f.data_liquidacao IS DISTINCT FROM ob.ts;

    UPDATE bi.fato_contas_pagar f
       SET data_liquidacao = ob.ts
      FROM (SELECT tenant_id, aggregate_id, MIN(ocorrido_em) AS ts
              FROM bi.eventos_outbox WHERE tipo_evento = 'ContaPagarLiquidada'
             GROUP BY 1, 2) ob
     WHERE f.tenant_id = ob.tenant_id AND f.conta_id::text = ob.aggregate_id
       AND f.data_liquidacao IS DISTINCT FROM ob.ts;

    UPDATE bi.watermarks SET ultimo = COALESCE((SELECT MAX(atualizado_em) FROM proj_contas_receber), ultimo)
     WHERE tabela = 'proj_contas_receber';
    UPDATE bi.watermarks SET ultimo = COALESCE((SELECT MAX(atualizado_em) FROM proj_contas_pagar), ultimo)
     WHERE tabela = 'proj_contas_pagar';
    RETURN n + m;
END $$;

CREATE OR REPLACE FUNCTION bi.etl_orcamentos() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE corte TIMESTAMPTZ := bi.watermark_corte('proj_orcamentos'); n BIGINT;
BEGIN
    INSERT INTO bi.fato_orcamentos AS f
        (tenant_id, orcamento_id, vendedor_id, cliente_id, total_centavos, desconto_centavos,
         status, validade_dias, venda_id, criado_em, data_decisao)
    SELECT o.tenant_id, o.orcamento_id, o.vendedor_id, o.cliente_id, o.total_centavos,
           o.desconto_centavos, o.status, o.validade_dias, o.venda_id, o.criado_em,
           CASE WHEN o.status IN ('aceito', 'recusado', 'expirado', 'convertido', 'cancelado')
                THEN o.atualizado_em END
      FROM proj_orcamentos o
     WHERE o.atualizado_em > corte
    ON CONFLICT (tenant_id, orcamento_id) DO UPDATE
       SET total_centavos = EXCLUDED.total_centavos,
           desconto_centavos = EXCLUDED.desconto_centavos,
           status = EXCLUDED.status,
           venda_id = EXCLUDED.venda_id,
           data_decisao = COALESCE(f.data_decisao, EXCLUDED.data_decisao);
    GET DIAGNOSTICS n = ROW_COUNT;

    -- Data de decisão exata a partir do outbox de eventos (quando disponível,
    -- substitui o proxy de atualizado_em).
    UPDATE bi.fato_orcamentos f
       SET data_decisao = ob.ts
      FROM (SELECT tenant_id, aggregate_id, MIN(ocorrido_em) AS ts
              FROM bi.eventos_outbox
             WHERE tipo_evento IN ('OrcamentoAceito', 'OrcamentoRecusado', 'OrcamentoExpirado',
                                   'OrcamentoConvertidoEmVenda', 'OrcamentoCancelado')
             GROUP BY 1, 2) ob
     WHERE f.tenant_id = ob.tenant_id AND f.orcamento_id::text = ob.aggregate_id
       AND f.data_decisao IS DISTINCT FROM ob.ts;

    UPDATE bi.watermarks SET ultimo = COALESCE((SELECT MAX(atualizado_em) FROM proj_orcamentos), ultimo)
     WHERE tabela = 'proj_orcamentos';
    RETURN n;
END $$;

-- Foto diária do estoque (reexecuções no mesmo dia sobrescrevem a foto do dia).
CREATE OR REPLACE FUNCTION bi.snapshot_estoque() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE n BIGINT;
BEGIN
    INSERT INTO bi.fato_estoque_snapshot
        (tenant_id, produto_id, data, quantidade, custo_medio, valor_centavos, estoque_minimo, em_ruptura)
    SELECT s.tenant_id, s.produto_id, CURRENT_DATE, s.quantidade, s.custo_medio,
           s.quantidade::bigint * s.custo_medio, s.estoque_minimo,
           s.quantidade <= s.estoque_minimo
      FROM proj_saldo_estoque s
    ON CONFLICT (tenant_id, produto_id, data) DO UPDATE
       SET quantidade = EXCLUDED.quantidade,
           custo_medio = EXCLUDED.custo_medio,
           valor_centavos = EXCLUDED.valor_centavos,
           estoque_minimo = EXCLUDED.estoque_minimo,
           em_ruptura = EXCLUDED.em_ruptura;
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;

-- ── Análises derivadas: ABC/XYZ/cobertura e RFM ──────────────────────────────

-- Rebuild completo a cada ciclo (volumes de PME — mais simples e auditável que
-- manutenção incremental de janelas móveis).
CREATE OR REPLACE FUNCTION bi.calcular_analise_produtos() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE n BIGINT;
BEGIN
    DELETE FROM bi.analise_produtos;

    INSERT INTO bi.analise_produtos
        (tenant_id, produto_id, sku, descricao, categoria, receita_12m, margem_12m,
         qtd_12m, demanda_dia_90d, cobertura_dias, classe_abc, classe_xyz,
         dias_sem_venda, quantidade, estoque_minimo, valor_imobilizado)
    WITH v12 AS (
        SELECT f.tenant_id, f.produto_id,
               SUM(f.receita_centavos) AS receita,
               SUM(f.margem_centavos)  AS margem,
               SUM(f.quantidade)       AS qtd,
               MAX(f.data_venda)       AS ultima_venda
          FROM bi.fato_vendas_item f
         WHERE f.status = 'confirmada' AND f.data_venda >= CURRENT_DATE - 365
         GROUP BY 1, 2
    ),
    d90 AS (
        SELECT f.tenant_id, f.produto_id, SUM(f.quantidade)::numeric / 90 AS dia
          FROM bi.fato_vendas_item f
         WHERE f.status = 'confirmada' AND f.data_venda >= CURRENT_DATE - 90
         GROUP BY 1, 2
    ),
    -- Demanda semanal (26 semanas, com semanas zeradas) → coeficiente de variação.
    semanal AS (
        SELECT p.tenant_id, p.produto_id, s.w,
               COALESCE(SUM(f.quantidade), 0) AS q
          FROM proj_produtos p
          CROSS JOIN generate_series(0, 25) AS s(w)
          LEFT JOIN bi.fato_vendas_item f
                 ON f.tenant_id = p.tenant_id AND f.produto_id = p.produto_id
                AND f.status = 'confirmada'
                AND f.data_venda >= CURRENT_DATE - (s.w + 1) * 7
                AND f.data_venda <  CURRENT_DATE - s.w * 7
         GROUP BY 1, 2, 3
    ),
    cv AS (
        SELECT tenant_id, produto_id, AVG(q) AS media, STDDEV_POP(q) AS desvio
          FROM semanal GROUP BY 1, 2
    ),
    abc AS (
        SELECT p.tenant_id, p.produto_id,
               COALESCE(v.receita, 0) AS receita,
               SUM(COALESCE(v.receita, 0)) OVER w_antes AS acumulado_antes,
               SUM(COALESCE(v.receita, 0)) OVER w_total AS total
          FROM proj_produtos p
          LEFT JOIN v12 v ON v.tenant_id = p.tenant_id AND v.produto_id = p.produto_id
        WINDOW w_antes AS (PARTITION BY p.tenant_id ORDER BY COALESCE(v.receita, 0) DESC
                           ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING),
               w_total AS (PARTITION BY p.tenant_id)
    )
    SELECT p.tenant_id, p.produto_id, p.sku, p.descricao, p.categoria,
           COALESCE(v.receita, 0), COALESCE(v.margem, 0), COALESCE(v.qtd, 0),
           COALESCE(d.dia, 0),
           CASE WHEN COALESCE(d.dia, 0) > 0
                THEN FLOOR(COALESCE(s.quantidade, 0) / d.dia)::int END,
           CASE WHEN COALESCE(v.receita, 0) = 0 OR a.total = 0 THEN 'C'
                WHEN COALESCE(a.acumulado_antes, 0)::numeric / a.total < 0.80 THEN 'A'
                WHEN COALESCE(a.acumulado_antes, 0)::numeric / a.total < 0.95 THEN 'B'
                ELSE 'C' END,
           CASE WHEN COALESCE(c.media, 0) = 0 THEN NULL
                WHEN c.desvio / c.media < 0.5 THEN 'X'
                WHEN c.desvio / c.media < 1.0 THEN 'Y'
                ELSE 'Z' END,
           CASE WHEN v.ultima_venda IS NOT NULL
                THEN (CURRENT_DATE - v.ultima_venda)::int END,
           COALESCE(s.quantidade, 0), COALESCE(s.estoque_minimo, 0),
           COALESCE(s.quantidade, 0)::bigint * COALESCE(s.custo_medio, 0)
      FROM proj_produtos p
      LEFT JOIN proj_saldo_estoque s ON s.tenant_id = p.tenant_id AND s.produto_id = p.produto_id
      LEFT JOIN v12 v ON v.tenant_id = p.tenant_id AND v.produto_id = p.produto_id
      LEFT JOIN d90 d ON d.tenant_id = p.tenant_id AND d.produto_id = p.produto_id
      LEFT JOIN cv  c ON c.tenant_id = p.tenant_id AND c.produto_id = p.produto_id
      LEFT JOIN abc a ON a.tenant_id = p.tenant_id AND a.produto_id = p.produto_id;
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;

CREATE OR REPLACE FUNCTION bi.calcular_analise_clientes() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE n BIGINT;
BEGIN
    DELETE FROM bi.analise_clientes;

    INSERT INTO bi.analise_clientes
        (tenant_id, cliente_id, nome, recencia_dias, frequencia_12m, valor_12m,
         score_r, score_f, score_m, segmento, saldo_vencido)
    WITH base AS (
        SELECT v.tenant_id, v.cliente_id,
               (CURRENT_DATE - MAX(v.confirmada_em)::date)::int AS recencia,
               COUNT(*)::int                                    AS freq,
               SUM(v.total_centavos)                            AS valor
          FROM proj_vendas v
         WHERE v.status = 'confirmada' AND v.cliente_id IS NOT NULL
           AND v.confirmada_em >= NOW() - INTERVAL '365 days'
         GROUP BY 1, 2
    ),
    scores AS (
        SELECT b.*,
               NTILE(5) OVER (PARTITION BY b.tenant_id ORDER BY b.recencia DESC) AS r,
               NTILE(5) OVER (PARTITION BY b.tenant_id ORDER BY b.freq ASC)      AS f,
               NTILE(5) OVER (PARTITION BY b.tenant_id ORDER BY b.valor ASC)     AS m
          FROM base b
    ),
    vencido AS (
        SELECT c.tenant_id, c.cliente_id, SUM(c.valor_original - c.valor_recebido) AS saldo
          FROM proj_contas_receber c
         WHERE c.status IN ('pendente', 'parcial') AND c.vencimento < NOW()
         GROUP BY 1, 2
    )
    SELECT s.tenant_id, s.cliente_id, cl.nome, s.recencia, s.freq, s.valor,
           s.r, s.f, s.m,
           CASE WHEN s.r >= 4 AND s.f >= 4                 THEN 'Campeão'
                WHEN s.r >= 3 AND s.f >= 3                 THEN 'Fiel'
                WHEN s.r >= 4                              THEN 'Novo/Recente'
                WHEN s.r <= 2 AND (s.f >= 4 OR s.m >= 4)   THEN 'Em risco'
                WHEN s.r <= 2                              THEN 'Perdido'
                ELSE 'Ocasional' END,
           COALESCE(ve.saldo, 0)
      FROM scores s
      JOIN proj_clientes cl ON cl.tenant_id = s.tenant_id AND cl.cliente_id = s.cliente_id
      LEFT JOIN vencido ve ON ve.tenant_id = s.tenant_id AND ve.cliente_id = s.cliente_id;
    GET DIAGNOSTICS n = ROW_COUNT;
    RETURN n;
END $$;

-- ── Avaliação de alertas ──────────────────────────────────────────────────────
-- Recalcula os "ofensores" atuais de cada regra ativa, resolve automaticamente
-- os alertas abertos cuja condição deixou de valer e insere/atualiza os demais.
-- score = impacto normalizado pelo porte do tenant × peso de urgência × peso da área.
CREATE OR REPLACE FUNCTION bi.avaliar_alertas() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE n BIGINT;
BEGIN
    CREATE TEMP TABLE tmp_alertas (
        tenant_id        UUID,
        codigo           VARCHAR(8),
        entidade_id      TEXT,
        impacto_centavos BIGINT,
        urgencia_dias    INTEGER,
        titulo           TEXT,
        mensagem         TEXT,
        link             TEXT     -- NULL → usa a tela padrão da regra
    ) ON COMMIT DROP;

    -- Receita mensal média (90d/3) por tenant — normalizador do score.
    CREATE TEMP TABLE tmp_receita ON COMMIT DROP AS
    SELECT v.tenant_id, COALESCE(SUM(v.total_centavos), 0) / 3 AS receita_mensal
      FROM proj_vendas v
     WHERE v.status = 'confirmada' AND v.confirmada_em >= NOW() - INTERVAL '90 days'
     GROUP BY v.tenant_id;

    -- Taxa de conversão de orçamentos por tenant (piso 10% para não zerar impacto).
    CREATE TEMP TABLE tmp_conversao ON COMMIT DROP AS
    SELECT o.tenant_id,
           GREATEST(
               COUNT(*) FILTER (WHERE o.status = 'convertido')::numeric
                   / NULLIF(COUNT(*) FILTER (WHERE o.status IN ('aceito','recusado','expirado','convertido')), 0),
               0.10) AS taxa
      FROM proj_orcamentos o
     WHERE o.atualizado_em >= NOW() - INTERVAL '90 days'
     GROUP BY o.tenant_id;

    -- A1: fluxo operacional projetado negativo em 30 dias (CR aberto − CP aberto).
    INSERT INTO tmp_alertas
    SELECT t.tenant_id, 'A1', 'caixa-30d',
           cp.total - cr.total, 0,
           'Seu dinheiro pode faltar nos próximos 30 dias',
           format('Você vai receber %s, mas vai precisar pagar %s nos próximos 30 dias — vai faltar %s. Cobre logo os clientes com maior valor em atraso e tente mais prazo com os fornecedores.',
                  bi.fmt_reais(cr.total), bi.fmt_reais(cp.total), bi.fmt_reais(cp.total - cr.total)),
           NULL
      FROM tenants t
      JOIN LATERAL (SELECT COALESCE(SUM(valor_original - valor_recebido), 0) AS total
                      FROM proj_contas_receber c
                     WHERE c.tenant_id = t.tenant_id AND c.status IN ('pendente', 'parcial')
                       AND c.vencimento <= NOW() + INTERVAL '30 days') cr ON TRUE
      JOIN LATERAL (SELECT COALESCE(SUM(valor_original - valor_pago), 0) AS total
                      FROM proj_contas_pagar c
                     WHERE c.tenant_id = t.tenant_id AND c.status IN ('pendente', 'parcial')
                       AND c.vencimento <= NOW() + INTERVAL '30 days') cp ON TRUE
     WHERE t.status = 'ativo' AND cp.total > cr.total;

    -- A2: contas vencidas, agregado por tenant com o maior devedor nomeado.
    INSERT INTO tmp_alertas
    SELECT venc.tenant_id, 'A2', 'cobranca',
           venc.total, 0,
           'Contas vencidas aguardando cobrança',
           format('%s cliente(s) somam %s vencidos. Maior devedor: %s (%s). Ligue hoje para os maiores — a lista está no Financeiro, filtro "vencidas".',
                  venc.clientes, bi.fmt_reais(venc.total), venc.top_nome, bi.fmt_reais(venc.top_saldo)),
           NULL
      FROM (
        SELECT s.tenant_id, SUM(s.saldo) AS total, COUNT(*) AS clientes,
               (ARRAY_AGG(s.nome ORDER BY s.saldo DESC))[1] AS top_nome,
               (ARRAY_AGG(s.saldo ORDER BY s.saldo DESC))[1] AS top_saldo
          FROM (
            SELECT c.tenant_id, c.cliente_id,
                   COALESCE(cl.nome, 'Consumidor não identificado') AS nome,
                   SUM(c.valor_original - c.valor_recebido) AS saldo
              FROM proj_contas_receber c
              LEFT JOIN proj_clientes cl
                     ON cl.tenant_id = c.tenant_id AND cl.cliente_id = c.cliente_id
             WHERE c.status IN ('pendente', 'parcial') AND c.vencimento < NOW()
             GROUP BY c.tenant_id, c.cliente_id, cl.nome
          ) s
         GROUP BY s.tenant_id
      ) venc
     WHERE venc.total > 0;

    -- A3: ruptura iminente — saldo no mínimo (ou abaixo) com demanda nos últimos 30d.
    -- estoque_minimo > 0 exclui itens não estocáveis (serviços) do radar.
    INSERT INTO tmp_alertas
    SELECT s.tenant_id, 'A3', s.produto_id::text,
           (d.dia * p.preco_venda * 7)::bigint,
           CASE WHEN d.dia > 0 THEN FLOOR(s.quantidade / d.dia)::int ELSE 0 END,
           format('"%s" pode faltar em breve', p.descricao),
           format('Você vende ~%s un/dia de "%s" e restam %s (mínimo %s) — cobertura de ~%s dia(s). Cada semana em falta ≈ %s de venda perdida. Gere um pedido de compra hoje.',
                  round(d.dia, 1), p.descricao, s.quantidade, s.estoque_minimo,
                  CASE WHEN d.dia > 0 THEN FLOOR(s.quantidade / d.dia)::text ELSE '0' END,
                  bi.fmt_reais((d.dia * p.preco_venda * 7)::bigint)),
           -- Pedido 1-clique: a tela de Compras pré-preenche o pedido a partir daqui.
           format('/compras?produto=%s&quantidade=%s',
                  s.produto_id, GREATEST(CEIL(d.dia * 30)::int - s.quantidade, 1))
      FROM proj_saldo_estoque s
      JOIN proj_produtos p ON p.tenant_id = s.tenant_id AND p.produto_id = s.produto_id
      JOIN LATERAL (SELECT COALESCE(SUM(vi.quantidade), 0)::numeric / 30 AS dia
                      FROM proj_vendas_itens vi
                      JOIN proj_vendas v ON v.tenant_id = vi.tenant_id AND v.venda_id = vi.venda_id
                     WHERE vi.tenant_id = s.tenant_id AND vi.produto_id = s.produto_id
                       AND v.status = 'confirmada'
                       AND v.confirmada_em >= NOW() - INTERVAL '30 days') d ON TRUE
     WHERE s.estoque_minimo > 0 AND s.quantidade <= s.estoque_minimo AND d.dia > 0;

    -- A5: orçamento emitido vencendo em até 3 dias (ou já vencido sem decisão).
    INSERT INTO tmp_alertas
    SELECT o.tenant_id, 'A5', o.orcamento_id::text,
           (o.total_centavos * COALESCE(cv.taxa, 0.25))::bigint,
           GREATEST(EXTRACT(day FROM (o.criado_em + o.validade_dias * INTERVAL '1 day') - NOW()), 0)::int,
           format('Orçamento de %s prestes a vencer', bi.fmt_reais(o.total_centavos)),
           format('O orçamento de %s para %s vence em %s dia(s) e o cliente ainda não respondeu. Pelo seu histórico, orçamentos assim viram venda perto de %s. Ligue para o cliente hoje.',
                  bi.fmt_reais(o.total_centavos),
                  COALESCE(cl.nome, 'cliente não identificado'),
                  GREATEST(EXTRACT(day FROM (o.criado_em + o.validade_dias * INTERVAL '1 day') - NOW()), 0)::int,
                  bi.fmt_reais((o.total_centavos * COALESCE(cv.taxa, 0.25))::bigint)),
           NULL
      FROM proj_orcamentos o
      LEFT JOIN proj_clientes cl ON cl.tenant_id = o.tenant_id AND cl.cliente_id = o.cliente_id
      LEFT JOIN tmp_conversao cv ON cv.tenant_id = o.tenant_id
     WHERE o.status = 'emitido'
       AND o.criado_em + o.validade_dias * INTERVAL '1 day' <= NOW() + INTERVAL '3 days';

    -- A6: NF-e rejeitada há mais de 24h sem outra autorizada para a mesma venda.
    INSERT INTO tmp_alertas
    SELECT nf.tenant_id, 'A6', nf.nf_id::text,
           nf.total_centavos, 0,
           format('NF-e %s/%s rejeitada há mais de 24h', nf.serie, nf.numero),
           format('A nota %s/%s (%s) foi rejeitada%s. Vender sem essa nota certinha pode dar multa — corrija o problema e envie de novo.',
                  nf.serie, nf.numero, bi.fmt_reais(nf.total_centavos),
                  CASE WHEN nf.rejeicao_codigo IS NOT NULL
                       THEN format(' (código %s: %s)', nf.rejeicao_codigo, COALESCE(left(nf.rejeicao_motivo, 120), 'sem detalhe'))
                       ELSE '' END),
           NULL
      FROM proj_notas_fiscais nf
     WHERE nf.status = 'rejeitada' AND nf.atualizado_em < NOW() - INTERVAL '24 hours'
       AND NOT EXISTS (SELECT 1 FROM proj_notas_fiscais nf2
                        WHERE nf2.tenant_id = nf.tenant_id AND nf2.venda_id = nf.venda_id
                          AND nf2.status = 'autorizada');

    -- A7: produto ativo com preço de venda abaixo do custo (cadastro ou médio real).
    INSERT INTO tmp_alertas
    SELECT p.tenant_id, 'A7', p.produto_id::text,
           (GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0)) - p.preco_venda)
               * GREATEST(COALESCE(d.qtd30, 0), 1),
           0,
           format('"%s" está sendo vendido abaixo do custo', p.descricao),
           format('"%s" (SKU %s) tem preço de venda %s e custo %s — prejuízo de %s por unidade. Corrija o preço no Catálogo.',
                  p.descricao, p.sku, bi.fmt_reais(p.preco_venda),
                  bi.fmt_reais(GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0))),
                  bi.fmt_reais(GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0)) - p.preco_venda)),
           NULL
      FROM proj_produtos p
      LEFT JOIN proj_saldo_estoque s ON s.tenant_id = p.tenant_id AND s.produto_id = p.produto_id
      LEFT JOIN LATERAL (SELECT SUM(vi.quantidade) AS qtd30
                           FROM proj_vendas_itens vi
                           JOIN proj_vendas v ON v.tenant_id = vi.tenant_id AND v.venda_id = vi.venda_id
                          WHERE vi.tenant_id = p.tenant_id AND vi.produto_id = p.produto_id
                            AND v.status = 'confirmada'
                            AND v.confirmada_em >= NOW() - INTERVAL '30 days') d ON TRUE
     WHERE p.ativo AND p.preco_venda < GREATEST(p.preco_custo, COALESCE(s.custo_medio, 0));

    -- A8: cliente com saldo vencido há mais de 15 dias e ainda liberado a prazo.
    INSERT INTO tmp_alertas
    SELECT dev.tenant_id, 'A8', dev.cliente_id::text,
           dev.saldo, 0,
           format('%s deve %s e não está bloqueado', dev.nome, bi.fmt_reais(dev.saldo)),
           format('%s tem %s vencidos há mais de 15 dias e continua liberado para comprar a prazo. Bloqueie novas vendas a prazo ou negocie o débito.',
                  dev.nome, bi.fmt_reais(dev.saldo)),
           NULL
      FROM (
        SELECT c.tenant_id, c.cliente_id, cl.nome,
               SUM(c.valor_original - c.valor_recebido) AS saldo
          FROM proj_contas_receber c
          JOIN proj_clientes cl ON cl.tenant_id = c.tenant_id AND cl.cliente_id = c.cliente_id
         WHERE c.status IN ('pendente', 'parcial')
           AND c.vencimento < NOW() - INTERVAL '15 days'
           AND NOT cl.bloqueado
         GROUP BY c.tenant_id, c.cliente_id, cl.nome
        HAVING SUM(c.valor_original - c.valor_recebido) > 0
      ) dev;

    -- A9: venda iniciada no PDV há mais de 24h sem confirmação/cancelamento.
    INSERT INTO tmp_alertas
    SELECT v.tenant_id, 'A9', v.venda_id::text,
           v.total_centavos, 0,
           'Venda aberta no PDV há mais de 24h',
           format('Uma venda de %s aberta em %s ainda não foi confirmada nem cancelada. Termine ou cancele ela no Terminal.',
                  bi.fmt_reais(v.total_centavos), to_char(v.criada_em, 'DD/MM/YYYY')),
           NULL
      FROM proj_vendas v
     WHERE v.status = 'iniciada' AND v.criada_em < NOW() - INTERVAL '24 hours';

    -- A4: estoque morto — capital imobilizado sem giro há 90+ dias.
    INSERT INTO tmp_alertas
    SELECT ap.tenant_id, 'A4', ap.produto_id::text,
           ap.valor_imobilizado, 30,
           format('%s parados em "%s" sem giro', bi.fmt_reais(ap.valor_imobilizado), ap.descricao),
           format('"%s" (SKU %s) tem %s unidade(s) paradas (%s) %s. Promova, remaneje ou devolva ao fornecedor para liberar caixa.',
                  ap.descricao, ap.sku, ap.quantidade, bi.fmt_reais(ap.valor_imobilizado),
                  CASE WHEN ap.dias_sem_venda IS NULL THEN 'sem nenhuma venda registrada'
                       ELSE format('há %s dias sem vender', ap.dias_sem_venda) END),
           NULL
      FROM bi.analise_produtos ap
      JOIN proj_produtos p ON p.tenant_id = ap.tenant_id AND p.produto_id = ap.produto_id
     WHERE ap.quantidade > 0 AND ap.valor_imobilizado > 0
       -- Nunca vendeu só conta como morto se o cadastro tem 90+ dias.
       AND (ap.dias_sem_venda >= 90
            OR (ap.dias_sem_venda IS NULL AND p.criado_em < NOW() - INTERVAL '90 days'));

    -- A10: receita da última semana < 70% da média das 8 semanas anteriores.
    INSERT INTO tmp_alertas
    SELECT h.tenant_id, 'A10', 'receita-7d',
           (h.media - h.atual)::bigint, 7,
           'Você vendeu bem menos essa semana',
           format('Nos últimos 7 dias você vendeu %s — %s%% menos do que costuma vender numa semana (%s). Veja o que mudou antes de baixar preços, e retome contato com os orçamentos em aberto.',
                  bi.fmt_reais(h.atual), ROUND((1 - h.atual / h.media) * 100), bi.fmt_reais(h.media)),
           NULL
      FROM (
        SELECT t.tenant_id,
               COALESCE(SUM(v.total_centavos) FILTER (
                   WHERE v.confirmada_em >= NOW() - INTERVAL '7 days'), 0)::numeric AS atual,
               COALESCE(SUM(v.total_centavos) FILTER (
                   WHERE v.confirmada_em >= NOW() - INTERVAL '63 days'
                     AND v.confirmada_em <  NOW() - INTERVAL '7 days'), 0)::numeric / 8 AS media
          FROM tenants t
          LEFT JOIN proj_vendas v ON v.tenant_id = t.tenant_id AND v.status = 'confirmada'
         WHERE t.status = 'ativo'
         GROUP BY t.tenant_id
      ) h
     WHERE h.media > 0 AND h.atual < 0.7 * h.media;

    -- A11: aperto de caixa nas próximas 4 semanas + capital parado em estoque.
    -- Contas a pagar NÃO entram na fórmula de preço (são fluxo de caixa, não
    -- custo unitário) — aqui elas atuam como SINAL: se vai faltar dinheiro e
    -- há estoque encalhado, a recomendação é acelerar o giro usando o desconto
    -- que o assistente de preços já sugere para itens parados.
    INSERT INTO tmp_alertas
    SELECT cx.tenant_id, 'A11', 'caixa-giro',
           LEAST(cx.deficit, enc.valor_parado), 7,
           'Vai faltar caixa e há dinheiro parado em estoque',
           format('Nas próximas 4 semanas devem sair %s a mais do que entrar, e você tem %s parados em %s produto(s) sem venda há 90+ dias. O assistente de preços já sugere desconto para essas peças girarem — veja a aba "Preços e Margens" em Análises e aplique as sugestões para virar estoque em caixa.',
                  bi.fmt_reais(cx.deficit), bi.fmt_reais(enc.valor_parado), enc.produtos),
           NULL
      FROM (
        SELECT t.tenant_id, cp.total - cr.total AS deficit
          FROM tenants t
          JOIN LATERAL (SELECT COALESCE(SUM(valor_original - valor_recebido), 0) AS total
                          FROM proj_contas_receber c
                         WHERE c.tenant_id = t.tenant_id AND c.status IN ('pendente', 'parcial')
                           AND c.vencimento <= NOW() + INTERVAL '28 days') cr ON TRUE
          JOIN LATERAL (SELECT COALESCE(SUM(valor_original - valor_pago), 0) AS total
                          FROM proj_contas_pagar c
                         WHERE c.tenant_id = t.tenant_id AND c.status IN ('pendente', 'parcial')
                           AND c.vencimento <= NOW() + INTERVAL '28 days') cp ON TRUE
         WHERE t.status = 'ativo' AND cp.total > cr.total
      ) cx
      JOIN LATERAL (
        SELECT COALESCE(SUM(ap.valor_imobilizado), 0) AS valor_parado, COUNT(*) AS produtos
          FROM bi.analise_produtos ap
          JOIN proj_produtos p ON p.tenant_id = ap.tenant_id AND p.produto_id = ap.produto_id
         WHERE ap.tenant_id = cx.tenant_id AND ap.quantidade > 0 AND ap.valor_imobilizado > 0
           -- Nunca vendeu só conta como parado se o cadastro tem 90+ dias.
           AND (ap.dias_sem_venda >= 90
                OR (ap.dias_sem_venda IS NULL AND p.criado_em < NOW() - INTERVAL '90 days'))
      ) enc ON TRUE
     WHERE enc.valor_parado > 0;

    -- A12: recalibração do rateio — o faturamento real (média de 90 dias)
    -- divergiu 25%+ do estimado nas Configurações. Como os custos fixos são
    -- uma fração do preço (fixos ÷ faturamento estimado), a divergência faz
    -- cada venda carregar custo fixo de mais (freia venda) ou de menos (corrói
    -- a sobra). Só avalia com 60+ dias de histórico, para a média ser honesta.
    INSERT INTO tmp_alertas
    SELECT f.tenant_id, 'A12', 'rateio-faturamento',
           ABS(f.real_mensal - f.estimado)::bigint, 30,
           'Atualize o faturamento estimado — o rateio dos custos fixos desatualizou',
           format('Você está faturando ~%s/mês (média de 90 dias), mas a sugestão de preço divide os custos fixos assumindo %s/mês. %s Atualize o "Faturamento esperado por mês" em Configurações para os preços refletirem a realidade.',
                  bi.fmt_reais(f.real_mensal::bigint), bi.fmt_reais(f.estimado::bigint),
                  CASE WHEN f.real_mensal > f.estimado
                       THEN 'Seus preços estão carregando custo fixo A MAIS do que o necessário.'
                       ELSE 'Seus preços estão carregando custo fixo A MENOS — a sobra some sem aparecer.' END),
           NULL
      FROM (
        SELECT t.tenant_id, t.faturamento_mensal_estimado_centavos::numeric AS estimado,
               COALESCE(SUM(v.total_centavos), 0) / 3.0 AS real_mensal,
               MIN(v.confirmada_em) AS primeira_venda
          FROM tenants t
          LEFT JOIN proj_vendas v ON v.tenant_id = t.tenant_id AND v.status = 'confirmada'
                 AND v.confirmada_em >= NOW() - INTERVAL '90 days'
         WHERE t.status = 'ativo' AND t.faturamento_mensal_estimado_centavos IS NOT NULL
           AND t.custos_fixos_mensais_centavos IS NOT NULL
         GROUP BY t.tenant_id, t.faturamento_mensal_estimado_centavos
      ) f
     WHERE f.primeira_venda <= NOW() - INTERVAL '60 days'
       AND ABS(f.real_mensal - f.estimado) > 0.25 * f.estimado;

    -- Auto-resolução: alerta aberto cuja condição sumiu do recálculo.
    UPDATE bi.alertas a
       SET status = 'resolvido', resolvido_em = NOW(), atualizado_em = NOW()
     WHERE a.status IN ('novo', 'visto')
       AND NOT EXISTS (SELECT 1 FROM tmp_alertas t
                        WHERE t.tenant_id = a.tenant_id AND t.codigo = a.codigo
                          AND t.entidade_id = a.entidade_id);

    -- Upsert dos ofensores atuais (pula regra inativa e entidades em snooze).
    INSERT INTO bi.alertas AS a
        (tenant_id, codigo, entidade_id, impacto_centavos, urgencia_dias,
         score, titulo, mensagem, link)
    SELECT t.tenant_id, t.codigo, t.entidade_id, t.impacto_centavos, t.urgencia_dias,
           ROUND(
               (ln(1 + GREATEST(t.impacto_centavos, 0) / 100.0)
                   / GREATEST(ln(1 + COALESCE(rc.receita_mensal, 0) / 100.0), 1.0))
               * CASE WHEN t.urgencia_dias <= 0 THEN 2.0
                      WHEN t.urgencia_dias <= 7 THEN 1.5
                      ELSE 1.0 END
               * r.peso_area
               -- Decay de feedback: cada "ignorar" da mesma regra no tenant (90d)
               -- corta o score pela metade (piso 0,25) — o gestor calibra o motor.
               * GREATEST(0.25, POWER(0.5, fb.ignorados)), 4),
           t.titulo, t.mensagem, COALESCE(t.link, r.tela_destino)
      FROM tmp_alertas t
      JOIN bi.regras_alerta r ON r.codigo = t.codigo AND r.ativo
      LEFT JOIN tmp_receita rc ON rc.tenant_id = t.tenant_id
      LEFT JOIN LATERAL (SELECT COUNT(*) AS ignorados FROM bi.alertas ig
                          WHERE ig.tenant_id = t.tenant_id AND ig.codigo = t.codigo
                            AND ig.status = 'ignorado'
                            AND ig.atualizado_em >= NOW() - INTERVAL '90 days') fb ON TRUE
     WHERE NOT EXISTS (SELECT 1 FROM bi.alertas ig
                        WHERE ig.tenant_id = t.tenant_id AND ig.codigo = t.codigo
                          AND ig.entidade_id = t.entidade_id
                          AND ig.status = 'ignorado' AND ig.snooze_ate > NOW())
    ON CONFLICT (tenant_id, codigo, entidade_id) WHERE status IN ('novo', 'visto')
    DO UPDATE
       SET impacto_centavos = EXCLUDED.impacto_centavos,
           urgencia_dias = EXCLUDED.urgencia_dias,
           score = EXCLUDED.score,
           titulo = EXCLUDED.titulo,
           mensagem = EXCLUDED.mensagem,
           link = EXCLUDED.link,
           atualizado_em = NOW();
    GET DIAGNOSTICS n = ROW_COUNT;

    DROP TABLE IF EXISTS tmp_alertas;
    DROP TABLE IF EXISTS tmp_receita;
    DROP TABLE IF EXISTS tmp_conversao;
    RETURN n;
END $$;

-- ── Score de saúde do negócio ─────────────────────────────────────────────────
-- Nota 0–100 composta pelas métricas já coletadas, com o detalhamento de cada
-- componente (nota, peso e explicação) para a UI mostrar o porquê. Componentes
-- sem dado (ex.: sem estoque, sem mês anterior) saem da conta e os pesos são
-- renormalizados — nenhum número é inventado.
CREATE OR REPLACE FUNCTION bi.score_saude(p_tenant UUID) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE
    receita_30d NUMERIC;  receita_ant NUMERIC;  receita_mes NUMERIC;
    cr30 NUMERIC; cp30 NUMERIC; vencidas NUMERIC;
    margem_pct NUMERIC;
    estoque_total NUMERIC; estoque_parado NUMERIC;
    meta NUMERIC; esperado_ate_hoje NUMERIC;
    comp JSONB := '[]'::jsonb;
    nota NUMERIC; soma_pesos NUMERIC := 0; soma_notas NUMERIC := 0;
BEGIN
    SELECT COALESCE(SUM(total_centavos) FILTER (WHERE confirmada_em >= NOW() - INTERVAL '30 days'), 0),
           COALESCE(SUM(total_centavos) FILTER (WHERE confirmada_em >= NOW() - INTERVAL '60 days'
                                                  AND confirmada_em <  NOW() - INTERVAL '30 days'), 0),
           COALESCE(SUM(total_centavos) FILTER (WHERE confirmada_em >= date_trunc('month', NOW())), 0)
      INTO receita_30d, receita_ant, receita_mes
      FROM proj_vendas WHERE tenant_id = p_tenant AND status = 'confirmada';

    SELECT meta_faturamento_mensal_centavos INTO meta
      FROM tenants WHERE tenant_id = p_tenant;

    SELECT COALESCE(SUM(valor_original - valor_recebido) FILTER (
               WHERE vencimento <= NOW() + INTERVAL '30 days'), 0),
           COALESCE(SUM(valor_original - valor_recebido) FILTER (WHERE vencimento < NOW()), 0)
      INTO cr30, vencidas
      FROM proj_contas_receber
     WHERE tenant_id = p_tenant AND status IN ('pendente', 'parcial');

    SELECT COALESCE(SUM(valor_original - valor_pago), 0) INTO cp30
      FROM proj_contas_pagar
     WHERE tenant_id = p_tenant AND status IN ('pendente', 'parcial')
       AND vencimento <= NOW() + INTERVAL '30 days';

    SELECT CASE WHEN SUM(receita_centavos) > 0
                THEN 100.0 * SUM(margem_centavos) / SUM(receita_centavos) END
      INTO margem_pct
      FROM bi.fato_vendas_item
     WHERE tenant_id = p_tenant AND status = 'confirmada'
       AND data_venda >= CURRENT_DATE - 30;

    -- "Parado" = 90+ dias sem vender; quem nunca vendeu só conta se o cadastro
    -- também tem 90+ dias (senão todo tenant recém-migrado nasceria nota 0).
    SELECT COALESCE(SUM(ap.valor_imobilizado), 0),
           COALESCE(SUM(ap.valor_imobilizado) FILTER (
               WHERE ap.dias_sem_venda >= 90
                  OR (ap.dias_sem_venda IS NULL AND p.criado_em < NOW() - INTERVAL '90 days')), 0)
      INTO estoque_total, estoque_parado
      FROM bi.analise_produtos ap
      JOIN proj_produtos p ON p.tenant_id = ap.tenant_id AND p.produto_id = ap.produto_id
     WHERE ap.tenant_id = p_tenant AND ap.quantidade > 0;

    -- 1. Caixa 30d (peso 25): saldo projetado ≥ 0 é nota cheia; déficit é
    --    penalizado proporcionalmente à receita mensal (déficit = 50% dela → 0).
    IF receita_30d > 0 OR cp30 > cr30 THEN
        nota := CASE
            WHEN cr30 >= cp30 THEN 100
            WHEN receita_30d <= 0 THEN 0
            ELSE GREATEST(0, 100 * (1 - (cp30 - cr30) / (0.5 * receita_30d)))
        END;
        comp := comp || jsonb_build_object('nome', 'Caixa (30 dias)', 'nota', ROUND(nota), 'peso', 25,
            'detalhe', CASE WHEN cr30 >= cp30 THEN 'o que entra cobre o que sai'
                            ELSE format('vai faltar %s nos próximos 30 dias', bi.fmt_reais((cp30 - cr30)::bigint)) END);
        soma_pesos := soma_pesos + 25; soma_notas := soma_notas + nota * 25;
    END IF;

    -- 2. Cobrança (peso 15): vencidas vs receita mensal (30% dela → 0).
    IF receita_30d > 0 THEN
        nota := GREATEST(0, 100 * (1 - vencidas / (0.30 * receita_30d)));
        comp := comp || jsonb_build_object('nome', 'Cobrança', 'nota', ROUND(nota), 'peso', 15,
            'detalhe', CASE WHEN vencidas = 0 THEN 'nenhum cliente em atraso'
                            ELSE format('%s vencidos aguardando cobrança', bi.fmt_reais(vencidas::bigint)) END);
        soma_pesos := soma_pesos + 15; soma_notas := soma_notas + nota * 15;
    END IF;

    -- 3. Margem de balcão do mês (peso 20): 28% = nota cheia. É a margem
    -- BRUTA (preço − custo do produto) — dela ainda saem fixos e taxas; o
    -- rótulo evita confundir com a "sobra final" das Configurações.
    IF margem_pct IS NOT NULL THEN
        nota := LEAST(100, GREATEST(0, 100 * margem_pct / 28.0));
        comp := comp || jsonb_build_object('nome', 'Margem de balcão', 'nota', ROUND(nota), 'peso', 20,
            'detalhe', format('as vendas arrecadam R$ %s a cada R$ 100 para pagar as contas (referência: R$ 28+)', ROUND(margem_pct)));
        soma_pesos := soma_pesos + 20; soma_notas := soma_notas + nota * 20;
    END IF;

    -- 4. Giro de estoque (peso 20): % do capital parado 90+ dias (50% → 0).
    IF estoque_total > 0 THEN
        nota := GREATEST(0, 100 * (1 - (estoque_parado / estoque_total) / 0.5));
        comp := comp || jsonb_build_object('nome', 'Giro de estoque', 'nota', ROUND(nota), 'peso', 20,
            'detalhe', format('%s%% do estoque sem vender há 90+ dias', ROUND(100 * estoque_parado / estoque_total)));
        soma_pesos := soma_pesos + 20; soma_notas := soma_notas + nota * 20;
    END IF;

    -- 5. Tendência de vendas (peso 20): 30d vs 30d anteriores (metade → 0).
    IF receita_ant > 0 THEN
        nota := LEAST(100, GREATEST(0, 100 * ((receita_30d / receita_ant) - 0.5) / 0.5));
        comp := comp || jsonb_build_object('nome', 'Tendência de vendas', 'nota', ROUND(nota), 'peso', 20,
            'detalhe', CASE WHEN receita_30d >= receita_ant
                            THEN format('vendendo %s%% a mais que no período anterior', ROUND(100 * (receita_30d / receita_ant - 1)))
                            ELSE format('vendendo %s%% a menos que no período anterior', ROUND(100 * (1 - receita_30d / receita_ant))) END);
        soma_pesos := soma_pesos + 20; soma_notas := soma_notas + nota * 20;
    END IF;

    -- 6. Rumo à meta do mês (peso 15): compara o realizado com a fração da
    --    meta esperada até hoje (proporcional aos dias corridos do mês).
    IF meta IS NOT NULL AND meta > 0 THEN
        esperado_ate_hoje := meta * EXTRACT(day FROM NOW())
            / EXTRACT(day FROM (date_trunc('month', NOW()) + INTERVAL '1 month - 1 day'));
        nota := LEAST(100, GREATEST(0, 100 * receita_mes / esperado_ate_hoje));
        comp := comp || jsonb_build_object('nome', 'Rumo à meta do mês', 'nota', ROUND(nota), 'peso', 15,
            'detalhe', format('%s vendidos de %s da meta (%s%% do mês já passou)',
                              bi.fmt_reais(receita_mes::bigint), bi.fmt_reais(meta::bigint),
                              ROUND(100 * EXTRACT(day FROM NOW())
                                  / EXTRACT(day FROM (date_trunc('month', NOW()) + INTERVAL '1 month - 1 day')))));
        soma_pesos := soma_pesos + 15; soma_notas := soma_notas + nota * 15;
    END IF;

    IF soma_pesos = 0 THEN
        RETURN jsonb_build_object('score', NULL, 'componentes', comp);
    END IF;
    RETURN jsonb_build_object('score', ROUND(soma_notas / soma_pesos), 'componentes', comp);
END $$;

-- ── Orquestrador (chamado pelo job do backend a cada ciclo) ───────────────────

CREATE OR REPLACE FUNCTION bi.executar_etl() RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER SET search_path = bi, public AS $$
DECLARE resultado JSONB;
BEGIN
    PERFORM bi.etl_dim_produto();
    resultado := jsonb_build_object(
        'vendas',    bi.etl_vendas(),
        'contas',    bi.etl_financeiro(),
        'orcamentos', bi.etl_orcamentos(),
        'estoque',   bi.snapshot_estoque(),
        'analise_produtos', bi.calcular_analise_produtos(),
        'analise_clientes', bi.calcular_analise_clientes(),
        'alertas',   bi.avaliar_alertas()
    );
    RETURN resultado;
END $$;

-- ── Grants ────────────────────────────────────────────────────────────────────

REVOKE ALL ON FUNCTION bi.executar_etl() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.avaliar_alertas() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.etl_dim_produto() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.etl_vendas() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.etl_financeiro() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.etl_orcamentos() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.snapshot_estoque() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.calcular_analise_produtos() FROM PUBLIC;
REVOKE ALL ON FUNCTION bi.calcular_analise_clientes() FROM PUBLIC;

DO $$ BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'finledger') THEN
        GRANT USAGE ON SCHEMA bi TO finledger;
        GRANT SELECT ON bi.dim_tempo, bi.dim_produto, bi.fato_vendas_item,
                        bi.fato_orcamentos, bi.fato_contas_receber, bi.fato_contas_pagar,
                        bi.fato_estoque_snapshot, bi.regras_alerta,
                        bi.analise_produtos, bi.analise_clientes TO finledger;
        -- Feedback do gestor (Resolvido/Ignorar) e outbox do EventBus.
        GRANT SELECT, UPDATE ON bi.alertas TO finledger;
        GRANT INSERT ON bi.eventos_outbox TO finledger;
        GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA bi TO finledger;
        GRANT EXECUTE ON FUNCTION bi.executar_etl() TO finledger;
        GRANT EXECUTE ON FUNCTION bi.fmt_reais(NUMERIC) TO finledger;
    END IF;
END $$;
