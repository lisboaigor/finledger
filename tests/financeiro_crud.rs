#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Financeiro: contas a receber/pagar criadas via eventos
/// cross-BC, pagamentos, abatimento, estorno, devoluções e query handlers.
mod helpers;
use helpers::{
    TestResult, drenar_outbox, in_tenant, montar_app, new_tenant_id, seed_produto, setup_db,
    start_postgres,
};

use chrono::Utc;
use pharos_app::{DispatchError, EventHandler, dispatch, query_dispatch};
use finledger::compras::application::commands::{
    AprovarPedidoCompra, EnviarPedidoCompra, GerarPedidoCompra, ItemPedidoInput, ItemRecebidoInput,
    ReceberMercadoria,
};
use finledger::error::AppError;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::financeiro::application::commands::{
    EfetuarPagamento, EstornarContaReceber, RegistrarAbatimentoContaReceber,
    RegistrarPagamentoRecebido,
};
use finledger::financeiro::application::event_handlers::FinanceiroVendaEventHandler;
use finledger::financeiro::application::queries::{
    BuscarContaPagar, BuscarContaReceber, ListarContasPagar, ListarContasReceber,
};
use finledger::vendas::application::commands::{
    AdicionarItemVenda, ConfirmarVenda, DefinirFormaPagamento, DevolucaoItem, DevolverItensVenda,
    IniciarVenda,
};
use finledger::vendas::domain::events::VendaEvent;
use finledger::vendas::domain::value_objects::FormaPagamento;
use uuid::Uuid;

