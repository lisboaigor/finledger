-- 015 — Alíquota efetiva do DAS (Simples Nacional) no perfil fiscal do tenant.
-- Em bps (700 = 7%): é a alíquota efetiva do anexo/faixa do Simples, usada como
-- CUSTO tributário do vendedor quando o regime é Simples Nacional configurado —
-- a NF não destaca ICMS/PIS/COFINS/ISS (recolhimento por dentro do DAS).
-- Nullable: sem valor, o custo do DAS fica 0 (com aviso em log) até configurar.
--
-- Idempotente: pode ser reaplicado com segurança (psql -U postgres -d finledger -f ...).

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS aliquota_das_bps INTEGER
        CHECK (aliquota_das_bps IS NULL OR aliquota_das_bps BETWEEN 0 AND 20000);
