use std::time::Duration;

use pharos_postgres::Pool;
use tokio::time::MissedTickBehavior;

/// Intervalo padrão do ciclo de ETL + avaliação de alertas (sobreponível via
/// `BI_ETL_INTERVAL_SECS`). O primeiro ciclo roda imediatamente no boot.
const INTERVALO_PADRAO_SECS: u64 = 300;

/// Agenda o ciclo de BI: `SELECT bi.executar_etl()` (dimensões SCD2, fatos
/// incrementais por watermark, snapshot diário de estoque e recálculo de
/// alertas — tudo SECURITY DEFINER no banco). Falhas viram warn e o ciclo
/// seguinte tenta de novo — o backend sobe mesmo sem o schema `bi` aplicado.
pub fn spawn(pool: Pool) {
    let secs = std::env::var("BI_ETL_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(INTERVALO_PADRAO_SECS);

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(secs));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match sqlx::query_scalar::<_, serde_json::Value>("SELECT bi.executar_etl()")
                .fetch_one(&pool)
                .await
            {
                Ok(resultado) => tracing::info!(%resultado, "ciclo de BI executado"),
                Err(e) => {
                    // Erro (não warn): ETL parado congela dashboards e alertas em
                    // silêncio; o resumo do BI expõe `etl_atualizado_em` para o
                    // dashboard denunciar dados velhos.
                    tracing::error!(error = %e, "ciclo de BI falhou — schema `bi` aplicado? (docker/postgres/bi.sql)");
                }
            }
        }
    });
}
