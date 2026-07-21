use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

// ── Estoque & Compras ─────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct MatrizAbcXyzResult {
    pub abc: String,
    pub xyz: String,
    pub produtos: i64,
    pub valor_centavos: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RupturaResult {
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub classe_abc: String,
    pub quantidade: i32,
    pub cobertura_dias: Option<i32>,
    pub sugestao_compra: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EstoqueMortoResult {
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub quantidade: i32,
    pub valor_centavos: i64,
    pub dias_sem_venda: Option<i32>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CategoriaGiroResult {
    pub categoria: String,
    pub receita_centavos: i64,
    pub margem_percent: Option<f64>,
    pub valor_estoque_centavos: i64,
    pub giro: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PedidoParadoResult {
    pub pedido_id: Uuid,
    pub fornecedor: String,
    pub total_centavos: i64,
    pub status: String,
    pub dias_parado: i32,
}

#[derive(Serialize)]
pub struct EstoqueBi {
    pub matriz: Vec<MatrizAbcXyzResult>,
    pub rupturas: Vec<RupturaResult>,
    pub mortos: Vec<EstoqueMortoResult>,
    pub categorias: Vec<CategoriaGiroResult>,
    pub pedidos_parados: Vec<PedidoParadoResult>,
}

#[derive(Query)]
#[query(result = EstoqueBi)]
pub struct ObterEstoqueBi;

impl QueryHandler<ObterEstoqueBi> for BiHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ObterEstoqueBi) -> Result<EstoqueBi, AppError> {
        Ok(EstoqueBi {
            matriz: self.repo.matriz_abc_xyz().await?,
            rupturas: self.repo.rupturas().await?,
            mortos: self.repo.estoque_morto().await?,
            categorias: self.repo.giro_categorias().await?,
            pedidos_parados: self.repo.pedidos_parados().await?,
        })
    }
}
