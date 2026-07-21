-- ─────────────────────────────────────────────────────────────────────────────
-- SEED DE HOMOLOGAÇÃO — Empresa Demo Finledger
--
-- Execução: psql -U postgres -d finledger -f seed_demo.sql
-- Ou: docker exec -i <container> psql -U postgres -d finledger -f /seed_demo.sql
--
-- Senhas:
--   admin / carlos.vendedor / lucia.financeiro / rafael.estoque /
--   patricia.compradora / marcos.fiscal / bianca.gerente
--   → "admin" (para o usuário admin) e "demo" para os demais
--
-- O script é idempotente: usa ON CONFLICT DO NOTHING em toda inserção.
-- ─────────────────────────────────────────────────────────────────────────────

\set ON_ERROR_STOP on

BEGIN;

-- ── 0. IDs fixos (facilitam debug e re-execução idempotente) ─────────────────

\set TENANT_DEMO        'a0000000-0000-0000-0000-000000000001'

-- Usuários do tenant
\set USR_ADMIN          'b0000000-0000-0000-0000-000000000001'
\set USR_CARLOS         'b0000000-0000-0000-0000-000000000002'
\set USR_LUCIA          'b0000000-0000-0000-0000-000000000003'
\set USR_RAFAEL         'b0000000-0000-0000-0000-000000000004'
\set USR_PATRICIA       'b0000000-0000-0000-0000-000000000005'
\set USR_MARCOS         'b0000000-0000-0000-0000-000000000006'
\set USR_BIANCA         'b0000000-0000-0000-0000-000000000007'

-- Produtos
\set P_FILTRO_OLEO      'c0000000-0000-0000-0000-000000000001'
\set P_FILTRO_AR        'c0000000-0000-0000-0000-000000000002'
\set P_FILTRO_COMB      'c0000000-0000-0000-0000-000000000003'
\set P_FILTRO_CABINE    'c0000000-0000-0000-0000-000000000004'
\set P_VELA_STD         'c0000000-0000-0000-0000-000000000005'
\set P_VELA_IRIDIUM     'c0000000-0000-0000-0000-000000000006'
\set P_CORREIA_KIT      'c0000000-0000-0000-0000-000000000007'
\set P_PAST_DIANT       'c0000000-0000-0000-0000-000000000008'
\set P_PAST_TRAS        'c0000000-0000-0000-0000-000000000009'
\set P_DISCO_DIANT      'c0000000-0000-0000-0000-000000000010'
\set P_AMORT_DIANT      'c0000000-0000-0000-0000-000000000011'
\set P_AMORT_TRAS       'c0000000-0000-0000-0000-000000000012'
\set P_OLEO_5W30        'c0000000-0000-0000-0000-000000000013'
\set P_OLEO_10W40       'c0000000-0000-0000-0000-000000000014'
\set P_BATERIA_60       'c0000000-0000-0000-0000-000000000015'
\set P_BATERIA_45       'c0000000-0000-0000-0000-000000000016'
\set P_LAMP_H4          'c0000000-0000-0000-0000-000000000017'
\set P_LAMP_LED         'c0000000-0000-0000-0000-000000000018'
\set P_LIMPADOR         'c0000000-0000-0000-0000-000000000019'
\set P_REVISAO          'c0000000-0000-0000-0000-000000000020'

-- Clientes
\set CLI_JOAO           'd0000000-0000-0000-0000-000000000001'
\set CLI_MARIA          'd0000000-0000-0000-0000-000000000002'
\set CLI_CARLOS         'd0000000-0000-0000-0000-000000000003'
\set CLI_TRANSPORTES    'd0000000-0000-0000-0000-000000000004'
\set CLI_ANA            'd0000000-0000-0000-0000-000000000005'
\set CLI_RICARDO        'd0000000-0000-0000-0000-000000000006'
\set CLI_FERNANDA       'd0000000-0000-0000-0000-000000000007'
\set CLI_AUTOPECAS      'd0000000-0000-0000-0000-000000000008'
\set CLI_PAULO          'd0000000-0000-0000-0000-000000000009'
\set CLI_TATIANA        'd0000000-0000-0000-0000-000000000010'

-- Fornecedores
\set FORN_BOSCH         'e0000000-0000-0000-0000-000000000001'
\set FORN_MONROE        'e0000000-0000-0000-0000-000000000002'
\set FORN_NGK           'e0000000-0000-0000-0000-000000000003'
\set FORN_MOURA         'e0000000-0000-0000-0000-000000000004'
\set FORN_CENTRAL       'e0000000-0000-0000-0000-000000000005'

-- Vendas (8 confirmadas, 1 em andamento, 1 cancelada)
\set V1 'f0000000-0000-0000-0000-000000000001'
\set V2 'f0000000-0000-0000-0000-000000000002'
\set V3 'f0000000-0000-0000-0000-000000000003'
\set V4 'f0000000-0000-0000-0000-000000000004'
\set V5 'f0000000-0000-0000-0000-000000000005'
\set V6 'f0000000-0000-0000-0000-000000000006'
\set V7 'f0000000-0000-0000-0000-000000000007'
\set V8 'f0000000-0000-0000-0000-000000000008'
\set V9  'f0000000-0000-0000-0000-000000000009'
\set V10 'f0000000-0000-0000-0000-000000000010'

-- Orçamentos (aceito / emitido / recusado / rascunho)
\set ORC1 'f1000000-0000-0000-0000-000000000001'
\set ORC2 'f1000000-0000-0000-0000-000000000002'
\set ORC3 'f1000000-0000-0000-0000-000000000003'
\set ORC4 'f1000000-0000-0000-0000-000000000004'

-- Pedidos de Compra (recebido / enviado / gerado)
\set PC1 'f2000000-0000-0000-0000-000000000001'
\set PC2 'f2000000-0000-0000-0000-000000000002'
\set PC3 'f2000000-0000-0000-0000-000000000003'

-- Contas a Receber
\set CR1 'f3000000-0000-0000-0000-000000000001'
\set CR2 'f3000000-0000-0000-0000-000000000002'
\set CR3 'f3000000-0000-0000-0000-000000000003'

-- Contas a Pagar
\set CP1 'f4000000-0000-0000-0000-000000000001'
\set CP2 'f4000000-0000-0000-0000-000000000002'

-- Notas Fiscais
\set NF1 'f5000000-0000-0000-0000-000000000001'
\set NF2 'f5000000-0000-0000-0000-000000000002'
\set NF3 'f5000000-0000-0000-0000-000000000003'
\set NF4 'f5000000-0000-0000-0000-000000000004'
\set NF5 'f5000000-0000-0000-0000-000000000005'
\set NF6 'f5000000-0000-0000-0000-000000000006'
\set NF7 'f5000000-0000-0000-0000-000000000007'
\set NF8 'f5000000-0000-0000-0000-000000000008'

-- Hashes Argon2id (gerados com m=19456, t=2, p=1)
-- Senha "admin"
\set HASH_ADMIN '$argon2id$v=19$m=19456,t=2,p=1$aUR+9WYRBqr8HJhcUDRT0g$49WmyeGoWgOXqR13jnPzUsPRzDoav1xDHHBBXhIbmFI'
-- Senha "demo"
\set HASH_DEMO  '$argon2id$v=19$m=19456,t=2,p=1$ZLc72HCFVuKgvgEsy3c1QA$jbkiZ3tSCxsIo8O4MGcZ1x0JpIlDff5vAMfGSj/eBUc'

-- ── 1. Tenant ─────────────────────────────────────────────────────────────────

