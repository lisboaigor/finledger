use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;
use serde::{Deserialize, Serialize};

use super::value_objects::FormaPagamento;

/// Snapshot de um item no momento da confirmação da venda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemVendaSnapshot {
    pub item_id: String,
    pub produto_id: String,
    pub sku: String,
    pub descricao: String,
    pub quantidade: u32,
    pub preco_unitario_centavos: i64,
}

#[derive(Debug, Clone, DomainEvent)]
pub enum VendaEvent {
    VendaIniciada {
        #[aggregate_id]
        venda_id: String,
        vendedor_id: String,
        cliente_id: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ItemAdicionado {
        #[aggregate_id]
        venda_id: String,
        item_id: String,
        produto_id: String,
        sku: String,
        descricao: String,
        quantidade: u32,
        preco_unitario_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ItemRemovido {
        #[aggregate_id]
        venda_id: String,
        item_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    FormaPagamentoDefinida {
        #[aggregate_id]
        venda_id: String,
        forma: FormaPagamento,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    VendaConfirmada {
        #[aggregate_id]
        venda_id: String,
        vendedor_id: String,
        cliente_id: Option<String>,
        itens: Vec<ItemVendaSnapshot>,
        total_centavos: i64,
        forma_pagamento: FormaPagamento,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    VendaCancelada {
        #[aggregate_id]
        venda_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    /// Devolução de itens de uma venda confirmada. Carrega tanto os itens
    /// devolvidos (para o estoque reentrar) quanto os restantes (para o fiscal
    /// reemitir a NF quando a integração SEFAZ estiver ativa). Em devolução
    /// total, `VendaCancelada` é emitido em seguida (venda desfeita).
    ItensDevolvidos {
        #[aggregate_id]
        venda_id: String,
        cliente_id: Option<String>,
        /// Itens com a quantidade DEVOLVIDA (não a vendida).
        itens_devolvidos: Vec<ItemVendaSnapshot>,
        /// Itens que permanecem na venda após a devolução (vazio = total).
        itens_restantes: Vec<ItemVendaSnapshot>,
        total_devolvido_centavos: i64,
        devolucao_total: bool,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    VendaAtualizada {
        #[aggregate_id]
        venda_id: String,
        cliente_id: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
