use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum CrmEvent {
    ClienteCadastrado {
        #[aggregate_id]
        cliente_id: String,
        nome: String,
        cpf_cnpj: String,
        uf: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ClienteAtualizado {
        #[aggregate_id]
        cliente_id: String,
        nome: String,
        telefone: Option<String>,
        email: Option<String>,
        uf: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ClienteBloqueado {
        #[aggregate_id]
        cliente_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ClienteDesbloqueado {
        #[aggregate_id]
        cliente_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ClienteDesativado {
        #[aggregate_id]
        cliente_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ClienteReativado {
        #[aggregate_id]
        cliente_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
