use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum FornecedorEvent {
    FornecedorCadastrado {
        #[aggregate_id]
        fornecedor_id: String,
        razao_social: String,
        cnpj: String,
        telefone: Option<String>,
        email: Option<String>,
        prazo_pagamento_dias: u16,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    FornecedorAtualizado {
        #[aggregate_id]
        fornecedor_id: String,
        razao_social: String,
        telefone: Option<String>,
        email: Option<String>,
        prazo_pagamento_dias: u16,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    FornecedorDesativado {
        #[aggregate_id]
        fornecedor_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    FornecedorReativado {
        #[aggregate_id]
        fornecedor_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
