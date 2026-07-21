#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::future::Future;

use pharos_app::{CURRENT_TENANT, TenantContext};
use pharos_postgres::{Pool, connect_pool};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};
use uuid::Uuid;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn start_postgres_with_url() -> TestResult<(ContainerAsync<GenericImage>, Pool, String)> {
    let container = GenericImage::new("postgres", "16-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .start()
        .await?;

    let host = container.get_host().await?.to_string();
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = connect_pool(&url, 4)?;

    Ok((container, pool, url))
}

pub async fn start_postgres() -> TestResult<(ContainerAsync<GenericImage>, Pool)> {
    let (c, pool, _) = start_postgres_with_url().await?;
    Ok((c, pool))
}

/// Lê `docker/postgres/init.sql` e executa apenas o DDL puro (tabelas, índices, RLS).
/// Descarta as linhas de setup do Docker (CREATE USER, GRANT, \c) que dependem
/// do banco `finledger` e do usuário `admin` criados pelo container.
pub async fn setup_db(pool: &Pool) -> TestResult {
    let sql = std::fs::read_to_string("docker/postgres/init.sql")?;

    // Tudo a partir do primeiro separador de seção é DDL puro.
    let start = sql
        .find("-- ── Control plane")
        .expect("marcador '-- ── Control plane' não encontrado em init.sql");
    // Meta-comandos do psql (linhas começando com '\', ex.: \i seed) não são SQL.
    let ddl_filtrado: String = sql[start..]
        .lines()
        .filter(|l| !l.trim_start().starts_with('\\'))
        .collect::<Vec<_>>()
        .join("\n");
    // Box::leak é aceitável em testes — o conteúdo é pequeno e o processo termina logo.
    let ddl: &'static str = Box::leak(ddl_filtrado.into_boxed_str());

    sqlx::raw_sql(ddl).execute(pool).await?;

    // Migrações incrementais (docker/postgres/migrations/*.sql, idempotentes)
    // — mesmas aplicadas pelo initdb/`just migrate` fora dos testes.
    let mut migracoes: Vec<_> = std::fs::read_dir("docker/postgres/migrations")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    migracoes.sort();
    for caminho in migracoes {
        let sql: &'static str = Box::leak(std::fs::read_to_string(caminho)?.into_boxed_str());
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}

/// Aplica o schema analítico `bi` (docker/postgres/bi.sql) — necessário para
/// testes de score de saúde/ETL. Idempotente, como no initdb.
#[allow(dead_code)]
pub async fn setup_bi(pool: &Pool) -> TestResult {
    let sql = std::fs::read_to_string("docker/postgres/bi.sql")?;
    let filtrado: String = sql
        .lines()
        .filter(|l| !l.trim_start().starts_with('\\'))
        .collect::<Vec<_>>()
        .join("\n");
    let sql: &'static str = Box::leak(filtrado.into_boxed_str());
    sqlx::raw_sql(sql).execute(pool).await?;
    Ok(())
}

/// Retorna um UUID novo para usar como tenant_id em testes.
#[allow(dead_code)]
pub fn new_tenant_id() -> Uuid {
    Uuid::new_v4()
}

/// Executa `f` dentro de um escopo de tenant — equivalente ao que `require_auth` faz em produção.
#[allow(dead_code)]
pub async fn in_tenant<F: Future>(tenant_id: Uuid, f: F) -> F::Output {
    CURRENT_TENANT
        .scope(Some(TenantContext::new(tenant_id)), f)
        .await
}

/// Insere um tenant na tabela `tenants` e retorna seu UUID.
#[allow(dead_code)]
pub async fn create_tenant(pool: &Pool, slug: &str) -> TestResult<Uuid> {
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO tenants (slug, nome) VALUES ($1, $1) RETURNING tenant_id")
            .bind(slug)
            .fetch_one(pool)
            .await?;
    Ok(id)
}

/// Suspende um tenant diretamente no banco.
#[allow(dead_code)]
pub async fn suspend_tenant(pool: &Pool, tenant_id: Uuid) -> TestResult {
    sqlx::query("UPDATE tenants SET status = 'suspenso' WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Monta todos os handlers com a mesma fiação de produção
/// (event handlers cross-BC + projeções registrados no bus).
#[allow(dead_code)]
pub fn montar_app(pool: &Pool) -> finledger::bootstrap::handlers::Handlers {
    use std::sync::Arc;

    use pharos_app::EventBus;
    use finledger::auth::AuthConfig;
    use finledger::bootstrap::{handlers::Handlers, repositories::Repositories};

    let bus = EventBus::new();
    let auth = Arc::new(AuthConfig::new("segredo-de-teste".into()));
    let handlers = Handlers::new(Repositories::new(pool), pool.clone(), bus.clone(), auth);
    finledger::bootstrap::events::register(&bus, &handlers, pool.clone());
    finledger::bootstrap::projections::register(&bus, pool.clone());
    handlers
}

/// Aguarda as projeções assíncronas processarem os eventos publicados.
#[allow(dead_code)]
pub async fn aguardar_projecoes() {
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
}