use std::env::var;
use std::time::Duration;

use anyhow::{Context, Result};
use pharos_postgres::Pool;
use sqlx::ConnectOptions;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::{info, warn};

use crate::shared::tenant::current_tenant;

pub async fn create_pool() -> Result<Pool> {
    let database_url =
        var("DATABASE_URL").context("variável de ambiente DATABASE_URL não definida")?;

    let options = database_url
        .parse::<PgConnectOptions>()
        .context("DATABASE_URL inválida")?
        .log_statements(tracing::log::LevelFilter::Debug);

    let pool = PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(30))
        // Enforcement da RLS: antes de entregar a conexão ao request, grava o tenant
        // da requisição atual (task-local `CURRENT_TENANT`) na GUC `app.tenant_id`, que
        // as policies de `pharos_tenant_aggregates` e `proj_*` usam para filtrar linhas.
        // Sem tenant em escopo (login pré-tenant, backoffice, health) grava vazio → as
        // policies não casam nenhuma linha (deny-by-default) via `NULLIF(...,'')::uuid`.
        .before_acquire(|conn, _meta| {
            Box::pin(async move {
                let value = current_tenant()
                    .map(|t| t.tenant_id().as_uuid().to_string())
                    .unwrap_or_default();
                sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
                    .bind(value)
                    .execute(&mut *conn)
                    .await?;
                Ok(true)
            })
        })
        // Ao devolver a conexão ao pool, limpa a GUC para não vazar o tenant de um
        // request para o próximo que reutilize a mesma conexão física.
        .after_release(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SELECT set_config('app.tenant_id', '', false)")
                    .execute(&mut *conn)
                    .await?;
                Ok(true)
            })
        })
        .connect_lazy_with(options);

    wait_for_db(&pool).await?;

    Ok(pool)
}

async fn wait_for_db(pool: &Pool) -> Result<()> {
    let mut delay = Duration::from_millis(500);
    for attempt in 1..=20 {
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => {
                info!("banco de dados pronto");
                return Ok(());
            }
            Err(e) => {
                warn!(attempt, ?e, "banco não disponível, aguardando...");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(5));
            }
        }
    }
    anyhow::bail!("banco de dados não ficou disponível após 20 tentativas")
}
