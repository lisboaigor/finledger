use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOrcamentoSnapshot {
    pub item_id: String,
    pub produto_id: String,
    pub sku: String,
    pub descricao: String,
    pub quantidade: u32,
    pub preco_unitario_centavos: i64,
}

#[derive(Debug, Clone, DomainEvent)]
pub enum OrcamentoEvent {
    OrcamentoCriado {
        #[aggregate_id]
        orcamento_id: String,
        vendedor_id: String,
        cliente_id: Option<String>,
        /// Nome informal do cliente quando não há cadastro completo no CRM
        /// (ex.: "João, cliente de balcão"). Só tem sentido quando
        /// `cliente_id` é `None`.
        cliente_avulso: Option<String>,
        validade_dias: u16,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ItemAdicionadoOrcamento {
        #[aggregate_id]
        orcamento_id: String,
        item_id: String,
        produto_id: String,
        sku: String,
        descricao: String,
        quantidade: u32,
        preco_unitario_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ItemRemovidoOrcamento {
        #[aggregate_id]
        orcamento_id: String,
        item_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    DescontoOrcamentoAplicado {
        #[aggregate_id]
        orcamento_id: String,
        desconto_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoEmitido {
        #[aggregate_id]
        orcamento_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoAceito {
        #[aggregate_id]
        orcamento_id: String,
        itens: Vec<ItemOrcamentoSnapshot>,
        total_centavos: i64,
        // Vendedor/cliente do orçamento, para o assinante (VendaAPartirDe-
        // OrcamentoHandler) criar a venda EmAndamento já com essas partes.
        vendedor_id: String,
        cliente_id: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoRecusado {
        #[aggregate_id]
        orcamento_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoExpirado {
        #[aggregate_id]
        orcamento_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoConvertidoEmVenda {
        #[aggregate_id]
        orcamento_id: String,
        venda_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoAtualizado {
        #[aggregate_id]
        orcamento_id: String,
        cliente_id: Option<String>,
        cliente_avulso: Option<String>,
        validade_dias: u16,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    OrcamentoCancelado {
        #[aggregate_id]
        orcamento_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
