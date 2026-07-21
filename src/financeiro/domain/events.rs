use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum FinanceiroEvent {
    ContaReceberRegistrada {
        #[aggregate_id]
        conta_id: String,
        venda_id: String,
        cliente_id: Option<String>,
        valor_centavos: i64,
        vencimento: DateTime<Utc>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PagamentoRecebido {
        #[aggregate_id]
        conta_id: String,
        valor_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ContaReceberLiquidada {
        #[aggregate_id]
        conta_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ContaReceberEstornada {
        #[aggregate_id]
        conta_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ContaPagarRegistrada {
        #[aggregate_id]
        conta_id: String,
        pedido_id: String,
        fornecedor_id: String,
        valor_centavos: i64,
        vencimento: DateTime<Utc>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PagamentoEfetuado {
        #[aggregate_id]
        conta_id: String,
        valor_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ContaPagarLiquidada {
        #[aggregate_id]
        conta_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
