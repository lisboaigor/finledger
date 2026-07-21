#![allow(clippy::unwrap_used, clippy::expect_used)]

mod helpers;
use helpers::{TestResult, in_tenant, new_tenant_id, setup_db, start_postgres};

use std::sync::Arc;

use pharos_app::CommandHandler;
use pharos_app::EventBus;
use pharos_core::Entity;
use finledger::fiscal::{
    application::{commands::CancelarNotaFiscal, handler::FiscalHandlers},
    domain::value_objects::StatusNFe,
    infrastructure::{repository::PostgresNotaFiscalRepository, sefaz::StubSefazClient},
};
use uuid::Uuid;

#[tokio::test]
async fn fiscal_gerar_e_transmitir_autoriza_nf() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(repo, sefaz, bus);

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-001".into(),
        descricao: "Filtro".into(),
        quantidade: 1,
        preco_unitario_centavos: 5000,
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item])
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    Ok(())
}

#[tokio::test]
async fn fiscal_cancelar_nf_autorizada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(Arc::clone(&repo), sefaz, bus);

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-002".into(),
        descricao: "Vela".into(),
        quantidade: 2,
        preco_unitario_centavos: 3000,
    };

    use pharos_core::Repository;
    use finledger::fiscal::domain::nota_fiscal::NotaFiscalId;

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item])
            .await
            .expect("gerar e transmitir falhou");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let nf_id: Option<Uuid> =
            sqlx::query_scalar("SELECT nf_id FROM proj_notas_fiscais WHERE venda_id = $1")
                .bind(venda_id)
                .fetch_optional(&pool)
                .await
                .expect("query nf_id falhou");

        let id = nf_id.expect("deve existir exatamente uma NF para esta venda");
        let nf = repo
            .find_by_id(&NotaFiscalId::from(id))
            .await
            .expect("buscar NF falhou")
            .expect("NF não encontrada");

        assert_eq!(nf.status, StatusNFe::Autorizada);

        fiscal
            .handle(CancelarNotaFiscal {
                nf_id: nf.id().as_uuid(),
                motivo: "teste de cancelamento".into(),
            })
            .await
            .expect("cancelar NF falhou");
    })
    .await;

    Ok(())
}

#[tokio::test]
async fn fiscal_projecao_registra_nf_autorizada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(repo, sefaz, bus);

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-003".into(),
        descricao: "Amortecedor".into(),
        quantidade: 1,
        preco_unitario_centavos: 12000,
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item])
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM proj_notas_fiscais WHERE venda_id = $1")
            .bind(venda_id)
            .fetch_optional(&pool)
            .await?;

    assert_eq!(
        status.as_deref(),
        Some("autorizada"),
        "projeção deve refletir status autorizada"
    );
    Ok(())
}