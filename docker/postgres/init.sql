-- ── Usuário de aplicação ──────────────────────────────────────────────────────
-- 'postgres' = superusuário do Docker (POSTGRES_USER no docker-compose).
-- 'finledger'  = role da aplicação: sem SUPERUSER, sem DDL, apenas DML.
-- A RLS é efetivada pela aplicação: o pool grava `app.tenant_id` na conexão a cada
-- checkout (bootstrap/database.rs) e as policies abaixo filtram por essa GUC.
-- Por isso o role NÃO tem BYPASSRLS — o isolamento é enforçado pelo banco.
CREATE ROLE finledger LOGIN PASSWORD 'finledger';
GRANT CONNECT ON DATABASE finledger TO finledger;
\c finledger
GRANT USAGE ON SCHEMA public TO finledger;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO finledger;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO finledger;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO finledger;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO finledger;

-- ── Control plane ─────────────────────────────────────────────────────────────

-- Usuários do backoffice (superadmin + admins de suporte). Sem RLS — acesso cross-tenant.
CREATE TABLE backoffice_users (
    user_id       UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    username      VARCHAR(50)  NOT NULL UNIQUE,
    password_hash TEXT         NOT NULL,
    role          VARCHAR(16)  NOT NULL DEFAULT 'admin'
                      CHECK (role IN ('superadmin', 'admin')),
    permissions   TEXT[]       NOT NULL DEFAULT '{}',
    ativo         BOOLEAN      NOT NULL DEFAULT TRUE,
    criado_em     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE tenants (
    tenant_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    slug       VARCHAR(63) NOT NULL UNIQUE,
    nome       TEXT        NOT NULL,
    status     VARCHAR(10) NOT NULL DEFAULT 'ativo'
                   CHECK (status IN ('ativo', 'suspenso')),
    plano      VARCHAR(32) NOT NULL DEFAULT 'basico',
    -- Feature flag self-service (configurável pelo admin do próprio tenant,
    -- não pelo backoffice): permite adicionar itens a orçamentos acima do
    -- saldo em estoque. TRUE preserva o comportamento histórico do sistema.
    permite_orcamento_sem_estoque BOOLEAN NOT NULL DEFAULT TRUE,
    -- Dados da empresa (self-service, configurável pelo admin do próprio
    -- tenant): exibidos nas impressões de venda/orçamento. Todos opcionais —
    -- ausência apenas omite a linha correspondente no recibo.
    cnpj                    TEXT,
    telefone                TEXT,
    endereco                TEXT,
    chave_pix               TEXT,
    informacoes_adicionais  TEXT,
    -- Precificação assistida (self-service): percentuais em basis points
    -- (1 = 0,01%, ex. 4000 = 40%) — inteiro em vez de float, mesma lógica dos
    -- centavos. Todos opcionais; sem margem configurada não há sugestão.
    margem_padrao_bps            INTEGER,
    imposto_venda_bps            INTEGER,
    comissao_venda_bps           INTEGER,
    taxa_cartao_bps              INTEGER,
    frete_venda_bps              INTEGER,
    outras_despesas_venda_bps    INTEGER,
    -- Custos fixos mensais estimados e vendas/mês esperadas: rateiam custo
    -- fixo por unidade na sugestão de preço e calculam o ponto de equilíbrio.
    custos_fixos_mensais_centavos BIGINT,
    vendas_mensais_estimadas      INTEGER,
    criado_em  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Event store (pharos-rs) ───────────────────────────────────────────────────
-- pharos_aggregates: repositório não-tenant usado pela lib pharos-postgres.
CREATE TABLE pharos_aggregates (
    aggregate_type TEXT        NOT NULL,
    aggregate_id   TEXT        NOT NULL,
    payload        JSONB       NOT NULL,
    version        BIGINT      NOT NULL,
    updated_at     TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (aggregate_type, aggregate_id)
);
CREATE INDEX idx_pharos_aggregates_type_updated_at
    ON pharos_aggregates (aggregate_type, updated_at);

-- pharos_tenant_aggregates: snapshot store com isolamento por tenant.
-- 'version' é controle de concorrência otimista, não sequência de eventos.
CREATE TABLE pharos_tenant_aggregates (
    tenant_id      UUID        NOT NULL,
    aggregate_type TEXT        NOT NULL,
    aggregate_id   UUID        NOT NULL,
    payload        JSONB       NOT NULL,
    version        BIGINT      NOT NULL,
    updated_at     TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, aggregate_type, aggregate_id)
);
CREATE INDEX idx_pharos_tenant_aggregates_type_updated_at
    ON pharos_tenant_aggregates (tenant_id, aggregate_type, updated_at);

ALTER TABLE pharos_tenant_aggregates ENABLE ROW LEVEL SECURITY;
ALTER TABLE pharos_tenant_aggregates FORCE  ROW LEVEL SECURITY;
CREATE POLICY rls_pharos ON pharos_tenant_aggregates
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

-- ── Projeções CQRS ───────────────────────────────────────────────────────────

CREATE TABLE proj_produtos (
    produto_id    UUID        NOT NULL,
    tenant_id     UUID        NOT NULL,
    sku           VARCHAR(50) NOT NULL,
    descricao     TEXT        NOT NULL,
    ncm           VARCHAR(10) NOT NULL,
    unidade       VARCHAR(6)  NOT NULL,
    preco_custo   BIGINT      NOT NULL,
    preco_venda   BIGINT      NOT NULL,
    categoria     TEXT        NOT NULL,
    marca         TEXT,
    ativo         BOOLEAN     NOT NULL DEFAULT TRUE,
    -- FALSE para serviços/itens que não têm saldo de estoque (ex.: mão de
    -- obra) — produtos assim ficam de fora da checagem de disponibilidade em
    -- vendas/orçamentos (ver AdicionarItemVenda/AdicionarItemOrcamento).
    controla_estoque BOOLEAN  NOT NULL DEFAULT TRUE,
    criado_em     TIMESTAMPTZ NOT NULL,
    atualizado_em TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, produto_id)
);
CREATE UNIQUE INDEX idx_proj_produtos_sku ON proj_produtos(tenant_id, sku);

