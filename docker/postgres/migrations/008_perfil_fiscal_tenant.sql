-- 008 — Perfil fiscal do tenant (reforma tributária EC 132/2023, LC 214/2025).
-- Regime tributário, UF/município (IBGE) e CRT determinam como o motor
-- tributário calcula os impostos das NFs. Tudo nullable: tenant sem perfil
-- configurado cai no fallback legado (Simples Nacional/SP) e nada muda para
-- quem já está em produção.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS regime_tributario VARCHAR(20)
        CHECK (regime_tributario IS NULL OR regime_tributario IN
               ('simples_nacional', 'lucro_presumido', 'lucro_real')),
    ADD COLUMN IF NOT EXISTS uf CHAR(2),
    ADD COLUMN IF NOT EXISTS codigo_municipio VARCHAR(7),
    ADD COLUMN IF NOT EXISTS crt SMALLINT
        CHECK (crt IS NULL OR crt BETWEEN 1 AND 4),
    -- Simples Nacional pode optar pelo regime regular de IBS/CBS (LC 214,
    -- art. 41) para gerar crédito aos clientes; FALSE = recolhe via DAS e os
    -- valores de IBS/CBS na NF são informativos.
    ADD COLUMN IF NOT EXISTS ibs_cbs_regime_regular BOOLEAN NOT NULL DEFAULT FALSE;
