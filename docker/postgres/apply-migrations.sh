#!/bin/bash
# Aplica todas as migrações de docker/postgres/migrations/ em ordem.
# No initdb (primeiro boot do volume) roda automaticamente após o init.sql
# (montado como y-migrations.sh — ordem alfabética do entrypoint). Em bancos
# já existentes o entrypoint não roda de novo: aplicar manualmente com
#   docker exec -i <postgres> psql -U postgres -d finledger -f /migrations/00X_*.sql
# (as migrações são idempotentes).
set -euo pipefail

for f in /migrations/*.sql; do
    echo "aplicando migração $f"
    psql -v ON_ERROR_STOP=1 --username postgres --dbname finledger -f "$f"
done