-- ── Precificação assistida (configuração simples, não event-sourced) ─────────

-- Override de margem (e opcionalmente de custo fixo por unidade) por
-- categoria. Categoria é texto livre — casa por igualdade exata com
-- proj_produtos.categoria.
CREATE TABLE categoria_margens (
    tenant_id                    UUID    NOT NULL REFERENCES tenants(tenant_id),
    categoria                    TEXT    NOT NULL,
    margem_bps                   INTEGER NOT NULL,
    custo_fixo_unitario_centavos BIGINT,
    PRIMARY KEY (tenant_id, categoria)
);

-- Overrides de precificação direto no produto (prevalecem sobre categoria e
-- padrão da loja). Tabela de referência simples — não mexe no agregado
-- Produto. Linha existe se qualquer campo estiver definido.
CREATE TABLE produto_precificacao (
    tenant_id                    UUID    NOT NULL,
    produto_id                   UUID    NOT NULL,
    margem_bps                   INTEGER,
    custo_fixo_unitario_centavos BIGINT,
    frete_venda_bps              INTEGER,
    PRIMARY KEY (tenant_id, produto_id)
);

-- Máquinas de cartão com suas taxas. A sugestão de preço usa a MAIOR taxa
-- cadastrada (conservador: o lucro fecha mesmo se a venda cair na máquina
-- mais cara); sem máquinas, cai no taxa_cartao_bps único do tenant.
CREATE TABLE maquinas_cartao (
    tenant_id UUID    NOT NULL REFERENCES tenants(tenant_id),
    nome      TEXT    NOT NULL,
    taxa_bps  INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, nome)
);

-- Frete típico de compra por fornecedor (% sobre o valor da mercadoria) —
-- pré-preenche o campo "frete da remessa" na entrada de estoque. Config de
-- precificação (por isso mora aqui e não no agregado Fornecedor).
CREATE TABLE fornecedor_frete (
    tenant_id       UUID    NOT NULL,
    fornecedor_id   UUID    NOT NULL,
    frete_tipico_bps INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, fornecedor_id)
);

