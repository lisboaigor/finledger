use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

use super::value_objects::{ItemNF, ModeloNF, TotaisNF};

#[derive(Debug, Clone, DomainEvent)]
pub enum NotaFiscalEvent {
    NotaFiscalGerada {
        #[aggregate_id]
        nf_id: String,
        venda_id: String,
        cliente_id: Option<String>,
        modelo: ModeloNF,
        serie: String,
        numero: u32,
        itens: Vec<ItemNF>,
        totais: TotaisNF,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalTransmitida {
        #[aggregate_id]
        nf_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalAutorizada {
        #[aggregate_id]
        nf_id: String,
        chave: String,
        protocolo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalRejeitada {
        #[aggregate_id]
        nf_id: String,
        codigo: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalCancelada {
        #[aggregate_id]
        nf_id: String,
        protocolo_cancelamento: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    /// Cancelamento registrado como PENDENTE: a integração com a SEFAZ ainda
    /// não está ativa (trâmites burocráticos), então a nota fica marcada para
    /// cancelamento assim que a integração entrar em operação.
    CancelamentoNfSolicitado {
        #[aggregate_id]
        nf_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
