use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

// ── Financeiro / Caixa ────────────────────────────────────────────────────────

/// Ciclo de conversão de caixa (dias): CCC = DSO + DIO − DPO.
#[derive(Serialize, sqlx::FromRow)]
pub struct CicloFinanceiroResult {
    pub dso: f64,
    pub dio: f64,
    pub dpo: f64,
    pub ccc: f64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AgingResult {
    pub faixa: String,
    pub quantidade: i64,
    pub total_centavos: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct SemanaFluxoResult {
    pub semana: String,
    pub receber_centavos: i64,
    pub pagar_centavos: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DevedorResult {
    pub cliente_id: Option<Uuid>,
    pub nome: String,
    pub saldo_centavos: i64,
    pub dias_atraso: i32,
}

#[derive(Serialize)]
pub struct FinanceiroBi {
    pub ciclo: CicloFinanceiroResult,
    pub aging: Vec<AgingResult>,
    pub projecao: Vec<SemanaFluxoResult>,
    pub devedores: Vec<DevedorResult>,
}

#[derive(Query)]
#[query(result = FinanceiroBi)]
pub struct ObterFinanceiroBi;

impl QueryHandler<ObterFinanceiroBi> for BiHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ObterFinanceiroBi) -> Result<FinanceiroBi, AppError> {
        Ok(FinanceiroBi {
            ciclo: self.repo.ciclo_financeiro().await?,
            aging: self.repo.aging_recebiveis().await?,
            projecao: self.repo.projecao_semanal().await?,
            devedores: self.repo.top_devedores().await?,
        })
    }
}
