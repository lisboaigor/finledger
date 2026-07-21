-- 012 — Detalhamento de impostos na projeção de notas fiscais.
-- Os montantes vivem congelados nos eventos (NotaFiscalGerada); estas colunas
-- materializam o breakdown para listagem/BI. Notas anteriores ao motor ficam
-- com 0 (os valores não foram calculados à época — 0 é fiel, não aproximação).
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

ALTER TABLE proj_notas_fiscais
    ADD COLUMN IF NOT EXISTS icms_centavos    BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pis_centavos     BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS cofins_centavos  BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS iss_centavos     BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS cbs_centavos     BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS ibs_uf_centavos  BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS ibs_mun_centavos BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS is_centavos      BIGINT NOT NULL DEFAULT 0;
