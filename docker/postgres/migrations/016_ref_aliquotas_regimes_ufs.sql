-- 016 — Referência de alíquotas: PIS/COFINS por regime e ICMS interno das 27 UFs.
--
-- PIS/COFINS: as linhas genéricas (regime NULL, 65/300 bps — regime cumulativo)
-- são MANTIDAS intactas: elas são o caminho do fallback legado (tenant sem
-- perfil fiscal → PerfilFiscal::padrao_legado), que precisa continuar emitindo
-- exatamente os valores de hoje. Adicionamos linhas ESPECÍFICAS por regime —
-- a resolução por especificidade do provider (mais chaves não-NULL casando
-- vence) faz:
--   lucro_real       → 165/760 bps (regime não-cumulativo, Leis 10.637/10.833);
--   lucro_presumido  → 65/300 bps (linha específica, mesmo valor da genérica);
--   simples_nacional → sem linha específica de PIS/COFINS/ICMS de propósito:
--                      o Simples configurado não destaca legados (motor zera,
--                      recolhimento por dentro do DAS — ver issue #4);
--   sem perfil       → linhas genéricas (comportamento atual preservado).
--
-- ICMS: alíquota MODAL interna de referência por UF (vigentes 2025/2026), em
-- bps, vigência aberta. ATENÇÃO: valores modais de referência — conferir na
-- ativação de cada UF (reduções/adicionais de fundo de pobreza variam por
-- mercadoria). Não afetam ninguém até um tenant configurar UF ≠ SP; a linha de
-- SP (1800) já existe na migração 009 e não é recriada aqui.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

INSERT INTO ref_aliquotas
    (tributo, uf, codigo_municipio, regime, c_class_trib, ncm_prefixo,
     aliquota_bps, vigencia_inicio, vigencia_fim) VALUES
    -- PIS/COFINS específicos por regime (mesma vigência das genéricas:
    -- extintos em 2027, vigência encerra 2026-12-31).
    ('pis',    NULL, NULL, 'lucro_real',      NULL, NULL, 165, '2000-01-01', '2026-12-31'),
    ('cofins', NULL, NULL, 'lucro_real',      NULL, NULL, 760, '2000-01-01', '2026-12-31'),
    ('pis',    NULL, NULL, 'lucro_presumido', NULL, NULL,  65, '2000-01-01', '2026-12-31'),
    ('cofins', NULL, NULL, 'lucro_presumido', NULL, NULL, 300, '2000-01-01', '2026-12-31'),
    -- ICMS interno modal por UF (referência 2025/2026; SP já existe na 009).
    ('icms', 'AC', NULL, NULL, NULL, NULL, 1900, '2000-01-01', NULL),
    ('icms', 'AL', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'AM', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'AP', NULL, NULL, NULL, NULL, 1800, '2000-01-01', NULL),
    ('icms', 'BA', NULL, NULL, NULL, NULL, 2050, '2000-01-01', NULL),
    ('icms', 'CE', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'DF', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'ES', NULL, NULL, NULL, NULL, 1700, '2000-01-01', NULL),
    ('icms', 'GO', NULL, NULL, NULL, NULL, 1900, '2000-01-01', NULL),
    ('icms', 'MA', NULL, NULL, NULL, NULL, 2300, '2000-01-01', NULL),
    ('icms', 'MG', NULL, NULL, NULL, NULL, 1800, '2000-01-01', NULL),
    ('icms', 'MS', NULL, NULL, NULL, NULL, 1700, '2000-01-01', NULL),
    ('icms', 'MT', NULL, NULL, NULL, NULL, 1700, '2000-01-01', NULL),
    ('icms', 'PA', NULL, NULL, NULL, NULL, 1900, '2000-01-01', NULL),
    ('icms', 'PB', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'PE', NULL, NULL, NULL, NULL, 2050, '2000-01-01', NULL),
    ('icms', 'PI', NULL, NULL, NULL, NULL, 2250, '2000-01-01', NULL),
    ('icms', 'PR', NULL, NULL, NULL, NULL, 1950, '2000-01-01', NULL),
    ('icms', 'RJ', NULL, NULL, NULL, NULL, 2200, '2000-01-01', NULL), -- 20% + FECP 2%
    ('icms', 'RN', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'RO', NULL, NULL, NULL, NULL, 1950, '2000-01-01', NULL),
    ('icms', 'RR', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'RS', NULL, NULL, NULL, NULL, 1700, '2000-01-01', NULL),
    ('icms', 'SC', NULL, NULL, NULL, NULL, 1700, '2000-01-01', NULL),
    ('icms', 'SE', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL),
    ('icms', 'TO', NULL, NULL, NULL, NULL, 2000, '2000-01-01', NULL)
ON CONFLICT (tributo, uf, codigo_municipio, regime, c_class_trib, ncm_prefixo, vigencia_inicio)
    DO NOTHING;
