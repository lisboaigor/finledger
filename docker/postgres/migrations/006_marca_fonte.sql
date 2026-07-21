-- 006 — Fonte do wordmark whitelabel: o admin escolhe, entre um conjunto
-- curado, a tipografia do nome da marca (topbar, sidebar, login). Guarda a
-- CHAVE da fonte (ex.: 'pacifico'); o frontend mapeia chave → font-family e
-- carrega a fonte correspondente. Nula → fonte padrão (Grand Hotel).
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS marca_fonte TEXT
        CHECK (marca_fonte IS NULL OR marca_fonte ~ '^[a-z0-9-]{1,32}$');
