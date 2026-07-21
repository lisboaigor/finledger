use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

#[derive(Debug, Clone, DomainEvent)]
pub enum CatalogoEvent {
    ProdutoCadastrado {
        #[aggregate_id]
        produto_id: String,
        sku: String,
        descricao: String,
        ncm: String,
        unidade: String,
        preco_custo_centavos: i64,
        preco_venda_centavos: i64,
        categoria: String,
        marca: Option<String>,
        controla_estoque: bool,
        /// Classe tributária (cClassTrib) da reforma tributária.
        c_class_trib: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    PrecosAtualizados {
        #[aggregate_id]
        produto_id: String,
        preco_custo_centavos: i64,
        preco_venda_centavos: i64,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ProdutoAtualizado {
        #[aggregate_id]
        produto_id: String,
        sku: String,
        descricao: String,
        ncm: String,
        unidade: String,
        categoria: String,
        marca: Option<String>,
        controla_estoque: bool,
        c_class_trib: Option<String>,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ProdutoDesativado {
        #[aggregate_id]
        produto_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    ProdutoReativado {
        #[aggregate_id]
        produto_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}
