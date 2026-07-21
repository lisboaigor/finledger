use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum IdentityEvent {
    UsuarioCriado {
        #[aggregate_id]
        usuario_id: String,
        username: String,
        password_hash: String,
        roles: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    SenhaAlterada {
        #[aggregate_id]
        usuario_id: String,
        password_hash: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    UsuarioDesativado {
        #[aggregate_id]
        usuario_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    UsuarioReativado {
        #[aggregate_id]
        usuario_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    RolesAlteradas {
        #[aggregate_id]
        usuario_id: String,
        roles: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