INSERT INTO tenants (tenant_id, slug, nome, status, plano, criado_em)
VALUES (:'TENANT_DEMO', 'demo', 'Empresa Finledger Demo', 'ativo', 'profissional', NOW() - INTERVAL '90 days')
ON CONFLICT (slug) DO UPDATE
  SET tenant_id = :'TENANT_DEMO',
      nome      = 'Empresa Finledger Demo',
      plano     = 'profissional';

-- ── 2. Usuários do tenant ─────────────────────────────────────────────────────

INSERT INTO proj_usuarios (usuario_id, tenant_id, username, password_hash, roles, ativo, criado_em) VALUES
  (:'USR_ADMIN',    :'TENANT_DEMO', 'admin',               :'HASH_ADMIN', 'admin',                 TRUE, NOW() - INTERVAL '90 days'),
  (:'USR_CARLOS',   :'TENANT_DEMO', 'carlos.vendedor',     :'HASH_DEMO',  'vendedor',              TRUE, NOW() - INTERVAL '85 days'),
  (:'USR_LUCIA',    :'TENANT_DEMO', 'lucia.financeiro',    :'HASH_DEMO',  'financeiro',            TRUE, NOW() - INTERVAL '85 days'),
  (:'USR_RAFAEL',   :'TENANT_DEMO', 'rafael.estoque',      :'HASH_DEMO',  'estoquista',            TRUE, NOW() - INTERVAL '80 days'),
  (:'USR_PATRICIA', :'TENANT_DEMO', 'patricia.compradora', :'HASH_DEMO',  'comprador',             TRUE, NOW() - INTERVAL '80 days'),
  (:'USR_MARCOS',   :'TENANT_DEMO', 'marcos.fiscal',       :'HASH_DEMO',  'fiscal',                TRUE, NOW() - INTERVAL '75 days'),
  (:'USR_BIANCA',   :'TENANT_DEMO', 'bianca.gerente',      :'HASH_DEMO',  'admin,vendedor',        TRUE, NOW() - INTERVAL '70 days')
ON CONFLICT (tenant_id, usuario_id) DO NOTHING;

-- ── 3. Produtos ───────────────────────────────────────────────────────────────

INSERT INTO proj_produtos (produto_id, tenant_id, sku, descricao, ncm, unidade, preco_custo, preco_venda, categoria, ativo, criado_em, atualizado_em) VALUES
  (:'P_FILTRO_OLEO',   :'TENANT_DEMO', 'INFO-MOUSE-001',    'Mouse sem Fio Logitech M170',              '84716053', 'UN', 1890,  3490,  'Informática',       TRUE, NOW()-INTERVAL '89d', NOW()-INTERVAL '1d'),
  (:'P_FILTRO_AR',     :'TENANT_DEMO', 'INFO-TEC-001',      'Teclado USB Multilaser TC-085',             '84716052', 'UN', 1250,  2490,  'Informática',       TRUE, NOW()-INTERVAL '89d', NOW()-INTERVAL '1d'),
  (:'P_FILTRO_COMB',   :'TENANT_DEMO', 'INFO-WEBCAM-001',   'Webcam Full HD Logitech C920',              '85258090', 'UN', 2100,  3990,  'Informática',       TRUE, NOW()-INTERVAL '89d', NOW()-INTERVAL '1d'),
  (:'P_FILTRO_CABINE', :'TENANT_DEMO', 'INFO-HEADSET-001',  'Headset USB Multilaser PH219',              '85183000', 'UN', 1800,  3290,  'Informática',       TRUE, NOW()-INTERVAL '89d', NOW()-INTERVAL '1d'),
  (:'P_VELA_STD',      :'TENANT_DEMO', 'PAPEL-CANETA-001',  'Kit Canetas Esferográficas Bic (jogo 4)',   '96081000', 'JG', 1490,  3290,  'Papelaria',         TRUE, NOW()-INTERVAL '88d', NOW()-INTERVAL '1d'),
  (:'P_VELA_IRIDIUM',  :'TENANT_DEMO', 'PAPEL-MARCADOR-001','Kit Marcadores Faber-Castell (jogo 4)',     '96082000', 'JG', 3200,  6490,  'Papelaria',         TRUE, NOW()-INTERVAL '88d', NOW()-INTERVAL '1d'),
  (:'P_CORREIA_KIT',   :'TENANT_DEMO', 'PAPEL-KIT-001',     'Kit Organizador de Mesa + Porta-Documentos','39269090', 'KT',18500, 32900,  'Papelaria',         TRUE, NOW()-INTERVAL '88d', NOW()-INTERVAL '1d'),
  (:'P_PAST_DIANT',    :'TENANT_DEMO', 'LIMP-PANO-001',     'Kit Panos de Microfibra Multiuso',          '63071090', 'JG', 2890,  5490,  'Limpeza',           TRUE, NOW()-INTERVAL '87d', NOW()-INTERVAL '1d'),
  (:'P_PAST_TRAS',     :'TENANT_DEMO', 'LIMP-ESPONJA-001',  'Kit Esponjas Multiuso',                     '39241000', 'JG', 2100,  4290,  'Limpeza',           TRUE, NOW()-INTERVAL '87d', NOW()-INTERVAL '1d'),
  (:'P_DISCO_DIANT',   :'TENANT_DEMO', 'LIMP-LUVA-001',     'Par de Luvas de Limpeza Profissional',      '40151900', 'PR', 6800, 12900,  'Limpeza',           TRUE, NOW()-INTERVAL '87d', NOW()-INTERVAL '1d'),
  (:'P_AMORT_DIANT',   :'TENANT_DEMO', 'MOV-CADEIRA-001',   'Cadeira de Escritório Ergonômica',          '94013000', 'UN',12000, 22900,  'Móveis',            TRUE, NOW()-INTERVAL '86d', NOW()-INTERVAL '1d'),
  (:'P_AMORT_TRAS',    :'TENANT_DEMO', 'MOV-MESA-001',      'Mesa Dobrável para Escritório',             '94033000', 'UN', 9800, 18900,  'Móveis',            TRUE, NOW()-INTERVAL '86d', NOW()-INTERVAL '1d'),
  (:'P_OLEO_5W30',     :'TENANT_DEMO', 'BEB-SUCO-001',      'Suco de Laranja Natural 1L',                '20097900', 'LT', 1890,  3690,  'Bebidas',           TRUE, NOW()-INTERVAL '85d', NOW()-INTERVAL '1d'),
  (:'P_OLEO_10W40',    :'TENANT_DEMO', 'BEB-AGUA-001',      'Água Mineral sem Gás 1L',                   '22011000', 'LT', 1490,  2890,  'Bebidas',           TRUE, NOW()-INTERVAL '85d', NOW()-INTERVAL '1d'),
  (:'P_BATERIA_60',    :'TENANT_DEMO', 'ELET-POWERBANK-001','Power Bank 20000mAh',                       '85076000', 'UN',31000, 48900,  'Eletroeletrônicos', TRUE, NOW()-INTERVAL '84d', NOW()-INTERVAL '1d'),
  (:'P_BATERIA_45',    :'TENANT_DEMO', 'ELET-POWERBANK-002','Power Bank 10000mAh',                       '85076000', 'UN',22000, 35900,  'Eletroeletrônicos', TRUE, NOW()-INTERVAL '84d', NOW()-INTERVAL '1d'),
  (:'P_LAMP_H4',       :'TENANT_DEMO', 'ELET-PILHA-001',    'Par de Pilhas Recarregáveis AA',            '85068000', 'PR',  890,  1990,  'Eletroeletrônicos', TRUE, NOW()-INTERVAL '83d', NOW()-INTERVAL '1d'),
  (:'P_LAMP_LED',      :'TENANT_DEMO', 'ELET-LAMPLED-001',  'Par de Lâmpadas LED Inteligentes Wi-Fi',    '85395000', 'PR', 3200,  5990,  'Eletroeletrônicos', TRUE, NOW()-INTERVAL '83d', NOW()-INTERVAL '1d'),
  (:'P_LIMPADOR',      :'TENANT_DEMO', 'ACESS-ORG-001',     'Organizador de Cabos de Mesa',              '39269090', 'UN', 1490,  2990,  'Acessórios',        TRUE, NOW()-INTERVAL '82d', NOW()-INTERVAL '1d'),
  (:'P_REVISAO',       :'TENANT_DEMO', 'SERV-INSTALACAO-001','Instalação e Configuração Técnica',        '00000000', 'SV', 9000, 18900,  'Serviços',          TRUE, NOW()-INTERVAL '82d', NOW()-INTERVAL '1d')