-- Preços vistos na concorrência, anotados manualmente pelo gestor.
CREATE TABLE precos_concorrencia (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      UUID        NOT NULL,
    produto_id     UUID        NOT NULL,
    concorrente    TEXT,
    preco_centavos BIGINT      NOT NULL,
    observado_em   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_precos_concorrencia_produto
    ON precos_concorrencia(tenant_id, produto_id, observado_em DESC);

-- Histórico de preço de venda (uma linha por mudança), para o cálculo de
-- elasticidade de demanda. Projeção alimentada pelos eventos
-- ProdutoCadastrado/PrecosAtualizados do catálogo.
CREATE TABLE proj_historico_precos (
    tenant_id           UUID        NOT NULL,
    produto_id          UUID        NOT NULL,
    preco_venda_centavos BIGINT     NOT NULL,
    vigente_desde       TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, produto_id, vigente_desde)
);

CREATE TABLE proj_clientes (
    cliente_id    UUID        NOT NULL,
    tenant_id     UUID        NOT NULL,
    nome          TEXT        NOT NULL,
    cpf_cnpj      VARCHAR(18) NOT NULL,
    telefone      VARCHAR(20),
    email         VARCHAR(254),
    bloqueado     BOOLEAN     NOT NULL DEFAULT FALSE,
    ativo         BOOLEAN     NOT NULL DEFAULT TRUE,
    criado_em     TIMESTAMPTZ NOT NULL,
    atualizado_em TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, cliente_id)
);
CREATE UNIQUE INDEX idx_proj_clientes_cpf_cnpj ON proj_clientes(tenant_id, cpf_cnpj);

CREATE TABLE proj_fornecedores (
    fornecedor_id        UUID        NOT NULL,
    tenant_id            UUID        NOT NULL,
    razao_social         TEXT        NOT NULL,
    cnpj                 VARCHAR(18) NOT NULL,
    telefone             VARCHAR(20),
    email                VARCHAR(254),
    prazo_pagamento_dias INTEGER     NOT NULL DEFAULT 0,
    ativo                BOOLEAN     NOT NULL DEFAULT TRUE,
    criado_em            TIMESTAMPTZ NOT NULL,
    atualizado_em        TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, fornecedor_id)
);
CREATE UNIQUE INDEX idx_proj_fornecedores_cnpj ON proj_fornecedores(tenant_id, cnpj);

CREATE TABLE proj_saldo_estoque (
    produto_id     UUID        NOT NULL,
    tenant_id      UUID        NOT NULL,
    quantidade     INTEGER     NOT NULL DEFAULT 0,
    custo_medio    BIGINT      NOT NULL DEFAULT 0,
    estoque_minimo INTEGER     NOT NULL DEFAULT 0,
    atualizado_em  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, produto_id)
);

CREATE TABLE proj_vendas (
    venda_id        UUID        NOT NULL,
    tenant_id       UUID        NOT NULL,
    vendedor_id     UUID        NOT NULL,
    cliente_id      UUID,
    total_centavos  BIGINT      NOT NULL DEFAULT 0,
    status          VARCHAR(16) NOT NULL
                        CHECK (status IN ('iniciada', 'confirmada', 'cancelada')),
    forma_pagamento TEXT,
    criada_em       TIMESTAMPTZ NOT NULL,
    confirmada_em   TIMESTAMPTZ,
    atualizado_em   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, venda_id)
);
CREATE INDEX idx_proj_vendas_tenant ON proj_vendas(tenant_id);

CREATE TABLE proj_vendas_itens (
    item_id                 UUID        NOT NULL,
    tenant_id               UUID        NOT NULL,
    venda_id                UUID        NOT NULL,
    produto_id              UUID        NOT NULL,
    sku                     VARCHAR(50) NOT NULL,
    descricao               TEXT        NOT NULL,
    quantidade              INTEGER     NOT NULL,
    preco_unitario_centavos BIGINT      NOT NULL,
    PRIMARY KEY (tenant_id, item_id),
    FOREIGN KEY (tenant_id, venda_id) REFERENCES proj_vendas(tenant_id, venda_id) ON DELETE CASCADE
);
CREATE INDEX idx_proj_vendas_itens_venda ON proj_vendas_itens(tenant_id, venda_id);

