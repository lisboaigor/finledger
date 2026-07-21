-- 003 — Rateio proporcional dos custos fixos na precificação assistida.
-- Em vez de dividir os custos fixos igualmente por venda (R$ fixos por
-- unidade, que explodia a conta de itens baratos), eles viram um percentual
-- do preço: custos_fixos ÷ faturamento mensal estimado. O denominador é este
-- novo campo; vendas_mensais_estimadas permanece para o ponto de equilíbrio.
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS faturamento_mensal_estimado_centavos BIGINT
        CHECK (faturamento_mensal_estimado_centavos IS NULL
               OR faturamento_mensal_estimado_centavos > 0);