ON CONFLICT (tenant_id, produto_id) DO NOTHING;

-- Serviços (unidade 'SV') não têm saldo de estoque físico — ficam de fora da
-- checagem de disponibilidade em vendas/orçamentos.
UPDATE proj_produtos SET controla_estoque = FALSE
 WHERE tenant_id = :'TENANT_DEMO' AND unidade = 'SV';

-- ── 4. Fornecedores ───────────────────────────────────────────────────────────

INSERT INTO proj_fornecedores (fornecedor_id, tenant_id, razao_social, cnpj, telefone, email, prazo_pagamento_dias, ativo, criado_em, atualizado_em) VALUES
  (:'FORN_BOSCH',   :'TENANT_DEMO', 'TechDistribuidora Brasil Ltda',      '60.655.783/0001-52', '(11) 4234-5678', 'contato@techdistribuidora.com.br',   30, TRUE, NOW()-INTERVAL '88d', NOW()-INTERVAL '88d'),
  (:'FORN_MONROE',  :'TENANT_DEMO', 'MobiliaCorp Equipamentos S.A.',      '51.472.005/0001-17', '(11) 3456-7890', 'vendas@mobiliacorp.com.br',          45, TRUE, NOW()-INTERVAL '88d', NOW()-INTERVAL '88d'),
  (:'FORN_NGK',     :'TENANT_DEMO', 'Papelaria Industrial Brasil Ltda',   '61.797.924/0001-08', '(11) 2345-6789', 'contato@papelariaindustrial.com.br', 30, TRUE, NOW()-INTERVAL '87d', NOW()-INTERVAL '87d'),
  (:'FORN_MOURA',   :'TENANT_DEMO', 'PowerTech Baterias S.A.',            '08.367.739/0001-44', '(83) 3234-5678', 'comercial@powertech.com.br',         60, TRUE, NOW()-INTERVAL '87d', NOW()-INTERVAL '87d'),
  (:'FORN_CENTRAL', :'TENANT_DEMO', 'Distribuidora Central Ltda',         '15.789.345/0001-22', '(11) 9876-5432', 'compras@distribuidoracentral.com.br',15, TRUE, NOW()-INTERVAL '86d', NOW()-INTERVAL '86d')
ON CONFLICT (tenant_id, fornecedor_id) DO NOTHING;

-- ── 5. Clientes ───────────────────────────────────────────────────────────────

INSERT INTO proj_clientes (cliente_id, tenant_id, nome, cpf_cnpj, telefone, email, bloqueado, criado_em, atualizado_em) VALUES
  (:'CLI_JOAO',        :'TENANT_DEMO', 'João Silva Santos',        '123.456.789-00', '(11) 98765-4321', 'joao.silva@email.com',           FALSE, NOW()-INTERVAL '85d', NOW()-INTERVAL '85d'),
  (:'CLI_MARIA',       :'TENANT_DEMO', 'Maria Oliveira Costa',     '234.567.890-01', '(11) 97654-3210', 'maria.oliveira@email.com',       FALSE, NOW()-INTERVAL '84d', NOW()-INTERVAL '84d'),
  (:'CLI_CARLOS',      :'TENANT_DEMO', 'Carlos Eduardo Pereira',   '345.678.901-02', '(11) 96543-2109', NULL,                             FALSE, NOW()-INTERVAL '83d', NOW()-INTERVAL '83d'),
  (:'CLI_TRANSPORTES', :'TENANT_DEMO', 'Transportes Unidos Ltda',  '12.345.678/0001-99', '(11) 3333-4444', 'frota@transunidos.com.br',   FALSE, NOW()-INTERVAL '82d', NOW()-INTERVAL '82d'),
  (:'CLI_ANA',         :'TENANT_DEMO', 'Ana Paula Rodrigues',      '456.789.012-03', '(11) 95432-1098', 'ana.paula@gmail.com',            FALSE, NOW()-INTERVAL '81d', NOW()-INTERVAL '81d'),
  (:'CLI_RICARDO',     :'TENANT_DEMO', 'Ricardo Martins Alves',    '567.890.123-04', '(11) 94321-0987', NULL,                             FALSE, NOW()-INTERVAL '80d', NOW()-INTERVAL '80d'),
  (:'CLI_FERNANDA',    :'TENANT_DEMO', 'Fernanda Lima Souza',      '678.901.234-05', '(11) 93210-9876', 'fernanda.lima@hotmail.com',      FALSE, NOW()-INTERVAL '79d', NOW()-INTERVAL '79d'),
  (:'CLI_AUTOPECAS',   :'TENANT_DEMO', 'Comércio Rápido ME',       '23.456.789/0001-88', '(11) 3222-1111', 'compras@comerciorapido.com.br', FALSE, NOW()-INTERVAL '78d', NOW()-INTERVAL '78d'),
  (:'CLI_PAULO',       :'TENANT_DEMO', 'Paulo Roberto Ferreira',   '789.012.345-06', '(11) 92109-8765', NULL,                             TRUE,  NOW()-INTERVAL '77d', NOW()-INTERVAL '10d'),
  (:'CLI_TATIANA',     :'TENANT_DEMO', 'Tatiana Mendes Castro',    '890.123.456-07', '(11) 91098-7654', 'tatiana.mendes@email.com',       FALSE, NOW()-INTERVAL '76d', NOW()-INTERVAL '76d')
ON CONFLICT (tenant_id, cliente_id) DO NOTHING;

-- ── 6. Saldo de Estoque ───────────────────────────────────────────────────────