CREATE TABLE proj_orcamentos (
    orcamento_id      UUID        NOT NULL,
    tenant_id         UUID        NOT NULL,
    vendedor_id       UUID        NOT NULL,
    cliente_id        UUID,
    cliente_avulso    TEXT,
    total_centavos    BIGINT      NOT NULL DEFAULT 0,
    desconto_centavos BIGINT      NOT NULL DEFAULT 0,
    status            VARCHAR(16) NOT NULL
                          CHECK (status IN ('rascunho', 'emitido', 'aceito', 'recusado', 'expirado', 'convertido', 'cancelado')),
    validade_dias     INTEGER     NOT NULL,
    venda_id          UUID,
    criado_em         TIMESTAMPTZ NOT NULL,
    atualizado_em     TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, orcamento_id)
);
CREATE INDEX idx_proj_orcamentos_tenant ON proj_orcamentos(tenant_id);

CREATE TABLE proj_orcamentos_itens (
    item_id                 UUID        NOT NULL,
    tenant_id               UUID        NOT NULL,
    orcamento_id            UUID        NOT NULL,
    produto_id              UUID        NOT NULL,
    sku                     VARCHAR(50) NOT NULL,
    descricao               TEXT        NOT NULL,
    quantidade              INTEGER     NOT NULL,
    preco_unitario_centavos BIGINT      NOT NULL,
    PRIMARY KEY (tenant_id, item_id),
    FOREIGN KEY (tenant_id, orcamento_id) REFERENCES proj_orcamentos(tenant_id, orcamento_id) ON DELETE CASCADE
);
CREATE INDEX idx_proj_orcamentos_itens ON proj_orcamentos_itens(tenant_id, orcamento_id);

CREATE TABLE proj_pedidos_compra (
    pedido_id            UUID        NOT NULL,
    tenant_id            UUID        NOT NULL,
    comprador_id         UUID        NOT NULL,
    fornecedor_id        UUID        NOT NULL,
    total_centavos       BIGINT      NOT NULL DEFAULT 0,
    prazo_pagamento_dias INTEGER     NOT NULL,
    status               VARCHAR(20) NOT NULL
                             CHECK (status IN ('gerado', 'aprovado', 'enviado', 'recebido_parcial', 'recebido_total', 'cancelado')),
    criado_em            TIMESTAMPTZ NOT NULL,
    atualizado_em        TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, pedido_id)
);
CREATE INDEX idx_proj_pedidos_compra_tenant ON proj_pedidos_compra(tenant_id);

CREATE TABLE proj_pedidos_compra_itens (
    id                      BIGSERIAL   PRIMARY KEY,
    tenant_id               UUID        NOT NULL,
    pedido_id               UUID        NOT NULL,
    produto_id              UUID        NOT NULL,
    quantidade              INTEGER     NOT NULL,
    custo_unitario_centavos BIGINT      NOT NULL,
    FOREIGN KEY (tenant_id, pedido_id) REFERENCES proj_pedidos_compra(tenant_id, pedido_id) ON DELETE CASCADE
);
CREATE INDEX idx_proj_pedidos_compra_itens ON proj_pedidos_compra_itens(tenant_id, pedido_id);

CREATE TABLE proj_contas_receber (
    conta_id       UUID        NOT NULL,
    tenant_id      UUID        NOT NULL,
    venda_id       UUID        NOT NULL,
    cliente_id     UUID,
    valor_original BIGINT      NOT NULL,
    valor_recebido BIGINT      NOT NULL DEFAULT 0,
    status         VARCHAR(16) NOT NULL
                       CHECK (status IN ('pendente', 'parcial', 'liquidada', 'estornada')),
    vencimento     TIMESTAMPTZ NOT NULL,
    criada_em      TIMESTAMPTZ NOT NULL,
    atualizado_em  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, conta_id)
);
CREATE INDEX idx_proj_cr_tenant     ON proj_contas_receber(tenant_id);
CREATE INDEX idx_proj_cr_vencimento ON proj_contas_receber(vencimento);
CREATE INDEX idx_proj_cr_status     ON proj_contas_receber(status);

