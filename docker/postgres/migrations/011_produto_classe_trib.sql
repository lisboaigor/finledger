-- 011 — Classe tributária (cClassTrib, NT 2025.002) no produto.
-- NULL = classe padrão '000001' (tributação integral): o catálogo existente
-- continua funcionando sem reclassificação em massa.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

ALTER TABLE proj_produtos
    ADD COLUMN IF NOT EXISTS c_class_trib VARCHAR(6);
