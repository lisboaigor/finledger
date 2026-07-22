-- 014 — Desconto global da venda (issue #1: desconto do orçamento se perdia na
-- conversão para venda; agora a venda carrega `desconto_centavos` e o
-- `total_centavos` projetado é o LÍQUIDO (bruto dos itens − desconto).
-- A NF destaca o mesmo desconto e o total da nota também sai líquido.
--
-- Aditiva e idempotente: pode ser reaplicada com segurança
-- (psql -U postgres -d finledger -f ...). Linhas existentes ficam com 0 —
-- comportamento idêntico ao anterior.

ALTER TABLE proj_vendas
    ADD COLUMN IF NOT EXISTS desconto_centavos BIGINT NOT NULL DEFAULT 0;

ALTER TABLE proj_notas_fiscais
    ADD COLUMN IF NOT EXISTS desconto_centavos BIGINT NOT NULL DEFAULT 0;