CREATE TABLE proj_contas_pagar (
    conta_id       UUID        NOT NULL,
    tenant_id      UUID        NOT NULL,
    pedido_id      UUID        NOT NULL,
    fornecedor_id  UUID        NOT NULL,
    valor_original BIGINT      NOT NULL,
    valor_pago     BIGINT      NOT NULL DEFAULT 0,
    status         VARCHAR(16) NOT NULL
                       CHECK (status IN ('pendente', 'parcial', 'liquidada')),
    vencimento     TIMESTAMPTZ NOT NULL,
    criada_em      TIMESTAMPTZ NOT NULL,
    atualizado_em  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, conta_id)
);
CREATE INDEX idx_proj_cp_tenant     ON proj_contas_pagar(tenant_id);
CREATE INDEX idx_proj_cp_vencimento ON proj_contas_pagar(vencimento);
CREATE INDEX idx_proj_cp_status     ON proj_contas_pagar(status);

CREATE TABLE proj_usuarios (
    usuario_id    UUID         NOT NULL,
    tenant_id     UUID         NOT NULL,
    username      VARCHAR(50)  NOT NULL,
    password_hash VARCHAR(100) NOT NULL,
    roles         VARCHAR(500) NOT NULL DEFAULT '',
    ativo         BOOLEAN      NOT NULL DEFAULT TRUE,
    criado_em     TIMESTAMPTZ  NOT NULL,
    PRIMARY KEY (tenant_id, usuario_id)
);
CREATE UNIQUE INDEX idx_proj_usuarios_tenant_username ON proj_usuarios(tenant_id, username);

CREATE TABLE proj_notas_fiscais (
    nf_id           UUID        NOT NULL,
    tenant_id       UUID        NOT NULL,
    venda_id        UUID        NOT NULL,
    cliente_id      UUID,
    modelo          VARCHAR(2)  NOT NULL,
    serie           VARCHAR(3)  NOT NULL,
    numero          INTEGER     NOT NULL,
    chave           VARCHAR(44),
    protocolo       VARCHAR(20),
    status          VARCHAR(16) NOT NULL
                        CHECK (status IN ('gerada', 'transmitida', 'autorizada', 'rejeitada', 'cancelada')),
    total_centavos  BIGINT      NOT NULL DEFAULT 0,
    rejeicao_codigo VARCHAR(6),
    rejeicao_motivo TEXT,
    -- Devolução antes da integração SEFAZ ativa: cancelamento aguarda operação.
    cancelamento_pendente BOOLEAN NOT NULL DEFAULT FALSE,
    gerada_em       TIMESTAMPTZ NOT NULL,
    autorizada_em   TIMESTAMPTZ,
    cancelada_em    TIMESTAMPTZ,
    atualizado_em   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, nf_id)
);
CREATE INDEX idx_proj_nf_tenant ON proj_notas_fiscais(tenant_id);
CREATE INDEX idx_proj_nf_venda  ON proj_notas_fiscais(tenant_id, venda_id);
CREATE INDEX idx_proj_nf_status ON proj_notas_fiscais(status);
CREATE UNIQUE INDEX idx_proj_nf_chave  ON proj_notas_fiscais(tenant_id, chave) WHERE chave IS NOT NULL;
CREATE UNIQUE INDEX idx_proj_nf_numero ON proj_notas_fiscais(tenant_id, modelo, serie, numero);

-- ── RLS: isolamento por tenant ────────────────────────────────────────────────
-- USING: leitura/deleção — só vê linhas do próprio tenant.
-- WITH CHECK: escrita (INSERT/UPDATE) — só pode gravar no próprio tenant.

ALTER TABLE proj_produtos             ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_produtos             FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_clientes             ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_clientes             FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_fornecedores         ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_fornecedores         FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_saldo_estoque        ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_saldo_estoque        FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_vendas               ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_vendas               FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_vendas_itens         ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_vendas_itens         FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_orcamentos           ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_orcamentos           FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_orcamentos_itens     ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_orcamentos_itens     FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_pedidos_compra       ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_pedidos_compra       FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_pedidos_compra_itens ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_pedidos_compra_itens FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_contas_receber       ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_contas_receber       FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_contas_pagar         ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_contas_pagar         FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_notas_fiscais        ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_notas_fiscais        FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_usuarios             ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_usuarios             FORCE  ROW LEVEL SECURITY;