INSERT INTO proj_saldo_estoque (produto_id, tenant_id, quantidade, custo_medio, atualizado_em) VALUES
  (:'P_FILTRO_OLEO',   :'TENANT_DEMO',  42, 1890, NOW()-INTERVAL '3d'),
  (:'P_FILTRO_AR',     :'TENANT_DEMO',  36, 1250, NOW()-INTERVAL '3d'),
  (:'P_FILTRO_COMB',   :'TENANT_DEMO',  28, 2100, NOW()-INTERVAL '3d'),
  (:'P_FILTRO_CABINE', :'TENANT_DEMO',  22, 1800, NOW()-INTERVAL '5d'),
  (:'P_VELA_STD',      :'TENANT_DEMO',  55, 1490, NOW()-INTERVAL '2d'),
  (:'P_VELA_IRIDIUM',  :'TENANT_DEMO',  18, 3200, NOW()-INTERVAL '2d'),
  (:'P_CORREIA_KIT',   :'TENANT_DEMO',  12, 18500, NOW()-INTERVAL '7d'),
  (:'P_PAST_DIANT',    :'TENANT_DEMO',  38, 2890, NOW()-INTERVAL '1d'),
  (:'P_PAST_TRAS',     :'TENANT_DEMO',  27, 2100, NOW()-INTERVAL '1d'),
  (:'P_DISCO_DIANT',   :'TENANT_DEMO',  10, 6800, NOW()-INTERVAL '4d'),
  (:'P_AMORT_DIANT',   :'TENANT_DEMO',   6, 12000, NOW()-INTERVAL '4d'),
  (:'P_AMORT_TRAS',    :'TENANT_DEMO',   8, 9800, NOW()-INTERVAL '4d'),
  (:'P_OLEO_5W30',     :'TENANT_DEMO',  72, 1890, NOW()-INTERVAL '1d'),
  (:'P_OLEO_10W40',    :'TENANT_DEMO',  68, 1490, NOW()-INTERVAL '1d'),
  (:'P_BATERIA_60',    :'TENANT_DEMO',   5, 31000, NOW()-INTERVAL '6d'),
  (:'P_BATERIA_45',    :'TENANT_DEMO',   7, 22000, NOW()-INTERVAL '6d'),
  (:'P_LAMP_H4',       :'TENANT_DEMO',  25, 890, NOW()-INTERVAL '8d'),
  (:'P_LAMP_LED',      :'TENANT_DEMO',  14, 3200, NOW()-INTERVAL '8d'),
  (:'P_LIMPADOR',      :'TENANT_DEMO',  18, 1490, NOW()-INTERVAL '9d'),
  (:'P_REVISAO',       :'TENANT_DEMO',   0, 9000, NOW()-INTERVAL '10d')
ON CONFLICT (tenant_id, produto_id) DO NOTHING;

-- Agregados de estoque (pharos): sem eles, BaixarEstoque falha em "não
-- encontrado" e vendas confirmadas na demo nunca decrementam o saldo. Deriva
-- os snapshots direto da projeção (mesmo formato que o repositório grava).
INSERT INTO pharos_tenant_aggregates (tenant_id, aggregate_type, aggregate_id, payload, version, updated_at)
SELECT s.tenant_id, 'ItemEstoque', s.produto_id,
       jsonb_build_object(
           'id', s.produto_id, 'saldo', s.quantidade, 'version', 1,
           'produto_id', s.produto_id, 'estoque_minimo', s.estoque_minimo),
       1, NOW()
  FROM proj_saldo_estoque s
 WHERE s.tenant_id = :'TENANT_DEMO'
ON CONFLICT (tenant_id, aggregate_type, aggregate_id) DO NOTHING;

-- ── 7. Vendas (8 confirmadas + 1 em andamento + 1 cancelada) ─────────────────

