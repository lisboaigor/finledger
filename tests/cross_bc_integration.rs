#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Testa os fluxos cross-BC:
/// VendaConfirmada → ContaReceber criada (FinanceiroVendaEventHandler)
/// VendaConfirmada → NF emitida (FiscalVendaEventHandler)
/// MercadoriaRecebida → EstoqueEntrada (EstoqueComprasEventHandler)
mod helpers;
use helpers::{TestResult, aguardar_projecoes, in_tenant, new_tenant_id, setup_db, start_postgres};

use std::sync::Arc;

use pharos_app::{CommandHandler, EventBus};
use finledger::compras::application::commands::ItemRecebidoInput;
use finledger::compras::application::commands::{
    AprovarPedidoCompra, EnviarPedidoCompra, GerarPedidoCompra, ItemPedidoInput, ReceberMercadoria,
};
use finledger::compras::infrastructure::repository::PostgresPedidoCompraRepository;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::estoque::infrastructure::repository::PostgresEstoqueRepository;
use finledger::financeiro::infrastructure::repository::{
    PostgresContaPagarRepository, PostgresContaReceberRepository,
};
use finledger::fiscal::infrastructure::aliquotas::PostgresAliquotaProvider;
use finledger::fiscal::infrastructure::repository::PostgresNotaFiscalRepository;
use finledger::tenants::repository::TenantRepository;
use finledger::vendas::application::commands::{
    AdicionarItemVenda, ConfirmarVenda, DefinirFormaPagamento, IniciarVenda,
};
use finledger::vendas::domain::value_objects::FormaPagamento;
use finledger::vendas::infrastructure::repository::PostgresVendaRepository;
use finledger::{
    compras::application::handler::ComprasHandlers,
    estoque::application::{
        event_handlers::{EstoqueComprasEventHandler, EstoqueVendaEventHandler},
        handler::EstoqueHandlers,
    },
    financeiro::application::{
        event_handlers::FinanceiroVendaEventHandler, handler::FinanceiroHandlers,
    },
    fiscal::{
        application::{event_handlers::FiscalVendaEventHandler, handler::FiscalHandlers},
        infrastructure::sefaz::StubSefazClient,
    },
    projections::{
        estoque::EstoqueProjection, financeiro::FinanceiroProjection, fiscal::FiscalProjection,
    },
    vendas::application::handler::VendasHandlers,
};
use uuid::Uuid;

async fn montar_handlers(
    pool: &pharos_postgres::Pool,
) -> (
    Arc<VendasHandlers>,
    Arc<EstoqueHandlers>,
    Arc<ComprasHandlers>,
) {
    let bus = EventBus::new();

    let estoque = Arc::new(EstoqueHandlers::new(
        Arc::new(PostgresEstoqueRepository::new(pool.clone())),
        bus.clone(),
    ));

    let vendas = Arc::new(VendasHandlers::new(
        Arc::new(PostgresVendaRepository::new(pool.clone())),
        bus.clone(),
        pool.clone(),
    ));

    let financeiro = Arc::new(FinanceiroHandlers::new(
        Arc::new(PostgresContaReceberRepository::new(pool.clone())),
        Arc::new(PostgresContaPagarRepository::new(pool.clone())),
        bus.clone(),
    ));

    let fiscal = Arc::new(FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(StubSefazClient),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus.clone(),
    ));

    let compras = Arc::new(ComprasHandlers::new(
        Arc::new(PostgresPedidoCompraRepository::new(pool.clone())),
        bus.clone(),
    ));

    bus.register(FinanceiroVendaEventHandler {
        financeiro: Arc::clone(&financeiro),
    });
    bus.register(FiscalVendaEventHandler {
        fiscal: Arc::clone(&fiscal),
    });
    bus.register(EstoqueComprasEventHandler {
        estoque: Arc::clone(&estoque),
    });
    bus.register(EstoqueVendaEventHandler {
        estoque: Arc::clone(&estoque),
    });
    bus.register(FinanceiroProjection::new(pool.clone()));
    bus.register(FiscalProjection::new(pool.clone()));
    bus.register(EstoqueProjection::new(pool.clone()));

    (vendas, estoque, compras)
}

#[tokio::test]
async fn venda_confirmada_gera_conta_receber_e_nf() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let (vendas, estoque, _) = montar_handlers(&pool).await;
    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    let vendedor_id = Uuid::new_v4();

    let venda_id = in_tenant(tenant_id, async move {
        estoque
            .handle(RegistrarEntradaEstoque {
                produto_id,
                quantidade: 10,
                custo_unitario_centavos: 1000,
                motivo: "entrada inicial".into(),
                nota_fiscal: None,
            })
            .await
            .expect("entrada estoque falhou");
        aguardar_projecoes().await;

        let venda_id = vendas
            .handle(IniciarVenda {
                vendedor_id,
                cliente_id: None,
            })
            .await
            .expect("iniciar venda falhou");

        vendas
            .handle(AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-X".into(),
                descricao: "Correia dentada".into(),
                quantidade: 2,
                preco_unitario_centavos: 8000,
                vender_sem_estoque: false,
            })
            .await
            .expect("adicionar item falhou");

        vendas
            .handle(DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
            })
            .await
            .expect("definir pagamento falhou");

        vendas
            .handle(ConfirmarVenda {
                venda_id: venda_id.as_uuid(),
            })
            .await
            .expect("confirmar venda falhou");

        venda_id
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let conta_receber_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM proj_contas_receber WHERE venda_id = $1")
            .bind(venda_id.as_uuid())
            .fetch_one(&pool)
            .await?;

    let nf_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM proj_notas_fiscais WHERE venda_id = $1")
            .bind(venda_id.as_uuid())
            .fetch_one(&pool)
            .await?;

    let saldo_estoque: i32 =
        sqlx::query_scalar("SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1")
            .bind(produto_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(conta_receber_count, 1, "deve existir uma conta a receber");
    assert_eq!(nf_count, 1, "deve existir uma NF para a venda");
    assert_eq!(
        saldo_estoque, 8,
        "estoque deve ser baixado (10 - 2) via EstoqueVendaEventHandler"
    );
    Ok(())
}

#[tokio::test]
async fn mercadoria_recebida_incrementa_estoque() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let (_, _, compras) = montar_handlers(&pool).await;
    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    let fornecedor_id = Uuid::new_v4();
    let comprador_id = Uuid::new_v4();

    in_tenant(tenant_id, async move {
        let pedido_id = compras
            .handle(GerarPedidoCompra {
                comprador_id,
                fornecedor_id,
                itens: vec![ItemPedidoInput {
                    produto_id,
                    quantidade: 20,
                    custo_unitario_centavos: 500,
                }],
                prazo_pagamento_dias: 30,
            })
            .await
            .expect("gerar pedido falhou");

        compras
            .handle(AprovarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
                aprovador_id: Uuid::new_v4(),
            })
            .await
            .expect("aprovar pedido falhou");
        compras
            .handle(EnviarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            })
            .await
            .expect("enviar pedido falhou");
        compras
            .handle(ReceberMercadoria {
                pedido_id: pedido_id.as_uuid(),
                itens_recebidos: vec![ItemRecebidoInput {
                    produto_id,
                    quantidade: 20,
                }],
            })
            .await
            .expect("receber mercadoria falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let saldo: Option<i32> =
        sqlx::query_scalar("SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1")
            .bind(produto_id)
            .fetch_optional(&pool)
            .await?;

    assert_eq!(saldo, Some(20), "saldo deve ser 20 após recebimento");
    Ok(())
}