use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPedidoSnapshot {
    pub produto_id: String,
    pub quantidade: u32,
    pub custo_unitario_centavos: i64,
}

#[derive(Debug, Clone, DomainEvent)]
pub enum ComprasEvent {
    PedidoCompraGerado {
        #[aggregate_id]
        pedido_id: String,
        comprador_id: String,
        fornecedor_id: String,
        itens: Vec<ItemPedidoSnapshot>,
        prazo_pagamento_dias: u16,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PedidoCompraAprovado {
        #[aggregate_id]
        pedido_id: String,
        aprovador_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PedidoCompraEnviado {
        #[aggregate_id]
        pedido_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    MercadoriaRecebida {
        #[aggregate_id]
        pedido_id: String,
        recebimento_id: String,
        fornecedor_id: String,
        itens: Vec<ItemPedidoSnapshot>,
        total_centavos: i64,
        prazo_pagamento_dias: u16,
        tudo_recebido: bool,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PedidoCancelado {
        #[aggregate_id]
        pedido_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