INSERT INTO proj_vendas (venda_id, tenant_id, vendedor_id, cliente_id, total_centavos, status, forma_pagamento, criada_em, confirmada_em, atualizado_em) VALUES
  -- V1: Instalação técnica + acessórios de informática, João, Pix (30 dias atrás)
  (:'V1', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_JOAO',       31160, 'confirmada', 'Pix',             NOW()-INTERVAL '30d', NOW()-INTERVAL '30d', NOW()-INTERVAL '30d'),
  -- V2: Kit de limpeza, Maria, débito (25 dias atrás)
  (:'V2', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_MARIA',      22670, 'confirmada', 'CartaoDebito',    NOW()-INTERVAL '25d', NOW()-INTERVAL '25d', NOW()-INTERVAL '25d'),
  -- V3: Cadeiras e mesas de escritório (4un), Transportes Unidos, crédito 3x (20 dias)
  (:'V3', :'TENANT_DEMO', :'USR_BIANCA', :'CLI_TRANSPORTES',163600, 'confirmada', '{"CartaoCredito":{"parcelas":3}}', NOW()-INTERVAL '20d', NOW()-INTERVAL '20d', NOW()-INTERVAL '20d'),
  -- V4: Power bank + pilhas recarregáveis, consumidor final, dinheiro (18 dias)
  (:'V4', :'TENANT_DEMO', :'USR_CARLOS', NULL,              50880, 'confirmada', 'Dinheiro',        NOW()-INTERVAL '18d', NOW()-INTERVAL '18d', NOW()-INTERVAL '18d'),
  -- V5: Kit instalação completo, Ana, prazo 30 dias (15 dias atrás — ainda dentro do prazo)
  (:'V5', :'TENANT_DEMO', :'USR_BIANCA', :'CLI_ANA',        34420, 'confirmada', '{"Prazo":{"dias":30}}', NOW()-INTERVAL '15d', NOW()-INTERVAL '15d', NOW()-INTERVAL '15d'),
  -- V6: Kit organizador + marcadores premium, Comércio Rápido, Pix (10 dias)
  (:'V6', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_AUTOPECAS',  39390, 'confirmada', 'Pix',             NOW()-INTERVAL '10d', NOW()-INTERVAL '10d', NOW()-INTERVAL '10d'),
  -- V7: Power bank 10000mAh, Fernanda, crédito 2x (5 dias)
  (:'V7', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_FERNANDA',   35900, 'confirmada', '{"CartaoCredito":{"parcelas":2}}', NOW()-INTERVAL '5d', NOW()-INTERVAL '5d', NOW()-INTERVAL '5d'),
  -- V8: Organizadores + lâmpadas LED, consumidor final, dinheiro (ontem)
  (:'V8', :'TENANT_DEMO', :'USR_CARLOS', NULL,              11970, 'confirmada', 'Dinheiro',        NOW()-INTERVAL '1d', NOW()-INTERVAL '1d', NOW()-INTERVAL '1d'),
  -- V9: Em andamento (hoje) — cliente Ricardo
  (:'V9', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_RICARDO',        0, 'iniciada',   NULL,              NOW()-INTERVAL '2h', NULL,                 NOW()-INTERVAL '2h'),
  -- V10: Cancelada (produto não disponível) — 40 dias atrás
  (:'V10', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_PAULO',         0, 'cancelada',  NULL,              NOW()-INTERVAL '40d', NULL,                NOW()-INTERVAL '40d')
ON CONFLICT (tenant_id, venda_id) DO NOTHING;

-- Itens das vendas confirmadas
INSERT INTO proj_vendas_itens (item_id, tenant_id, venda_id, produto_id, sku, descricao, quantidade, preco_unitario_centavos) VALUES
  -- V1
  (md5(:'TENANT_DEMO' || :'V1' || :'P_FILTRO_OLEO')::uuid, :'TENANT_DEMO', :'V1', :'P_FILTRO_OLEO',   'INFO-MOUSE-001',    'Mouse sem Fio Logitech M170',            1, 3490),
  (md5(:'TENANT_DEMO' || :'V1' || :'P_FILTRO_AR')::uuid, :'TENANT_DEMO', :'V1', :'P_FILTRO_AR',     'INFO-TEC-001',      'Teclado USB Multilaser TC-085',          1, 2490),
  (md5(:'TENANT_DEMO' || :'V1' || :'P_OLEO_5W30')::uuid, :'TENANT_DEMO', :'V1', :'P_OLEO_5W30',     'BEB-SUCO-001',      'Suco de Laranja Natural 1L',             4, 3690),
  (md5(:'TENANT_DEMO' || :'V1' || :'P_LIMPADOR')::uuid, :'TENANT_DEMO', :'V1', :'P_LIMPADOR',      'ACESS-ORG-001',     'Organizador de Cabos de Mesa',           1, 2990),
  -- V2
  (md5(:'TENANT_DEMO' || :'V2' || :'P_PAST_DIANT')::uuid, :'TENANT_DEMO', :'V2', :'P_PAST_DIANT',    'LIMP-PANO-001',     'Kit Panos de Microfibra Multiuso',       1, 5490),
  (md5(:'TENANT_DEMO' || :'V2' || :'P_PAST_TRAS')::uuid, :'TENANT_DEMO', :'V2', :'P_PAST_TRAS',     'LIMP-ESPONJA-001',  'Kit Esponjas Multiuso',                  1, 4290),
  (md5(:'TENANT_DEMO' || :'V2' || :'P_DISCO_DIANT')::uuid, :'TENANT_DEMO', :'V2', :'P_DISCO_DIANT',   'LIMP-LUVA-001',     'Par de Luvas de Limpeza Profissional',   1,12900),
  -- V3
  (md5(:'TENANT_DEMO' || :'V3' || :'P_AMORT_DIANT')::uuid, :'TENANT_DEMO', :'V3', :'P_AMORT_DIANT',   'MOV-CADEIRA-001',   'Cadeira de Escritório Ergonômica',       4,22900),
  (md5(:'TENANT_DEMO' || :'V3' || :'P_AMORT_TRAS')::uuid, :'TENANT_DEMO', :'V3', :'P_AMORT_TRAS',    'MOV-MESA-001',      'Mesa Dobrável para Escritório',          4,18900),
  -- V4
  (md5(:'TENANT_DEMO' || :'V4' || :'P_BATERIA_60')::uuid, :'TENANT_DEMO', :'V4', :'P_BATERIA_60',    'ELET-POWERBANK-001','Power Bank 20000mAh',                    1,48900),
  (md5(:'TENANT_DEMO' || :'V4' || :'P_LAMP_H4')::uuid, :'TENANT_DEMO', :'V4', :'P_LAMP_H4',       'ELET-PILHA-001',    'Par de Pilhas Recarregáveis AA',         1, 1990),
  -- V5
  (md5(:'TENANT_DEMO' || :'V5' || :'P_FILTRO_OLEO')::uuid, :'TENANT_DEMO', :'V5', :'P_FILTRO_OLEO',   'INFO-MOUSE-001',    'Mouse sem Fio Logitech M170',            1, 3490),
  (md5(:'TENANT_DEMO' || :'V5' || :'P_VELA_STD')::uuid, :'TENANT_DEMO', :'V5', :'P_VELA_STD',      'PAPEL-CANETA-001',  'Kit Canetas Esferográficas Bic (jogo 4)',1, 3290),
  (md5(:'TENANT_DEMO' || :'V5' || :'P_OLEO_5W30')::uuid, :'TENANT_DEMO', :'V5', :'P_OLEO_5W30',     'BEB-SUCO-001',      'Suco de Laranja Natural 1L',             4, 3690),
  (md5(:'TENANT_DEMO' || :'V5' || :'P_REVISAO')::uuid, :'TENANT_DEMO', :'V5', :'P_REVISAO',       'SERV-INSTALACAO-001','Instalação e Configuração Técnica',     1,18900),
  -- V6
  (md5(:'TENANT_DEMO' || :'V6' || :'P_CORREIA_KIT')::uuid, :'TENANT_DEMO', :'V6', :'P_CORREIA_KIT',   'PAPEL-KIT-001',     'Kit Organizador de Mesa + Porta-Documentos',1,32900),
  (md5(:'TENANT_DEMO' || :'V6' || :'P_VELA_IRIDIUM')::uuid, :'TENANT_DEMO', :'V6', :'P_VELA_IRIDIUM',  'PAPEL-MARCADOR-001','Kit Marcadores Faber-Castell (jogo 4)',  1, 6490),
  -- V7
  (md5(:'TENANT_DEMO' || :'V7' || :'P_BATERIA_45')::uuid, :'TENANT_DEMO', :'V7', :'P_BATERIA_45',    'ELET-POWERBANK-002','Power Bank 10000mAh',                    1,35900),
  -- V8
  (md5(:'TENANT_DEMO' || :'V8' || :'P_LIMPADOR')::uuid, :'TENANT_DEMO', :'V8', :'P_LIMPADOR',      'ACESS-ORG-001',     'Organizador de Cabos de Mesa',           2, 2990),
  (md5(:'TENANT_DEMO' || :'V8' || :'P_LAMP_LED')::uuid, :'TENANT_DEMO', :'V8', :'P_LAMP_LED',      'ELET-LAMPLED-001',  'Par de Lâmpadas LED Inteligentes Wi-Fi', 1, 5990),
  -- V9 (em andamento, sem itens ainda — simula operador abrindo nova venda)
  -- V10 (cancelada, sem itens)
  (md5(:'TENANT_DEMO' || :'V9' || :'P_FILTRO_OLEO')::uuid, :'TENANT_DEMO', :'V9', :'P_FILTRO_OLEO',   'INFO-MOUSE-001',    'Mouse sem Fio Logitech M170',            2, 3490)
ON CONFLICT DO NOTHING;

-- ── 8. Orçamentos ─────────────────────────────────────────────────────────────

INSERT INTO proj_orcamentos (orcamento_id, tenant_id, vendedor_id, cliente_id, total_centavos, desconto_centavos, status, validade_dias, criado_em, atualizado_em) VALUES
  (:'ORC1', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_RICARDO',   29440, 2000, 'aceito',  15, NOW()-INTERVAL '45d', NOW()-INTERVAL '43d'),
  (:'ORC2', :'TENANT_DEMO', :'USR_BIANCA', :'CLI_TRANSPORTES',163600, 0,  'emitido',  7, NOW()-INTERVAL '5d',  NOW()-INTERVAL '5d'),
  (:'ORC3', :'TENANT_DEMO', :'USR_CARLOS', :'CLI_CARLOS',    22680, 0,   'recusado', 5, NOW()-INTERVAL '12d', NOW()-INTERVAL '11d'),
  (:'ORC4', :'TENANT_DEMO', :'USR_BIANCA', :'CLI_TATIANA',   42880, 0,   'rascunho', 10, NOW()-INTERVAL '1d', NOW()-INTERVAL '1d')
ON CONFLICT (tenant_id, orcamento_id) DO NOTHING;

INSERT INTO proj_orcamentos_itens (item_id, tenant_id, orcamento_id, produto_id, sku, descricao, quantidade, preco_unitario_centavos) VALUES
  -- ORC1 (aceito): instalação técnica para Ricardo
  (md5(:'TENANT_DEMO' || :'ORC1' || :'P_FILTRO_OLEO')::uuid, :'TENANT_DEMO', :'ORC1', :'P_FILTRO_OLEO', 'INFO-MOUSE-001',   'Mouse sem Fio Logitech M170',              1, 3490),
  (md5(:'TENANT_DEMO' || :'ORC1' || :'P_VELA_STD')::uuid, :'TENANT_DEMO', :'ORC1', :'P_VELA_STD',    'PAPEL-CANETA-001', 'Kit Canetas Esferográficas Bic (jogo 4)',  1, 3290),
  (md5(:'TENANT_DEMO' || :'ORC1' || :'P_OLEO_5W30')::uuid, :'TENANT_DEMO', :'ORC1', :'P_OLEO_5W30',   'BEB-SUCO-001',     'Suco de Laranja Natural 1L',               4, 3690),
  (md5(:'TENANT_DEMO' || :'ORC1' || :'P_REVISAO')::uuid, :'TENANT_DEMO', :'ORC1', :'P_REVISAO',     'SERV-INSTALACAO-001','Instalação e Configuração Técnica',      1,18900),
  -- ORC2 (emitido, aguardando): cadeiras e mesas de escritório para a frota Transportes Unidos
  (md5(:'TENANT_DEMO' || :'ORC2' || :'P_AMORT_DIANT')::uuid, :'TENANT_DEMO', :'ORC2', :'P_AMORT_DIANT', 'MOV-CADEIRA-001',  'Cadeira de Escritório Ergonômica',        4,22900),
  (md5(:'TENANT_DEMO' || :'ORC2' || :'P_AMORT_TRAS')::uuid, :'TENANT_DEMO', :'ORC2', :'P_AMORT_TRAS',  'MOV-MESA-001',     'Mesa Dobrável para Escritório',           4,18900),
  -- ORC3 (recusado): kit de limpeza para Carlos
  (md5(:'TENANT_DEMO' || :'ORC3' || :'P_PAST_DIANT')::uuid, :'TENANT_DEMO', :'ORC3', :'P_PAST_DIANT',  'LIMP-PANO-001',    'Kit Panos de Microfibra Multiuso',        1, 5490),
  (md5(:'TENANT_DEMO' || :'ORC3' || :'P_PAST_TRAS')::uuid, :'TENANT_DEMO', :'ORC3', :'P_PAST_TRAS',   'LIMP-ESPONJA-001', 'Kit Esponjas Multiuso',                   1, 4290),
  (md5(:'TENANT_DEMO' || :'ORC3' || :'P_DISCO_DIANT')::uuid, :'TENANT_DEMO', :'ORC3', :'P_DISCO_DIANT', 'LIMP-LUVA-001',    'Par de Luvas de Limpeza Profissional',    1,12900),
  -- ORC4 (rascunho): power bank + lâmpadas LED para Tatiana
  (md5(:'TENANT_DEMO' || :'ORC4' || :'P_BATERIA_60')::uuid, :'TENANT_DEMO', :'ORC4', :'P_BATERIA_60',  'ELET-POWERBANK-001','Power Bank 20000mAh',                    1,48900),
  (md5(:'TENANT_DEMO' || :'ORC4' || :'P_BATERIA_45')::uuid, :'TENANT_DEMO', :'ORC4', :'P_BATERIA_45',  'ELET-POWERBANK-002','Power Bank 10000mAh',                    0,35900),  -- ainda escolhendo
  (md5(:'TENANT_DEMO' || :'ORC4' || :'P_LAMP_LED')::uuid, :'TENANT_DEMO', :'ORC4', :'P_LAMP_LED',    'ELET-LAMPLED-001', 'Par de Lâmpadas LED Inteligentes Wi-Fi',  2, 5990)
ON CONFLICT DO NOTHING;

-- Normaliza o total dos orçamentos: total_centavos é sempre LÍQUIDO
-- (itens − desconto), a mesma semântica mantida pela projeção.
UPDATE proj_orcamentos o
   SET total_centavos = (SELECT COALESCE(SUM(CAST(i.quantidade AS BIGINT) * i.preco_unitario_centavos), 0)
                           FROM proj_orcamentos_itens i
                          WHERE i.orcamento_id = o.orcamento_id AND i.tenant_id = o.tenant_id)
                        - o.desconto_centavos
 WHERE o.tenant_id = :'TENANT_DEMO';

-- ── 8.1 Agregados (event store) para clientes/vendas/orçamentos ──────────────
-- As tabelas proj_* acima são apenas o read-model. Comandos de edição/exclusão
-- (atualizar, desativar, cancelar, ...) carregam o agregado de
-- pharos_tenant_aggregates via aggregate_id — sem uma linha aqui o handler
-- retorna "entidade não encontrada" mesmo a linha existindo na projeção.
-- O payload é derivado das linhas já inseridas acima para não duplicar dados
-- (e possíveis erros de digitação) à mão.

INSERT INTO pharos_tenant_aggregates (tenant_id, aggregate_type, aggregate_id, payload, version, updated_at)
SELECT
    tenant_id,
    'Produto',
    produto_id,
    jsonb_build_object(
        'id', produto_id,
        'version', 1,
        'sku', sku,
        'descricao', descricao,
        'ncm', ncm,
        'unidade', unidade,
        'preco_custo', preco_custo,
        'preco_venda', preco_venda,
        'categoria', categoria,
        'marca', marca,
        'ativo', ativo,
        'controla_estoque', controla_estoque
    ),
    1,
    atualizado_em
FROM proj_produtos
WHERE tenant_id = :'TENANT_DEMO'
ON CONFLICT (tenant_id, aggregate_type, aggregate_id) DO NOTHING;

INSERT INTO pharos_tenant_aggregates (tenant_id, aggregate_type, aggregate_id, payload, version, updated_at)
SELECT
    tenant_id,
    'Cliente',
    cliente_id,
    jsonb_build_object(
        'id', cliente_id,
        'version', 1,
        'nome', nome,
        'cpf_cnpj', regexp_replace(cpf_cnpj, '\D', '', 'g'),
        'telefone', CASE WHEN telefone IS NULL THEN NULL ELSE regexp_replace(telefone, '\D', '', 'g') END,
        'email', lower(email),
        'ativo', ativo,
        'bloqueado', bloqueado
    ),
    1,
    atualizado_em
FROM proj_clientes
WHERE tenant_id = :'TENANT_DEMO'
ON CONFLICT (tenant_id, aggregate_type, aggregate_id) DO NOTHING;

INSERT INTO pharos_tenant_aggregates (tenant_id, aggregate_type, aggregate_id, payload, version, updated_at)
SELECT
    v.tenant_id,
    'Venda',
    v.venda_id,
    jsonb_build_object(
        'id', v.venda_id,
        'version', 1,
        'vendedor_id', v.vendedor_id,
        'cliente_id', v.cliente_id,
        'itens', COALESCE((
            SELECT jsonb_agg(jsonb_build_object(
                'item_id', vi.item_id,
                'produto_id', vi.produto_id,
                'sku', vi.sku,
                'descricao', vi.descricao,
                'quantidade', vi.quantidade,
                'preco_unitario_centavos', vi.preco_unitario_centavos
            ))
            FROM proj_vendas_itens vi
            WHERE vi.tenant_id = v.tenant_id AND vi.venda_id = v.venda_id
        ), '[]'::jsonb),
        'forma_pagamento', CASE
            WHEN v.forma_pagamento IS NULL THEN 'null'::jsonb
            WHEN left(v.forma_pagamento, 1) = '{' THEN v.forma_pagamento::jsonb
            ELSE to_jsonb(v.forma_pagamento)
        END,
        'status', CASE v.status
            WHEN 'iniciada' THEN 'Em Andamento'
            WHEN 'confirmada' THEN 'Confirmada'
            WHEN 'cancelada' THEN 'Cancelada'
        END
    ),
    1,
    v.atualizado_em
FROM proj_vendas v
WHERE v.tenant_id = :'TENANT_DEMO'
ON CONFLICT (tenant_id, aggregate_type, aggregate_id) DO NOTHING;

INSERT INTO pharos_tenant_aggregates (tenant_id, aggregate_type, aggregate_id, payload, version, updated_at)
SELECT
    o.tenant_id,
    'Orcamento',
    o.orcamento_id,
    jsonb_build_object(
        'id', o.orcamento_id,
        'version', 1,
        'vendedor_id', o.vendedor_id,
        'cliente_id', o.cliente_id,
        'itens', COALESCE((
            SELECT jsonb_agg(jsonb_build_object(
                'item_id', oi.item_id,
                'produto_id', oi.produto_id,
                'sku', oi.sku,
                'descricao', oi.descricao,
                'quantidade', oi.quantidade,
                'preco_unitario_centavos', oi.preco_unitario_centavos
            ))
            FROM proj_orcamentos_itens oi
            WHERE oi.tenant_id = o.tenant_id AND oi.orcamento_id = o.orcamento_id
        ), '[]'::jsonb),
        'desconto_centavos', o.desconto_centavos,
        'validade_dias', o.validade_dias,
        'status', CASE o.status
            WHEN 'rascunho' THEN 'Rascunho'
            WHEN 'emitido' THEN 'Emitido'
            WHEN 'aceito' THEN 'Aceito'
            WHEN 'recusado' THEN 'Recusado'
            WHEN 'expirado' THEN 'Expirado'
            WHEN 'convertido' THEN 'ConvertidoEmVenda'
            WHEN 'cancelado' THEN 'Cancelado'
        END
    ),
    1,
    o.atualizado_em
FROM proj_orcamentos o
WHERE o.tenant_id = :'TENANT_DEMO'
ON CONFLICT (tenant_id, aggregate_type, aggregate_id) DO NOTHING;

-- ── 9. Pedidos de Compra ─────────────────────────────────────────────────────

INSERT INTO proj_pedidos_compra (pedido_id, tenant_id, comprador_id, fornecedor_id, total_centavos, prazo_pagamento_dias, status, criado_em, atualizado_em) VALUES
  (:'PC1', :'TENANT_DEMO', :'USR_PATRICIA', :'FORN_BOSCH',   95930, 30, 'recebido_total', NOW()-INTERVAL '60d', NOW()-INTERVAL '50d'),
  (:'PC2', :'TENANT_DEMO', :'USR_PATRICIA', :'FORN_MONROE',  756000, 45, 'enviado', NOW()-INTERVAL '20d', NOW()-INTERVAL '18d'),
  (:'PC3', :'TENANT_DEMO', :'USR_PATRICIA', :'FORN_CENTRAL', 895900,  15, 'gerado',  NOW()-INTERVAL '2d', NOW()-INTERVAL '2d')
ON CONFLICT (tenant_id, pedido_id) DO NOTHING;

INSERT INTO proj_pedidos_compra_itens (tenant_id, pedido_id, produto_id, quantidade, custo_unitario_centavos) VALUES
  -- PC1 TechDistribuidora (recebido)
  (:'TENANT_DEMO', :'PC1', :'P_FILTRO_OLEO', 100, 1890),
  (:'TENANT_DEMO', :'PC1', :'P_FILTRO_AR',    80, 1250),
  (:'TENANT_DEMO', :'PC1', :'P_PAST_DIANT',   60, 2890),
  (:'TENANT_DEMO', :'PC1', :'P_PAST_TRAS',    50, 2100),
  -- PC2 MobiliaCorp (enviado, aguardando recebimento)
  (:'TENANT_DEMO', :'PC2', :'P_AMORT_DIANT',  20, 12000),
  (:'TENANT_DEMO', :'PC2', :'P_AMORT_TRAS',   20,  9800),
  -- PC3 Distribuidora Central (gerado, aguardando aprovação)
  (:'TENANT_DEMO', :'PC3', :'P_VELA_STD',    100,  1490),
  (:'TENANT_DEMO', :'PC3', :'P_VELA_IRIDIUM', 30,  3200),
  (:'TENANT_DEMO', :'PC3', :'P_OLEO_5W30',   150,  1890),
  (:'TENANT_DEMO', :'PC3', :'P_OLEO_10W40',  150,  1490),
  (:'TENANT_DEMO', :'PC3', :'P_BATERIA_60',   10, 31000),
  (:'TENANT_DEMO', :'PC3', :'P_BATERIA_45',   15, 22000)
ON CONFLICT DO NOTHING;

-- ── 10. Contas a Receber ──────────────────────────────────────────────────────
-- Geradas pelas vendas (apenas as que têm valor financeiro significativo).

INSERT INTO proj_contas_receber (conta_id, tenant_id, venda_id, cliente_id, valor_original, valor_recebido, status, vencimento, criada_em, atualizado_em) VALUES
  -- V1 Pix/João — liquidada imediatamente
  (:'CR2', :'TENANT_DEMO', :'V1', :'CLI_JOAO',        31160,  31160, 'liquidada', NOW()-INTERVAL '30d', NOW()-INTERVAL '30d', NOW()-INTERVAL '30d'),
  -- V3 Crédito 3x/Transportes — parcialmente recebida (1ª parcela)
  (:'CR3', :'TENANT_DEMO', :'V3', :'CLI_TRANSPORTES', 163600, 54534, 'parcial',   NOW()-INTERVAL '20d', NOW()-INTERVAL '20d', NOW()-INTERVAL '15d'),
  -- V5 Prazo 30d/Ana — pendente (vence em 15 dias)
  (:'CR1', :'TENANT_DEMO', :'V5', :'CLI_ANA',          34420,      0, 'pendente',  NOW()+INTERVAL '15d',  NOW()-INTERVAL '15d', NOW()-INTERVAL '15d'),
  -- V6 Pix — liquidada
  (md5(:'TENANT_DEMO' || :'V6' || :'CLI_AUTOPECAS')::uuid, :'TENANT_DEMO', :'V6', :'CLI_AUTOPECAS', 39390,  39390, 'liquidada', NOW()-INTERVAL '10d', NOW()-INTERVAL '10d', NOW()-INTERVAL '10d'),
  -- V7 Crédito 2x/Fernanda — pendente (parcela 2)
  (md5(:'TENANT_DEMO' || :'V7' || :'CLI_FERNANDA')::uuid, :'TENANT_DEMO', :'V7', :'CLI_FERNANDA',  35900,  17950, 'parcial',   NOW()+INTERVAL '25d',  NOW()-INTERVAL '5d',  NOW()-INTERVAL '5d')
ON CONFLICT (tenant_id, conta_id) DO NOTHING;

-- ── 11. Contas a Pagar ────────────────────────────────────────────────────────

INSERT INTO proj_contas_pagar (conta_id, tenant_id, pedido_id, fornecedor_id, valor_original, valor_pago, status, vencimento, criada_em, atualizado_em) VALUES
  -- PC1 TechDistribuidora (recebido há 50d, vencimento 30d, já pago)
  (:'CP1', :'TENANT_DEMO', :'PC1', :'FORN_BOSCH',   95930,  95930, 'liquidada', NOW()-INTERVAL '20d', NOW()-INTERVAL '50d', NOW()-INTERVAL '21d'),
  -- PC2 MobiliaCorp (enviado, vencimento 45d a partir do recebimento — ainda pendente)
  (:'CP2', :'TENANT_DEMO', :'PC2', :'FORN_MONROE', 756000,      0, 'pendente',  NOW()+INTERVAL '27d',  NOW()-INTERVAL '18d', NOW()-INTERVAL '18d')
ON CONFLICT (tenant_id, conta_id) DO NOTHING;

-- ── 12. Notas Fiscais ─────────────────────────────────────────────────────────
-- Geradas automaticamente ao confirmar venda (simuladas aqui).
-- Modelo 65 (NFC-e) para consumidor final; Modelo 55 (NF-e) para pessoa jurídica.

INSERT INTO proj_notas_fiscais (nf_id, tenant_id, venda_id, cliente_id, modelo, serie, numero, chave, protocolo, status, total_centavos, gerada_em, autorizada_em, atualizado_em) VALUES
  -- NF1 V1 João (CPF) → NFC-e autorizada
  (:'NF1', :'TENANT_DEMO', :'V1', :'CLI_JOAO',        '65', '001', 1,
   '35' || to_char(NOW()-INTERVAL '30d','YYMMDD') || '00000000000000000000000000001',
   '135240000000001', 'autorizada',  31160, NOW()-INTERVAL '30d', NOW()-INTERVAL '30d', NOW()-INTERVAL '30d'),
  -- NF2 V2 Maria → NFC-e autorizada
  (:'NF2', :'TENANT_DEMO', :'V2', :'CLI_MARIA',       '65', '001', 2,
   '35' || to_char(NOW()-INTERVAL '25d','YYMMDD') || '00000000000000000000000000002',
   '135240000000002', 'autorizada',  22670, NOW()-INTERVAL '25d', NOW()-INTERVAL '25d', NOW()-INTERVAL '25d'),
  -- NF3 V3 Transportes (CNPJ) → NF-e autorizada
  (:'NF3', :'TENANT_DEMO', :'V3', :'CLI_TRANSPORTES', '55', '001', 1,
   '35' || to_char(NOW()-INTERVAL '20d','YYMMDD') || '00000000000000000000000000003',
   '135240000000003', 'autorizada', 163600, NOW()-INTERVAL '20d', NOW()-INTERVAL '20d', NOW()-INTERVAL '20d'),
  -- NF4 V4 consumidor final → NFC-e autorizada
  (:'NF4', :'TENANT_DEMO', :'V4', NULL,               '65', '001', 3,
   '35' || to_char(NOW()-INTERVAL '18d','YYMMDD') || '00000000000000000000000000004',
   '135240000000004', 'autorizada',  50880, NOW()-INTERVAL '18d', NOW()-INTERVAL '18d', NOW()-INTERVAL '18d'),
  -- NF5 V5 Ana → NFC-e autorizada
  (:'NF5', :'TENANT_DEMO', :'V5', :'CLI_ANA',         '65', '001', 4,
   '35' || to_char(NOW()-INTERVAL '15d','YYMMDD') || '00000000000000000000000000005',
   '135240000000005', 'autorizada',  34420, NOW()-INTERVAL '15d', NOW()-INTERVAL '15d', NOW()-INTERVAL '15d'),
  -- NF6 V6 Comércio Rápido (CNPJ) → NF-e autorizada
  (:'NF6', :'TENANT_DEMO', :'V6', :'CLI_AUTOPECAS',   '55', '001', 2,
   '35' || to_char(NOW()-INTERVAL '10d','YYMMDD') || '00000000000000000000000000006',
   '135240000000006', 'autorizada',  39390, NOW()-INTERVAL '10d', NOW()-INTERVAL '10d', NOW()-INTERVAL '10d'),
  -- NF7 V7 Fernanda → NFC-e autorizada
  (:'NF7', :'TENANT_DEMO', :'V7', :'CLI_FERNANDA',    '65', '001', 5,
   '35' || to_char(NOW()-INTERVAL '5d','YYMMDD') || '00000000000000000000000000007',
   '135240000000007', 'autorizada',  35900, NOW()-INTERVAL '5d',  NOW()-INTERVAL '5d',  NOW()-INTERVAL '5d'),
  -- NF8 V8 consumidor final (ontem) → NFC-e autorizada
  (:'NF8', :'TENANT_DEMO', :'V8', NULL,               '65', '001', 6,
   '35' || to_char(NOW()-INTERVAL '1d','YYMMDD') || '00000000000000000000000000008',
   '135240000000008', 'autorizada',  11970, NOW()-INTERVAL '1d',  NOW()-INTERVAL '1d',  NOW()-INTERVAL '1d')
ON CONFLICT (tenant_id, nf_id) DO NOTHING;

-- ── Normaliza forma_pagamento para o rótulo humano exibido pela UI ───────────
-- Os INSERTs acima usam o formato do enum (necessário para reconstruir os
-- payloads dos eventos); a coluna de projeção, porém, guarda o rótulo de
-- exibição (Display de FormaPagamento em vendas/domain/value_objects.rs).
-- Idempotente: rótulos já normalizados caem no ELSE e ficam como estão.
UPDATE proj_vendas SET forma_pagamento = CASE
    WHEN forma_pagamento IN ('Dinheiro', '"Dinheiro"')         THEN 'Dinheiro'
    WHEN forma_pagamento IN ('Pix', '"Pix"')                   THEN 'Pix'
    WHEN forma_pagamento IN ('CartaoDebito', '"CartaoDebito"') THEN 'Cartão de débito'
    WHEN left(forma_pagamento, 1) = '{' AND forma_pagamento::jsonb ? 'CartaoCredito'
        THEN 'Cartão de crédito (' || (forma_pagamento::jsonb->'CartaoCredito'->>'parcelas') || 'x)'
    WHEN left(forma_pagamento, 1) = '{' AND forma_pagamento::jsonb ? 'Prazo'
        THEN 'A prazo (' || (forma_pagamento::jsonb->'Prazo'->>'dias') || ' dias)'
    ELSE forma_pagamento
END
WHERE forma_pagamento IS NOT NULL;

COMMIT;

-- ── Resumo ─────────────────────────────────────────────────────────────────────

DO $$
DECLARE
  tid UUID := 'a0000000-0000-0000-0000-000000000001';
BEGIN
  RAISE NOTICE '=== SEED DEMO concluído ===';
  RAISE NOTICE 'Tenant:       demo  (id: %)', tid;
  RAISE NOTICE 'Usuários:     %', (SELECT count(*) FROM proj_usuarios    WHERE tenant_id = tid);
  RAISE NOTICE 'Produtos:     %', (SELECT count(*) FROM proj_produtos     WHERE tenant_id = tid);
  RAISE NOTICE 'Clientes:     %', (SELECT count(*) FROM proj_clientes     WHERE tenant_id = tid);
  RAISE NOTICE 'Fornecedores: %', (SELECT count(*) FROM proj_fornecedores WHERE tenant_id = tid);
  RAISE NOTICE 'Vendas:       %', (SELECT count(*) FROM proj_vendas       WHERE tenant_id = tid);
  RAISE NOTICE 'Orçamentos:   %', (SELECT count(*) FROM proj_orcamentos   WHERE tenant_id = tid);
  RAISE NOTICE 'Pedidos:      %', (SELECT count(*) FROM proj_pedidos_compra WHERE tenant_id = tid);
  RAISE NOTICE 'CR:           %', (SELECT count(*) FROM proj_contas_receber WHERE tenant_id = tid);
  RAISE NOTICE 'CP:           %', (SELECT count(*) FROM proj_contas_pagar   WHERE tenant_id = tid);
  RAISE NOTICE 'NF-e/NFC-e:   %', (SELECT count(*) FROM proj_notas_fiscais  WHERE tenant_id = tid);
  RAISE NOTICE '';
  RAISE NOTICE 'Logins: admin/admin  •  carlos.vendedor/demo  •  lucia.financeiro/demo';
  RAISE NOTICE '        rafael.estoque/demo  •  patricia.compradora/demo  •  marcos.fiscal/demo';
END
$$;
