-- 018 — Financeiro: abatimento em conta a receber + descrição livre (issues #6/#14).
--   • proj_contas_receber.valor_abatido: total abatido (devolução parcial ou
--     abatimento manual) — o saldo em aberto passa a ser
--     valor_original − valor_abatido − valor_recebido.
--   • descricao em CR/CP: rótulo humano ("Parcela 2/3 — venda X",
--     "Reembolso ao cliente — devolução da venda X"). Em CP de reembolso o
--     credor é o CLIENTE da venda (pedido_id/fornecedor_id são reaproveitados
--     como venda_id/cliente_id, a descrição deixa isso explícito).
--
-- Aditiva e idempotente: pode ser reaplicada com segurança.

ALTER TABLE proj_contas_receber
    ADD COLUMN IF NOT EXISTS valor_abatido BIGINT NOT NULL DEFAULT 0;

ALTER TABLE proj_contas_receber
    ADD COLUMN IF NOT EXISTS descricao TEXT;

ALTER TABLE proj_contas_pagar
    ADD COLUMN IF NOT EXISTS descricao TEXT;
