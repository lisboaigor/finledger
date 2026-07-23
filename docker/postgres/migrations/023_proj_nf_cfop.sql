-- 023 — CFOP materializado por item da NF.
-- O CFOP agora é dinâmico (operação intra/interestadual, venda/devolução) e não
-- mais fixo em 5102. Materializá-lo na projeção dá observabilidade fiscal
-- (relatórios/conferência) além do valor já congelado no evento/agregado.
--
-- Idempotente: reaplicável (psql -U postgres -d finledger -f ...).

ALTER TABLE proj_nf_itens ADD COLUMN IF NOT EXISTS cfop VARCHAR(4);
