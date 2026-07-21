use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{Router, routing::get, serve};
use axum_prometheus::PrometheusMetricLayer;
use dotenvy::dotenv;
use finledger::{bootstrap::Bootstrap, web::router};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("finledger=info,tower_http=info")),
        )
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let bootstrap = Bootstrap::initialize().await?;

    // Ciclo de BI (ETL do warehouse + motor de alertas) em background.
    finledger::bi::job::spawn(bootstrap.state.pool.clone());

    // Lixeira: arquiva vendas/orçamentos não concretizados após o prazo do tenant.
    finledger::arquivamento::spawn(bootstrap.state.pool.clone());

    // HTTP metrics (request count/duration/in-flight per route) exported in
    // Prometheus format. The exporter listens on its own port so /metrics is
    // never reachable through the public API surface. Defaults to loopback —
    // set METRICS_ADDR (e.g. 0.0.0.0:9464) to expose it on other interfaces,
    // and METRICS_TOKEN to require `Authorization: Bearer <token>` when doing so.
    let (metrics_layer, metrics_handle) = PrometheusMetricLayer::pair();

    let app = router(bootstrap.state).layer(metrics_layer);

    let metrics_addr: SocketAddr = std::env::var("METRICS_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 9464)));
    let metrics_token = std::env::var("METRICS_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    let metrics_app = Router::new().route(
        "/metrics",
        get(move |headers: axum::http::HeaderMap| {
            let handle = metrics_handle.clone();
            let token = metrics_token.clone();
            async move {
                if let Some(expected) = token {
                    let authorized = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.strip_prefix("Bearer "))
                        .is_some_and(|t| t == expected);
                    if !authorized {
                        return Err(axum::http::StatusCode::UNAUTHORIZED);
                    }
                }
                Ok(handle.render())
            }
        }),
    );
    tokio::spawn(async move {
        match TcpListener::bind(metrics_addr).await {
            Ok(listener) => {
                info!("métricas Prometheus em http://{metrics_addr}/metrics");
                if let Err(e) = serve(listener, metrics_app).await {
                    tracing::error!(error = %e, "servidor de métricas encerrou com erro");
                }
            }
            Err(e) => tracing::error!(error = %e, "não foi possível abrir a porta de métricas"),
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    // O bind vem ANTES do log de "ouvindo": com a porta ocupada, o erro real
    // aparece por último e sem log enganoso de servidor no ar.
    let listener = TcpListener::bind(addr).await.with_context(|| {
        format!(
            "porta {addr} já está em uso — outro finledger rodando? \
             (cargo run em background, sessão de debug antiga ou `just back`); \
             veja quem é com: lsof -nP -iTCP:3000 -sTCP:LISTEN"
        )
    })?;

    info!("Finledger ouvindo em http://{addr}");

    // `into_make_service_with_connect_info` expõe o IP do peer via `ConnectInfo`,
    // usado pelo rate limiter (fallback quando não há header X-Forwarded-For).
    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
