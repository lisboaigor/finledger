default:
    just --list

# Sobe banco + backend + frontend em paralelo
dev:
    just db &
    sleep 2
    just back &
    just front

# Apenas o banco (Docker)
db:
    docker compose up -d

# Apenas o backend Rust
back:
    cargo run

# Apenas o frontend Nuxt
front:
    cd frontend && pnpm dev

# Build de produção do frontend
build-front:
    cd frontend && pnpm build

# Instala dependências do frontend
install:
    cd frontend && pnpm install

# Aplica as migrações SQL no banco local já existente (idempotentes)
migrate:
    for f in docker/postgres/migrations/*.sql; do docker exec -i finledger-postgres-1 psql -v ON_ERROR_STOP=1 -U postgres -d finledger < "$f"; done

# Roda os testes Rust
test:
    cargo test

# Para tudo
stop:
    docker compose down

# Deploy em produção (envia o commit HEAD, migra e reconstrói na VPS).
# Ver scripts/deploy.py para variáveis e opções (FORCE=1, ALLOW_DIRTY=1).
deploy:
    python3 scripts/deploy.py
