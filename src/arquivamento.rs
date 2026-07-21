use std::time::Duration;

use pharos_postgres::Pool;
use tokio::time::MissedTickBehavior;

/// Intervalo padrão da varredura de arquivamento (sobreponível via
/// `ARQUIVAMENTO_INTERVAL_SECS`). Arquivar é barato e sem urgência — 1h basta.
const INTERVALO_PADRAO_SECS: u64 = 3600;

/// Agenda a "lixeira": `SELECT executar_arquivamento()` carimba vendas
/// abandonadas/canceladas e orçamentos não convertidos mais antigos que
/// `tenants.arquivamento_dias` (função SECURITY DEFINER da migração 002).
/// Nada é excluído — as listagens padrão apenas escondem o que tem carimbo,
/// e o gestor restaura pela lixeira.
pub fn spawn(pool: Pool) {
    let secs = std::env::var("ARQUIVAMENTO_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(INTERVALO_PADRAO_SECS);

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(secs));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match sqlx::query_scalar::<_, serde_json::Value>("SELECT executar_arquivamento()")
                .fetch_one(&pool)
                .await
            {
                Ok(resultado) => tracing::info!(%resultado, "varredura de arquivamento executada"),
                Err(e) => {
                    tracing::warn!(error = %e, "arquivamento falhou — migração 002 aplicada?");
                }
            }
        }
    });
}
