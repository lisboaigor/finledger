use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

// ── Comercial / Funil / Clientes ──────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow)]
pub struct FunilResult {
    pub status: String,
    pub quantidade: i64,
    pub total_centavos: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct OrcamentoExpirandoResult {
    pub orcamento_id: Uuid,
    pub cliente: String,
    pub total_centavos: i64,
    pub vence_em_dias: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct VendedorResult {
    pub vendedor: String,
    pub receita_centavos: i64,
    pub vendas: i64,
    pub ticket_centavos: i64,
    pub conversao_percent: Option<f64>,
    pub desconto_percent: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct RfmSegmentoResult {
    pub segmento: String,
    pub clientes: i64,
    pub valor_centavos: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ClienteRiscoResult {
    pub cliente_id: Uuid,
    pub nome: String,
    pub valor_12m_centavos: i64,
    pub recencia_dias: i32,
    pub telefone: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub struct ComercialBi {
    pub funil: Vec<FunilResult>,
    pub expirando: Vec<OrcamentoExpirandoResult>,
    pub vendedores: Vec<VendedorResult>,
    pub rfm: Vec<RfmSegmentoResult>,
    pub em_risco: Vec<ClienteRiscoResult>,
}

#[derive(Query)]
#[query(result = ComercialBi)]
pub struct ObterComercialBi;

impl QueryHandler<ObterComercialBi> for BiHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ObterComercialBi) -> Result<ComercialBi, AppError> {
        Ok(ComercialBi {
            funil: self.repo.funil_orcamentos().await?,
            expirando: self.repo.orcamentos_expirando().await?,
            vendedores: self.repo.desempenho_vendedores().await?,
            rfm: self.repo.rfm_segmentos().await?,
            em_risco: self.repo.clientes_em_risco().await?,
        })
    }
}
