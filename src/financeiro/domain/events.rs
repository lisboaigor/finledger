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
        /// Rótulo humano opcional (ex.: "Parcela 2/3 — venda X").
        descricao: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PagamentoRecebido {
        #[aggregate_id]
        conta_id: String,
        valor_centavos: i64,
        /// Total recebido acumulado APÓS este pagamento — permite à projeção
        /// gravar o valor absoluto (idempotente sob entrega at-least-once).
        valor_recebido_total_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    /// Redução do valor em aberto da conta (devolução parcial, desconto
    /// negociado). Mantém `valor_original` intacto para histórico.
    AbatimentoContaReceberRegistrado {
        #[aggregate_id]
        conta_id: String,
        valor_centavos: i64,
        /// Total abatido acumulado APÓS este abatimento (projeção idempotente).
        valor_abatido_total_centavos: i64,
        motivo: String,
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
        /// Rótulo humano opcional. Em CP de reembolso deixa explícito que o
        /// credor é o CLIENTE da venda devolvida (não um fornecedor).
        descricao: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PagamentoEfetuado {
        #[aggregate_id]
        conta_id: String,
        valor_centavos: i64,
        /// Total pago acumulado APÓS este pagamento (projeção idempotente).
        valor_pago_total_centavos: i64,
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
