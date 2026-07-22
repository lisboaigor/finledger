#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Financeiro: contas a receber/pagar criadas via eventos
/// cross-BC, pagamentos, estorno e query handlers.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, seed_produto, setup_db,
    start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::compras::application::commands::{
    AprovarPedidoCompra, EnviarPedidoCompra, GerarPedidoCompra, ItemPedidoInput, ItemRecebidoInput,
    ReceberMercadoria,
};
use finledger::error::AppError;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::financeiro::application::commands::{
    EfetuarPagamento, EstornarContaReceber, RegistrarPagamentoRecebido,
};
use finledger::financeiro::application::queries::{
    BuscarContaPagar, BuscarContaReceber, ListarContasPagar, ListarContasReceber,
};
use finledger::vendas::application::commands::{
    AdicionarItemVenda, ConfirmarVenda, DefinirFormaPagamento, IniciarVenda,
};
use finledger::vendas::domain::value_objects::FormaPagamento;
use uuid::Uuid;

async fn confirmar_venda_a_prazo(
    app: &finledger::bootstrap::handlers::Handlers,
    pool: &pharos_postgres::Pool,
    tenant_id: Uuid,
) -> Uuid {
    let produto_id = Uuid::new_v4();
    // AdicionarItemVenda usa o preço de tabela do catálogo (15000).
    seed_produto(pool, tenant_id, produto_id, "SKU-1", 15000)
        .await
        .expect("seed produto");
    dispatch(
        &*app.estoque,
        RegistrarEntradaEstoque {
            produto_id,
            quantidade: 10,
            custo_unitario_centavos: 1000,
            motivo: "estoque".into(),
            nota_fiscal: None,
        },
    )
    .await
    .expect("entrada");

    let venda_id = dispatch(
        &*app.vendas,
        IniciarVenda {
            vendedor_id: Uuid::new_v4(),
            cliente_id: Some(Uuid::new_v4()),
        },
    )
    .await
    .expect("iniciar");
    dispatch(
        &*app.vendas,
        AdicionarItemVenda {
            venda_id: venda_id.as_uuid(),
            produto_id,
            sku: "SKU-1".into(),
            descricao: "Amortecedor".into(),
            quantidade: 2,
            preco_unitario_centavos: 15000,
            vender_sem_estoque: false,
                preservar_preco_informado: false,
        },
    )
    .await
    .expect("item");
    dispatch(
        &*app.vendas,
        DefinirFormaPagamento {
            venda_id: venda_id.as_uuid(),
            forma: FormaPagamento::Prazo { dias: 30 },
        },
    )
    .await
    .expect("forma");
    dispatch(
        &*app.vendas,
        ConfirmarVenda {
            venda_id: venda_id.as_uuid(),
        },
    )
    .await
    .expect("confirmar");
    venda_id.as_uuid()
}

#[tokio::test]
async fn conta_receber_pagamento_parcial_e_quitacao() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        let venda_id = confirmar_venda_a_prazo(&app, &pool_seed, tenant_id).await;
        aguardar_projecoes().await;

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber)
            .await
            .expect("listar");
        assert_eq!(contas.len(), 1);
        let conta = &contas[0];
        assert_eq!(conta.venda_id, venda_id);
        assert_eq!(conta.valor_original, 30000);
        assert_eq!(conta.status, "Pendente");

        // Pagamento parcial
        dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id: conta.conta_id,
                valor_centavos: 10000,
            },
        )
        .await
        .expect("pagamento parcial");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.financeiro,
            BuscarContaReceber {
                conta_id: conta.conta_id,
            },
        )
        .await
        .expect("buscar")
        .expect("conta deve existir");
        assert_eq!(row.valor_recebido, 10000);

        // Quitação
        dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id: conta.conta_id,
                valor_centavos: 20000,
            },
        )
        .await
        .expect("quitação");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.financeiro,
            BuscarContaReceber {
                conta_id: conta.conta_id,
            },
        )
        .await
        .expect("buscar")
        .expect("conta deve existir");
        assert_eq!(row.valor_recebido, 30000);
        assert_eq!(row.status, "Liquidada");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn estornar_conta_receber() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        confirmar_venda_a_prazo(&app, &pool_seed, tenant_id).await;
        aguardar_projecoes().await;

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber)
            .await
            .expect("listar");
        let conta_id = contas[0].conta_id;

        dispatch(
            &*app.financeiro,
            EstornarContaReceber {
                conta_id,
                motivo: "venda cancelada".into(),
            },
        )
        .await
        .expect("estornar");
        aguardar_projecoes().await;

        let row = query_dispatch(&*app.financeiro, BuscarContaReceber { conta_id })
            .await
            .expect("buscar")
            .expect("conta deve existir");
        assert_eq!(row.status, "Estornada");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn conta_pagar_criada_no_recebimento_e_quitada() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let produto_id = Uuid::new_v4();
        let pedido_id = dispatch(
            &*app.compras,
            GerarPedidoCompra {
                comprador_id: Uuid::new_v4(),
                fornecedor_id: Uuid::new_v4(),
                itens: vec![ItemPedidoInput {
                    produto_id,
                    quantidade: 10,
                    custo_unitario_centavos: 700,
                }],
                prazo_pagamento_dias: 15,
            },
        )
        .await
        .expect("gerar");
        dispatch(
            &*app.compras,
            AprovarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
                aprovador_id: Uuid::new_v4(),
            },
        )
        .await
        .expect("aprovar");
        dispatch(
            &*app.compras,
            EnviarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            },
        )
        .await
        .expect("enviar");
        dispatch(
            &*app.compras,
            ReceberMercadoria {
                pedido_id: pedido_id.as_uuid(),
                itens_recebidos: vec![ItemRecebidoInput {
                    produto_id,
                    quantidade: 10,
                }],
            },
        )
        .await
        .expect("receber");
        aguardar_projecoes().await;

        let contas = query_dispatch(&*app.financeiro, ListarContasPagar)
            .await
            .expect("listar");
        assert_eq!(contas.len(), 1);
        let conta = &contas[0];
        assert_eq!(conta.pedido_id, pedido_id.as_uuid());
        assert_eq!(conta.valor_original, 7000);

        dispatch(
            &*app.financeiro,
            EfetuarPagamento {
                conta_id: conta.conta_id,
                valor_centavos: 7000,
            },
        )
        .await
        .expect("pagar");
        aguardar_projecoes().await;

        let row = query_dispatch(
            &*app.financeiro,
            BuscarContaPagar {
                conta_id: conta.conta_id,
            },
        )
        .await
        .expect("buscar")
        .expect("conta deve existir");
        assert_eq!(row.valor_pago, 7000);
        assert_eq!(row.status, "Liquidada");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn pagamento_em_conta_inexistente_retorna_not_found() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let r = dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id: Uuid::new_v4(),
                valor_centavos: 100,
            },
        )
        .await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::NotFound))));
    })
    .await;
    Ok(())
}