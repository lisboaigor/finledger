use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum EstoqueEvent {
    EstoqueEntrada {
        #[aggregate_id]
        item_id: String,
        produto_id: String,
        quantidade: u32,
        custo_unitario_centavos: i64,
        motivo: String,
        nota_fiscal: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    EstoqueSaida {
        #[aggregate_id]
        item_id: String,
        produto_id: String,
        quantidade: u32,
        motivo: String,
        referencia_id: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    AjusteEstoque {
        #[aggregate_id]
        item_id: String,
        quantidade_anterior: u32,
        quantidade_nova: u32,
        // Custo unitário das unidades acrescentadas num ajuste para cima —
        // obrigatório quando o saldo aumenta, para não diluir o custo médio
        // com unidades "de graça". `None` em ajuste para baixo ou igual.
        custo_unitario_centavos: Option<i64>,
        justificativa: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    EstoqueMinimoDefinido {
        #[aggregate_id]
        item_id: String,
        produto_id: String,
        estoque_minimo: u32,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    EstoqueMinimoPadraoAtingido {
        #[aggregate_id]
        item_id: String,
        produto_id: String,
        saldo_atual: u32,
        estoque_minimo: u32,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