CREATE POLICY rls_proj_produtos ON proj_produtos
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_clientes ON proj_clientes
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_fornecedores ON proj_fornecedores
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_saldo_estoque ON proj_saldo_estoque
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_vendas ON proj_vendas
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_vendas_itens ON proj_vendas_itens
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_orcamentos ON proj_orcamentos
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_orcamentos_itens ON proj_orcamentos_itens
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_pedidos_compra ON proj_pedidos_compra
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_pedidos_compra_itens ON proj_pedidos_compra_itens
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_contas_receber ON proj_contas_receber
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_contas_pagar ON proj_contas_pagar
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_notas_fiscais ON proj_notas_fiscais
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_usuarios ON proj_usuarios
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

ALTER TABLE categoria_margens             ENABLE ROW LEVEL SECURITY;
ALTER TABLE categoria_margens             FORCE  ROW LEVEL SECURITY;
ALTER TABLE produto_precificacao          ENABLE ROW LEVEL SECURITY;
ALTER TABLE produto_precificacao          FORCE  ROW LEVEL SECURITY;
ALTER TABLE maquinas_cartao               ENABLE ROW LEVEL SECURITY;
ALTER TABLE maquinas_cartao               FORCE  ROW LEVEL SECURITY;
ALTER TABLE fornecedor_frete              ENABLE ROW LEVEL SECURITY;
ALTER TABLE fornecedor_frete              FORCE  ROW LEVEL SECURITY;
ALTER TABLE precos_concorrencia           ENABLE ROW LEVEL SECURITY;
ALTER TABLE precos_concorrencia           FORCE  ROW LEVEL SECURITY;
ALTER TABLE proj_historico_precos         ENABLE ROW LEVEL SECURITY;
ALTER TABLE proj_historico_precos         FORCE  ROW LEVEL SECURITY;

