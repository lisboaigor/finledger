-- 005 — Marca whitelabel: o admin do tenant personaliza a identidade visual da
-- aplicação (nome exibido no wordmark, logo, cor de destaque e cor de fundo).
-- Tudo self-service e opcional — colunas nulas caem no tema/marca padrão
-- (Finledger). O logo é guardado como data URI (base64) na própria linha do
-- tenant: sem infra de object storage, e o payload cabe no limite de 1 MB do
-- corpo das requisições (o upload é redimensionado no cliente).
--
-- Idempotente: pode ser reaplicado com segurança.

ALTER TABLE tenants
    -- Nome exibido no lugar de "Finledger" (topbar, sidebar, login). Vazio → padrão.
    ADD COLUMN IF NOT EXISTS marca_nome TEXT
        CHECK (marca_nome IS NULL OR char_length(marca_nome) <= 40),
    -- Logo como data URI (data:image/...;base64,...). Limite defensivo de ~700 KB
    -- de texto (≈512 KB de imagem) para caber no corpo com folga.
    ADD COLUMN IF NOT EXISTS marca_logo_data_uri TEXT
        CHECK (marca_logo_data_uri IS NULL
               OR (marca_logo_data_uri LIKE 'data:image/%'
                   AND char_length(marca_logo_data_uri) <= 700000)),
    -- Cor de destaque (accent/primária), hex #RRGGBB.
    ADD COLUMN IF NOT EXISTS marca_cor_primaria TEXT
        CHECK (marca_cor_primaria IS NULL OR marca_cor_primaria ~ '^#[0-9a-fA-F]{6}$'),
    -- Cor de fundo geral da aplicação, hex #RRGGBB.
    ADD COLUMN IF NOT EXISTS marca_cor_fundo TEXT
        CHECK (marca_cor_fundo IS NULL OR marca_cor_fundo ~ '^#[0-9a-fA-F]{6}$');
