-- 007 — Tamanho e cor do wordmark whitelabel: além da fonte (006), o admin
-- ajusta o TAMANHO (percentual sobre o tamanho base de cada local: topbar,
-- sidebar, login, landing) e a COR do nome da marca. Ambos opcionais — nulos
-- caem no padrão (100% e cor de texto herdada).
--
--   marca_fonte_tamanho — inteiro percentual, 50..200 (100 = padrão).
--   marca_fonte_cor     — hex #RRGGBB.
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS marca_fonte_tamanho SMALLINT
        CHECK (marca_fonte_tamanho IS NULL OR marca_fonte_tamanho BETWEEN 50 AND 200);

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS marca_fonte_cor TEXT
        CHECK (marca_fonte_cor IS NULL OR marca_fonte_cor ~ '^#[0-9a-fA-F]{6}$');
