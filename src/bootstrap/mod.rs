pub mod database;
pub mod events;
pub mod handlers;
pub mod projections;
pub mod repositories;
pub mod seed;

use std::sync::Arc;

use anyhow::{Context, Result};
use pharos_app::EventBus;

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

        let auth = Arc::new(AuthConfig::new(jwt_secret));
        let bus = EventBus::new();
        let repositories = Repositories::new(&pool);
        let handlers = Handlers::new(repositories, pool.clone(), bus.clone(), auth);

        events::register(&bus, &handlers, pool.clone());

        projections::register(&bus, pool.clone());

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
