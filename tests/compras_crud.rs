#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Compras: fluxo completo de comandos, queries e efeitos
/// cross-BC (estoque e contas a pagar) já cobertos parcialmente em
/// cross_bc_integration — aqui o foco é o ciclo do pedido e as queries.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, setup_db, start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::compras::application::commands::{
    AprovarPedidoCompra, CancelarPedidoCompra, EnviarPedidoCompra, GerarPedidoCompra,
    ItemPedidoInput, ItemRecebidoInput, ReceberMercadoria,
};
use finledger::compras::application::queries::{BuscarPedidoCompra, ListarPedidosCompra};
use finledger::error::AppError;
use uuid::Uuid;

fn cmd_gerar(produto_id: Uuid) -> GerarPedidoCompra {
    GerarPedidoCompra {
        comprador_id: Uuid::new_v4(),
        fornecedor_id: Uuid::new_v4(),
        itens: vec![ItemPedidoInput {
            produto_id,
            quantidade: 20,
            custo_unitario_centavos: 500,
        }],
        prazo_pagamento_dias: 30,
    }
}

#[tokio::test]
async fn ciclo_completo_do_pedido_de_compra() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let produto_id = Uuid::new_v4();
        let pedido_id = dispatch(&*app.compras, cmd_gerar(produto_id))
            .await
            .expect("gerar");
        aguardar_projecoes().await;

        let lista = query_dispatch(&*app.compras, ListarPedidosCompra::default())
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let detalhes = query_dispatch(
            &*app.compras,
            BuscarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("pedido deve existir");
        assert_eq!(detalhes.pedido.total_centavos, 10000);
        assert_eq!(detalhes.itens.len(), 1);

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
                    quantidade: 20,
                }],
            },
        )
        .await
        .expect("receber");
        aguardar_projecoes().await;

        let detalhes = query_dispatch(
            &*app.compras,
            BuscarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("pedido deve existir");
        assert_eq!(detalhes.pedido.status, "RecebidoTotal");

        // Efeitos cross-BC: entrada de estoque + conta a pagar
        let saldo: i32 =
            sqlx::query_scalar("SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1")
                .bind(produto_id)
                .fetch_one(&pool)
                .await
                .expect("saldo");
        assert_eq!(saldo, 20);
        let contas: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM proj_contas_pagar WHERE pedido_id = $1")
                .bind(pedido_id.as_uuid())
                .fetch_one(&pool)
                .await
                .expect("contas");
        assert_eq!(contas, 1);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn cancelar_pedido_pendente() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let pedido_id = dispatch(&*app.compras, cmd_gerar(Uuid::new_v4()))
            .await
            .expect("gerar");
        dispatch(
            &*app.compras,
            CancelarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
                motivo: "fornecedor sem prazo".into(),
            },
        )
        .await
        .expect("cancelar");
        aguardar_projecoes().await;

        let detalhes = query_dispatch(
            &*app.compras,
            BuscarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("pedido deve existir");
        assert_eq!(detalhes.pedido.status, "Cancelado");

        // Receber depois de cancelado → regra de negócio
        let r = dispatch(
            &*app.compras,
            EnviarPedidoCompra {
                pedido_id: pedido_id.as_uuid(),
            },
        )
        .await;
        assert!(matches!(
            r,
            Err(DispatchError::Handler(AppError::Domain(_)))
        ));
    })
    .await;
    Ok(())
}