/// Confirma uma venda de 2 × R$150,00 (total 30000) com a forma de pagamento
/// dada. Retorna (venda_id, item_id) — o item serve para testes de devolução.
async fn confirmar_venda(
    app: &finledger::bootstrap::handlers::Handlers,
    pool: &pharos_postgres::Pool,
    tenant_id: Uuid,
    forma: FormaPagamento,
) -> (Uuid, Uuid) {
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
    let item_id = dispatch(
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
            forma,
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
    (venda_id.as_uuid(), item_id)
}

async fn confirmar_venda_a_prazo(
    app: &finledger::bootstrap::handlers::Handlers,
    pool: &pharos_postgres::Pool,
    tenant_id: Uuid,
) -> Uuid {
    confirmar_venda(app, pool, tenant_id, FormaPagamento::Prazo { dias: 30 })
        .await
        .0
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
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
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
        drenar_outbox(&pool).await.expect("drenar outbox");
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
        drenar_outbox(&pool).await.expect("drenar outbox");
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
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
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
        drenar_outbox(&pool).await.expect("drenar outbox");

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
async fn estorno_manual_de_conta_com_recebimento_e_rejeitado() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        confirmar_venda_a_prazo(&app, &pool_seed, tenant_id).await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        let conta_id = contas[0].conta_id;

        dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id,
                valor_centavos: 10000,
            },
        )
        .await
        .expect("pagamento");

        let r = dispatch(
            &*app.financeiro,
            EstornarContaReceber {
                conta_id,
                motivo: "engano".into(),
            },
        )
        .await;
        assert!(
            matches!(r, Err(DispatchError::Handler(AppError::Domain(_)))),
            "estorno de conta com recebimento deve ser bloqueado, veio {r:?}"
        );
        drenar_outbox(&pool).await.expect("drenar outbox");

        let row = query_dispatch(&*app.financeiro, BuscarContaReceber { conta_id })
            .await
            .expect("buscar")
            .expect("conta deve existir");
        assert_eq!(row.status, "Parcial", "conta deve permanecer intacta");
        assert_eq!(row.valor_recebido, 10000);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn venda_a_vista_cria_conta_ja_liquidada() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        let (venda_id, _) = confirmar_venda(&app, &pool_seed, tenant_id, FormaPagamento::Pix).await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        assert_eq!(contas.len(), 1);
        let conta = &contas[0];
        assert_eq!(conta.venda_id, venda_id);
        assert_eq!(conta.valor_original, 30000);
        assert_eq!(
            conta.valor_recebido, 30000,
            "à vista: o recebimento entra no ato"
        );
        assert_eq!(conta.status, "Liquidada");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn cartao_credito_3x_cria_tres_parcelas_com_sobra_na_ultima() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        // Total 30001 (indivisível por 3) para exercer a sobra de arredondamento.
        let produto_id = Uuid::new_v4();
        seed_produto(&pool_seed, tenant_id, produto_id, "SKU-3X", 30001)
            .await
            .expect("seed produto");
        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id,
                quantidade: 5,
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
                sku: "SKU-3X".into(),
                descricao: "Kit embreagem".into(),
                quantidade: 1,
                preco_unitario_centavos: 30001,
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
                forma: FormaPagamento::CartaoCredito { parcelas: 3 },
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
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Listagem ordenada por vencimento = ordem das parcelas.
        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        assert_eq!(contas.len(), 3, "cartão 3x deve gerar 3 contas");
        assert_eq!(contas[0].valor_original, 10000);
        assert_eq!(contas[1].valor_original, 10000);
        assert_eq!(
            contas[2].valor_original, 10001,
            "sobra de arredondamento na última parcela"
        );
        let soma: i64 = contas.iter().map(|c| c.valor_original).sum();
        assert_eq!(soma, 30001);
        for (i, conta) in contas.iter().enumerate() {
            assert_eq!(conta.status, "Pendente");
            let descricao = conta.descricao.as_deref().unwrap_or_default();
            assert!(
                descricao.starts_with(&format!("Parcela {}/3", i + 1)),
                "descrição inesperada: {descricao:?}"
            );
        }
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn devolucao_total_com_conta_paga_gera_reembolso() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        let (venda_id, item_id) = confirmar_venda(
            &app,
            &pool_seed,
            tenant_id,
            FormaPagamento::Prazo { dias: 30 },
        )
        .await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Cliente quita a conta antes de devolver.
        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        let conta_id = contas[0].conta_id;
        dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id,
                valor_centavos: 30000,
            },
        )
        .await
        .expect("quitar");
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Devolução TOTAL (as 2 unidades).
        dispatch(
            &*app.vendas,
            DevolverItensVenda {
                venda_id,
                itens: vec![DevolucaoItem {
                    item_id,
                    quantidade: 2,
                }],
                motivo: "defeito".into(),
            },
        )
        .await
        .expect("devolver");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let row = query_dispatch(&*app.financeiro, BuscarContaReceber { conta_id })
            .await
            .expect("buscar")
            .expect("conta deve existir");
        assert_eq!(row.status, "Estornada");

        // Reembolso ao cliente no valor recebido, como conta a pagar.
        let pagar = query_dispatch(&*app.financeiro, ListarContasPagar::default())
            .await
            .expect("listar CP");
        assert_eq!(pagar.len(), 1, "deve existir uma CP de reembolso");
        let reembolso = &pagar[0];
        assert_eq!(reembolso.valor_original, 30000);
        assert_eq!(reembolso.pedido_id, venda_id, "origem = venda devolvida");
        assert!(
            reembolso
                .descricao
                .as_deref()
                .unwrap_or_default()
                .contains("Reembolso"),
            "descrição deve deixar o reembolso claro: {:?}",
            reembolso.descricao
        );
        assert_eq!(reembolso.status, "Pendente");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn devolucao_parcial_abate_saldo_automaticamente() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        let (venda_id, item_id) = confirmar_venda(
            &app,
            &pool_seed,
            tenant_id,
            FormaPagamento::Prazo { dias: 30 },
        )
        .await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Devolve 1 das 2 unidades (15000 dos 30000).
        dispatch(
            &*app.vendas,
            DevolverItensVenda {
                venda_id,
                itens: vec![DevolucaoItem {
                    item_id,
                    quantidade: 1,
                }],
                motivo: "cliente desistiu".into(),
            },
        )
        .await
        .expect("devolver");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        assert_eq!(contas.len(), 1);
        let conta = &contas[0];
        assert_eq!(conta.valor_original, 30000, "valor original é histórico");
        assert_eq!(conta.valor_abatido, 15000, "devolvido abatido do saldo");
        assert_eq!(conta.status, "Pendente");

        // Nenhum reembolso: nada tinha sido recebido.
        let pagar = query_dispatch(&*app.financeiro, ListarContasPagar::default())
            .await
            .expect("listar CP");
        assert!(pagar.is_empty());

        // O saldo restante (15000) quita a conta.
        dispatch(
            &*app.financeiro,
            RegistrarPagamentoRecebido {
                conta_id: conta.conta_id,
                valor_centavos: 15000,
            },
        )
        .await
        .expect("quitar saldo");
        drenar_outbox(&pool).await.expect("drenar outbox");
        let row = query_dispatch(
            &*app.financeiro,
            BuscarContaReceber {
                conta_id: conta.conta_id,
            },
        )
        .await
        .expect("buscar")
        .expect("conta deve existir");
        assert_eq!(row.status, "Liquidada");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn devolucao_parcial_de_venda_a_vista_gera_reembolso_do_excedente() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        // À vista: conta nasce liquidada (30000 recebidos), saldo em aberto 0.
        let (venda_id, item_id) =
            confirmar_venda(&app, &pool_seed, tenant_id, FormaPagamento::Dinheiro).await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        dispatch(
            &*app.vendas,
            DevolverItensVenda {
                venda_id,
                itens: vec![DevolucaoItem {
                    item_id,
                    quantidade: 1,
                }],
                motivo: "troca".into(),
            },
        )
        .await
        .expect("devolver");
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Sem saldo em aberto, o valor devolvido vira reembolso integral.
        let pagar = query_dispatch(&*app.financeiro, ListarContasPagar::default())
            .await
            .expect("listar CP");
        assert_eq!(pagar.len(), 1);
        assert_eq!(pagar[0].valor_original, 15000);
        assert_eq!(pagar[0].pedido_id, venda_id);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn abatimento_manual_reflete_na_projecao_e_respeita_saldo() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    let pool_seed = pool.clone();
    in_tenant(tenant_id, async move {
        confirmar_venda_a_prazo(&app, &pool_seed, tenant_id).await;
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        let conta_id = contas[0].conta_id;

        // Abatimento maior que o saldo em aberto → erro de negócio.
        let r = dispatch(
            &*app.financeiro,
            RegistrarAbatimentoContaReceber {
                conta_id,
                valor_centavos: 30001,
                motivo: "desconto".into(),
            },
        )
        .await;
        assert!(
            matches!(r, Err(DispatchError::Handler(AppError::Domain(_)))),
            "abatimento acima do saldo deve falhar, veio {r:?}"
        );

        // Abatimento válido reflete na projeção.
        dispatch(
            &*app.financeiro,
            RegistrarAbatimentoContaReceber {
                conta_id,
                valor_centavos: 5000,
                motivo: "desconto negociado".into(),
            },
        )
        .await
        .expect("abater");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let row = query_dispatch(&*app.financeiro, BuscarContaReceber { conta_id })
            .await
            .expect("buscar")
            .expect("conta deve existir");
        assert_eq!(row.valor_abatido, 5000);
        assert_eq!(row.valor_original, 30000);
        assert_eq!(row.status, "Pendente");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn reprocessar_venda_confirmada_nao_duplica_conta() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = new_tenant_id();
    in_tenant(tenant_id, async move {
        let financeiro = app.financeiro.clone();
        let handler = FinanceiroVendaEventHandler { financeiro };
        let evento = VendaEvent::VendaConfirmada {
            venda_id: Uuid::new_v4().to_string(),
            vendedor_id: Uuid::new_v4().to_string(),
            cliente_id: None,
            itens: vec![],
            total_centavos: 12000,
            desconto_centavos: 0,
            forma_pagamento: FormaPagamento::Prazo { dias: 15 },
            occurred_at: Utc::now(),
        };

        // Entrega at-least-once simulada: o MESMO evento processado duas vezes.
        handler.handle(&evento).await.expect("primeira entrega");
        drenar_outbox(&pool).await.expect("drenar outbox");
        handler.handle(&evento).await.expect("segunda entrega");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasReceber::default())
            .await
            .expect("listar");
        assert_eq!(
            contas.len(),
            1,
            "reprocessamento não deve duplicar a conta (id determinístico)"
        );
        assert_eq!(contas[0].valor_original, 12000);
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
        drenar_outbox(&pool).await.expect("drenar outbox");

        let contas = query_dispatch(&*app.financeiro, ListarContasPagar::default())
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
        drenar_outbox(&pool).await.expect("drenar outbox");

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