CREATE POLICY rls_categoria_margens ON categoria_margens
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_produto_precificacao ON produto_precificacao
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_maquinas_cartao ON maquinas_cartao
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_fornecedor_frete ON fornecedor_frete
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_precos_concorrencia ON precos_concorrencia
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY rls_proj_historico_precos ON proj_historico_precos
    USING      (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

-- ── Backoffice: cross-tenant revenue aggregates ───────────────────────────────
-- proj_vendas has FORCE RLS and the app role can't bypass it, so the backoffice
-- reads revenue through SECURITY DEFINER functions owned by the superuser that
-- ran this script. Only these two aggregates are exposed — never raw rows.

CREATE FUNCTION backoffice_revenue_by_tenant()
RETURNS TABLE (
    tenant_id        UUID,
    total_cents      BIGINT,
    sales_count      BIGINT,
    last_30d_cents   BIGINT,
    last_30d_count   BIGINT,
    prev_30d_cents   BIGINT
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = public
AS $$
    SELECT v.tenant_id,
           COALESCE(SUM(v.total_centavos), 0)::BIGINT,
           COUNT(*)::BIGINT,
           COALESCE(SUM(v.total_centavos)
               FILTER (WHERE v.confirmada_em >= NOW() - INTERVAL '30 days'), 0)::BIGINT,
           COUNT(*) FILTER (WHERE v.confirmada_em >= NOW() - INTERVAL '30 days')::BIGINT,
           COALESCE(SUM(v.total_centavos)
               FILTER (WHERE v.confirmada_em >= NOW() - INTERVAL '60 days'
                         AND v.confirmada_em <  NOW() - INTERVAL '30 days'), 0)::BIGINT
    FROM proj_vendas v
    WHERE v.status = 'confirmada'
    GROUP BY v.tenant_id
$$;

CREATE FUNCTION backoffice_revenue_monthly(months INT DEFAULT 12)
RETURNS TABLE (
    month        DATE,
    total_cents  BIGINT,
    sales_count  BIGINT
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = public
AS $$
    SELECT date_trunc('month', v.confirmada_em)::date,
           COALESCE(SUM(v.total_centavos), 0)::BIGINT,
           COUNT(*)::BIGINT
    FROM proj_vendas v
    WHERE v.status = 'confirmada'
      AND v.confirmada_em >= date_trunc('month', NOW()) - make_interval(months => months - 1)
    GROUP BY 1
    ORDER BY 1
$$;

CREATE FUNCTION backoffice_revenue_monthly_by_tenant(months INT DEFAULT 12)
RETURNS TABLE (
    tenant_id    UUID,
    month        DATE,
    total_cents  BIGINT
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = public
AS $$
    SELECT v.tenant_id,
           date_trunc('month', v.confirmada_em)::date,
           COALESCE(SUM(v.total_centavos), 0)::BIGINT
    FROM proj_vendas v
    WHERE v.status = 'confirmada'
      AND v.confirmada_em >= date_trunc('month', NOW()) - make_interval(months => months - 1)
    GROUP BY 1, 2
    ORDER BY 1, 2
$$;

CREATE FUNCTION backoffice_revenue_daily(days INT DEFAULT 30)
RETURNS TABLE (
    day          DATE,
    total_cents  BIGINT,
    sales_count  BIGINT
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = public
AS $$
    SELECT date_trunc('day', v.confirmada_em)::date,
           COALESCE(SUM(v.total_centavos), 0)::BIGINT,
           COUNT(*)::BIGINT
    FROM proj_vendas v
    WHERE v.status = 'confirmada'
      AND v.confirmada_em >= date_trunc('day', NOW()) - make_interval(days => days - 1)
    GROUP BY 1
    ORDER BY 1
$$;

-- Platform-wide operational counters for the backoffice overview.
CREATE FUNCTION backoffice_platform_stats()
RETURNS TABLE (
    total_users     BIGINT,
    active_users    BIGINT,
    total_products  BIGINT,
    total_clients   BIGINT,
    today_cents     BIGINT,
    today_count     BIGINT
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = public
AS $$
    SELECT (SELECT COUNT(*) FROM proj_usuarios),
           (SELECT COUNT(*) FROM proj_usuarios WHERE ativo),
           (SELECT COUNT(*) FROM proj_produtos WHERE ativo),
           (SELECT COUNT(*) FROM proj_clientes WHERE ativo),
           (SELECT COALESCE(SUM(total_centavos), 0)::BIGINT FROM proj_vendas
             WHERE status = 'confirmada' AND confirmada_em >= date_trunc('day', NOW())),
           (SELECT COUNT(*)::BIGINT FROM proj_vendas
             WHERE status = 'confirmada' AND confirmada_em >= date_trunc('day', NOW()))
$$;

-- Lock the definers down to the app role (the role only exists in the Docker
-- image, not in test containers — hence the conditional grant).
REVOKE ALL ON FUNCTION backoffice_revenue_by_tenant() FROM PUBLIC;
REVOKE ALL ON FUNCTION backoffice_revenue_monthly(INT) FROM PUBLIC;
REVOKE ALL ON FUNCTION backoffice_revenue_monthly_by_tenant(INT) FROM PUBLIC;
REVOKE ALL ON FUNCTION backoffice_revenue_daily(INT) FROM PUBLIC;
REVOKE ALL ON FUNCTION backoffice_platform_stats() FROM PUBLIC;
DO $$ BEGIN
    IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'finledger') THEN
        GRANT EXECUTE ON FUNCTION backoffice_revenue_by_tenant() TO finledger;
        GRANT EXECUTE ON FUNCTION backoffice_revenue_monthly(INT) TO finledger;
        GRANT EXECUTE ON FUNCTION backoffice_revenue_monthly_by_tenant(INT) TO finledger;
        GRANT EXECUTE ON FUNCTION backoffice_revenue_daily(INT) TO finledger;
        GRANT EXECUTE ON FUNCTION backoffice_platform_stats() TO finledger;
    END IF;
END $$;

-- Dados de homologação: seed_demo.sql roda sozinho quando montado em
-- /docker-entrypoint-initdb.d (ordem alfabética, após este arquivo). Não usar
-- \i aqui — em produção o seed não é montado e o include abortaria o initdb.
