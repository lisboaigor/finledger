#!/bin/sh
# Roda no PRIMEIRO initdb do Postgres (docker-entrypoint-initdb.d), depois do
# init.sql (montado como zz-prod-roles.sh — ordem alfabética). Num volume já
# inicializado o entrypoint pula esta pasta: para reaplicar, use psql manualmente.
#
# As senhas não podem conter aspas simples (') — ver .env.prod.example.
set -eu

# Substitui a senha placeholder do role da aplicação criada pelo init.sql.
psql -v ON_ERROR_STOP=1 --username postgres --dbname finledger <<-EOSQL
	ALTER ROLE finledger PASSWORD '${APP_DB_PASSWORD}';
EOSQL

# Role pessoal para acesso humano via túnel SSH (opcional). BYPASSRLS porque as
# tabelas têm RLS por tenant — sem isso uma sessão sem tenant não vê linha alguma.
if [ -n "${ADMIN_DB_USERNAME:-}" ] && [ -n "${ADMIN_DB_PASSWORD:-}" ]; then
	psql -v ON_ERROR_STOP=1 --username postgres --dbname finledger <<-EOSQL
		CREATE ROLE "${ADMIN_DB_USERNAME}" LOGIN BYPASSRLS PASSWORD '${ADMIN_DB_PASSWORD}';
		GRANT CONNECT ON DATABASE finledger TO "${ADMIN_DB_USERNAME}";
		GRANT USAGE ON SCHEMA public TO "${ADMIN_DB_USERNAME}";
		GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO "${ADMIN_DB_USERNAME}";
		GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO "${ADMIN_DB_USERNAME}";
		ALTER DEFAULT PRIVILEGES IN SCHEMA public
		    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO "${ADMIN_DB_USERNAME}";
		ALTER DEFAULT PRIVILEGES IN SCHEMA public
		    GRANT USAGE, SELECT ON SEQUENCES TO "${ADMIN_DB_USERNAME}";
	EOSQL
	echo "prod-roles.sh: role de acesso remoto '${ADMIN_DB_USERNAME}' criado."
else
	echo "prod-roles.sh: ADMIN_DB_USERNAME/ADMIN_DB_PASSWORD ausentes — role remoto não criado."
fi
