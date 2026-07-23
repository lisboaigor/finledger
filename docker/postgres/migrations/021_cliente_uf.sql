-- 021 — UF do destinatário no cliente.
-- Necessária para o CFOP da NF (operação intra vs. interestadual: 5102/6102 na
-- venda, 1202/2202 na devolução). Aditiva e nullable: clientes existentes ficam
-- sem UF e são tratados como operação interna (fallback seguro).
--
-- Idempotente: reaplicável (psql -U postgres -d finledger -f ...).

ALTER TABLE proj_clientes ADD COLUMN IF NOT EXISTS uf CHAR(2);
