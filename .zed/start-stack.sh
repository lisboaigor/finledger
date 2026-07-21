#!/usr/bin/env sh
# Prepara a stack de apoio para a sessão de debug do backend:
#   1. Postgres (docker compose) se ainda não estiver no ar
#   2. Compila o backend (o CodeLLDB anexa ao binário gerado)
#
# O frontend é iniciado à parte pela task "⚙️ Frontend (Nuxt)" para que a
# saída do Nuxt apareça ao vivo no terminal do Zed.
#
# Idempotente: rodar o debug várias vezes não duplica o Postgres.
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. Postgres
if ! docker compose ps --status running 2>/dev/null | grep -q .; then
  echo "[stack] subindo Postgres..."
  docker compose up -d
fi

# 2. Backend (build síncrono; o launch anexa ao binário)
echo "[stack] compilando backend..."
cargo build --bin finledger
