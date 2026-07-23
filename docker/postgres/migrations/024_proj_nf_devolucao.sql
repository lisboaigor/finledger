-- 024 — Finalidade/sentido e referência da NF de devolução.
-- A devolução de mercadoria já circulada não se resolve com cancelamento, e sim
-- com uma NF-e de devolução (entrada, finNFe=4) que referencia a chave da nota
-- original. Materializamos esses campos na projeção para conferência.
--
-- `finalidade` = finNFe ('1' normal, '4' devolução); `tipo_operacao` = tpNF
-- ('1' saída, '0' entrada). Padrões refletem as NFs de venda existentes.
--
-- Idempotente: reaplicável (psql -U postgres -d finledger -f ...).

ALTER TABLE proj_notas_fiscais
    ADD COLUMN IF NOT EXISTS finalidade            CHAR(1)     NOT NULL DEFAULT '1',
    ADD COLUMN IF NOT EXISTS tipo_operacao         CHAR(1)     NOT NULL DEFAULT '1',
    ADD COLUMN IF NOT EXISTS nf_chave_referenciada VARCHAR(44);
