-- 004 — Meta de faturamento mensal: alvo de crescimento definido pelo gestor.
-- Alimenta o card de progresso no dashboard e o componente "Rumo à meta" do
-- score de saúde. Distinta do faturamento_mensal_estimado (que é o retrato
-- atual usado no rateio dos custos fixos): a meta é onde se quer chegar.
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS meta_faturamento_mensal_centavos BIGINT
        CHECK (meta_faturamento_mensal_centavos IS NULL
               OR meta_faturamento_mensal_centavos > 0);
