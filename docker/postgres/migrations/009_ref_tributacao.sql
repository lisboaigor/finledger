-- 009 — Tabelas de referência tributária (reforma EC 132/2023, LC 214/2025).
-- Dados GLOBAIS (sem tenant_id, sem RLS): classes tributárias (cClassTrib da
-- NT 2025.002) e alíquotas por tributo/vigência/UF/regime/classe/NCM.
-- Alíquota é DADO, não código: atualizar valores (ex.: quando a alíquota de
-- referência de CBS/IBS for fixada por resolução do Senado) é uma migração
-- nova, não um release do backend.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

-- ── Classes tributárias (cClassTrib) ─────────────────────────────────────────
-- Enquadramento do item nos grupos de IBS/CBS: CST correspondente e a redução
-- de alíquota que a classe carrega (6000 bps = redução de 60% da LC 214;
-- 10000 = alíquota zero, ex.: cesta básica nacional).

CREATE TABLE IF NOT EXISTS ref_classes_tributarias (
    c_class_trib VARCHAR(6) PRIMARY KEY,
    descricao    TEXT       NOT NULL,
    cst_ibs_cbs  VARCHAR(3) NOT NULL,
    reducao_bps  INTEGER    NOT NULL DEFAULT 0 CHECK (reducao_bps BETWEEN 0 AND 10000)
);

INSERT INTO ref_classes_tributarias (c_class_trib, descricao, cst_ibs_cbs, reducao_bps) VALUES
    ('000001', 'Tributação integral',                                   '000', 0),
    ('200003', 'Redução de 60% — bens e serviços listados (LC 214, anexo VII)', '200', 6000),
    ('410001', 'Alíquota zero — cesta básica nacional (LC 214, anexo I)',       '410', 10000)
ON CONFLICT (c_class_trib) DO NOTHING;

-- ── Alíquotas ────────────────────────────────────────────────────────────────
-- Chaves NULL = "qualquer" (curinga). A resolução escolhe a linha vigente na
-- data com MAIOR especificidade (mais chaves não-NULL casando com o contexto).
-- ncm_prefixo casa por prefixo (ex.: '2203' pega toda cerveja) — usado pelo
-- Imposto Seletivo e por exceções por NCM.

CREATE TABLE IF NOT EXISTS ref_aliquotas (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
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
    UNIQUE NULLS NOT DISTINCT (tributo, uf, codigo_municipio, regime, c_class_trib,
                               ncm_prefixo, vigencia_inicio)
);

CREATE INDEX IF NOT EXISTS idx_ref_aliquotas_lookup
    ON ref_aliquotas (tributo, vigencia_inicio, vigencia_fim);

-- Seed da transição. ATENÇÃO: as alíquotas plenas de CBS (2027+) e IBS (2033)
-- são ALÍQUOTAS DE REFERÊNCIA ESTIMADAS — ajustar por migração quando fixadas
-- por resolução do Senado Federal.
INSERT INTO ref_aliquotas
    (tributo, uf, codigo_municipio, regime, c_class_trib, ncm_prefixo,
     aliquota_bps, vigencia_inicio, vigencia_fim) VALUES
    -- Legados: ICMS 18% (padrão SP, vigência aberta — o phase-down 2029–2032 é
    -- aplicado pelo motor via FaseTransicao, não pela tabela); PIS/COFINS
    -- extintos em 2027 (vigência encerra 2026-12-31).
    ('icms',    'SP', NULL, NULL, NULL, NULL, 1800, '2000-01-01', NULL),
    ('pis',     NULL, NULL, NULL, NULL, NULL,   65, '2000-01-01', '2026-12-31'),
    ('cofins',  NULL, NULL, NULL, NULL, NULL,  300, '2000-01-01', '2026-12-31'),
    -- 2026 (ano-teste): CBS 0,9% + IBS 0,1% (0,05 UF + 0,05 município),
    -- destacados de forma informativa.
    ('cbs',     NULL, NULL, NULL, NULL, NULL,   90, '2026-01-01', '2026-12-31'),
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL,    5, '2026-01-01', '2028-12-31'),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,    5, '2026-01-01', '2028-12-31'),
    -- 2027+: CBS plena (referência estimada 8,8%); IBS segue simbólico até 2028.
    ('cbs',     NULL, NULL, NULL, NULL, NULL,  880, '2027-01-01', NULL),
    -- 2029–2032: IBS sobe proporcionalmente à redução do ICMS/ISS
    -- (frações anuais da alíquota de referência estimada 17,7% ≈ 14,16% UF + 3,54% mun).
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL,  142, '2029-01-01', '2029-12-31'),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,   35, '2029-01-01', '2029-12-31'),
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL,  283, '2030-01-01', '2030-12-31'),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,   71, '2030-01-01', '2030-12-31'),
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL,  425, '2031-01-01', '2031-12-31'),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,  106, '2031-01-01', '2031-12-31'),
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL,  566, '2032-01-01', '2032-12-31'),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,  142, '2032-01-01', '2032-12-31'),
    -- 2033: extinção do ICMS/ISS — IBS pleno (referência estimada).
    ('ibs_uf',  NULL, NULL, NULL, NULL, NULL, 1416, '2033-01-01', NULL),
    ('ibs_mun', NULL, NULL, NULL, NULL, NULL,  354, '2033-01-01', NULL)
ON CONFLICT (tributo, uf, codigo_municipio, regime, c_class_trib, ncm_prefixo, vigencia_inicio)
    DO NOTHING;
