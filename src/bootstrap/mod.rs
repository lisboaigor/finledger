pub mod database;
pub mod events;
pub mod handlers;
pub mod outbox_relay;
pub mod projections;
pub mod repositories;
pub mod seed;

use std::sync::Arc;

use anyhow::{Context, Result};
use pharos_app::EventBus;
use pharos_postgres::migrate_postgres_eventing_schema;
use tokio::sync::Notify;

use handlers::Handlers;
use repositories::Repositories;

use crate::auth::AuthConfig;
use crate::web::state::AppState;

pub struct Bootstrap {
    pub state: AppState,
}

impl Bootstrap {
    pub async fn initialize() -> Result<Self> {
        let pool = database::create_pool()
            .await
            .context("falha ao inicializar banco de dados")?;

        let jwt_secret =
            std::env::var("JWT_SECRET").context("variável de ambiente JWT_SECRET não definida")?;

        // Tabelas de infra do outbox/inbox (issue #3). As migrações já as
        // materializam; reaplicar no boot é idempotente e cobre bases sem a
        // migração ainda aplicada.
        if let Err(e) = migrate_postgres_eventing_schema(&pool).await {
            tracing::warn!("migração do schema de outbox/inbox falhou (ignorando): {e:#}");
        }

        let auth = Arc::new(AuthConfig::new(jwt_secret));
        let bus = EventBus::new();
        let repositories = Repositories::new(&pool);
        let handlers = Handlers::new(repositories, pool.clone(), bus.clone(), auth);

        events::register(&bus, &handlers, pool.clone());

        projections::register(&bus, pool.clone());

        // Relay do outbox: decoders dos eventos produtores + task de fundo que
        // despacha os efeitos cross-context/projeções, cutucada a cada commit
        // durável para leitura pós-escrita sub-ms.
        outbox_relay::registrar_decoders(&bus);
        let kick = Arc::new(Notify::new());
        crate::shared::registrar_relay_kick(kick.clone());
        outbox_relay::spawn(pool.clone(), bus.clone(), kick);

        if let Err(e) = seed::seed_demo_tenant(&handlers.tenants, &handlers.identity.clone()).await
        {
            tracing::warn!("seed do tenant demo falhou (ignorando): {e:#}");
        }

        if let Err(e) = seed::seed_superadmin(&handlers.backoffice).await {
            tracing::warn!("seed do superadmin falhou (ignorando): {e:#}");
        }

        Ok(Self {
            state: handlers.into_state(pool),
        })
    }
}
