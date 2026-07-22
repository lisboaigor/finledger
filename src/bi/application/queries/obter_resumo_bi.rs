use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;

use crate::bi::application::handler::BiHandlers;
use crate::error::AppError;

/// KPIs do dashboard "Hoje". Percentuais podem ser nulos quando ainda não há
/// dados no período (ex.: margem exige o ETL do BI já ter carregado o mês).
#[derive(Serialize, sqlx::FromRow)]
pub struct BiResumoResult {
    pub receita_mes_centavos: i64,
    pub receita_mes_anterior_centavos: i64,
    pub vencidas_centavos: i64,
    pub caixa_30d_centavos: i64,
    pub margem_percent: Option<f64>,
    /// Margem líquida de impostos da NF (reforma LC 214/2025) — sobra após o
    /// imposto que é custo do vendedor; `None` sem vendas/NF no mês.
    pub margem_liquida_percent: Option<f64>,
    pub conversao_percent: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ReceitaDiaResult {
    pub dia: String,
    pub total_centavos: i64,
}

#[derive(Serialize)]
pub struct BiResumoCompleto {
    pub resumo: BiResumoResult,
    pub receita_diaria: Vec<ReceitaDiaResult>,
    /// Score de saúde 0–100 + detalhamento por componente (bi.score_saude);
    /// `{"score": null}` enquanto não há dado suficiente.
    pub saude: serde_json::Value,
    /// Meta de faturamento do mês (Configurações); None = sem meta definida.
    pub meta_faturamento_mensal_centavos: Option<i64>,
}

#[derive(Query)]
#[query(result = BiResumoCompleto)]
pub struct ObterResumoBi;

impl QueryHandler<ObterResumoBi> for BiHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ObterResumoBi) -> Result<BiResumoCompleto, AppError> {
        Ok(BiResumoCompleto {
            resumo: self.repo.resumo().await?,
            receita_diaria: self.repo.receita_diaria().await?,
            saude: self.repo.score_saude().await?,
            meta_faturamento_mensal_centavos: self.repo.meta_faturamento().await?,
        })
    }
}
