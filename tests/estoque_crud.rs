#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Estoque: entrada, ajuste, baixa, estoque mínimo,
/// query handlers e repositório.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, setup_db, start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::error::AppError;
use finledger::estoque::application::commands::{
    AjustarEstoque, BaixarEstoque, DefinirEstoqueMinimo, RegistrarEntradaEstoque,
};
use finledger::estoque::application::queries::{BuscarSaldo, ListarSaldos};
use uuid::Uuid;

#[tokio::test]
async fn ciclo_completo_de_movimentacao_de_estoque() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let produto_id = Uuid::new_v4();

        // Entrada
        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id,
                quantidade: 10,
                custo_unitario_centavos: 2000,
                motivo: "compra inicial".into(),
                nota_fiscal: None,
            },
        )
        .await
        .expect("entrada");
        aguardar_projecoes().await;

        let lista = query_dispatch(&*app.estoque, ListarSaldos)
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let saldo = query_dispatch(&*app.estoque, BuscarSaldo { produto_id })
            .await
            .expect("buscar")
            .expect("saldo deve existir");
        assert_eq!(saldo.quantidade, 10);
        assert_eq!(saldo.custo_medio, 2000);

        // Baixa
        dispatch(
            &*app.estoque,
            BaixarEstoque {
                produto_id,
                quantidade: 3,
                referencia_id: None,
            },
        )
        .await
        .expect("baixa");
        aguardar_projecoes().await;
        let saldo = query_dispatch(&*app.estoque, BuscarSaldo { produto_id })
            .await
            .expect("buscar")
            .expect("saldo deve existir");
        assert_eq!(saldo.quantidade, 7);

        // Baixa maior que o saldo → regra de negócio
        let r = dispatch(
            &*app.estoque,
            BaixarEstoque {
                produto_id,
                quantidade: 100,
                referencia_id: None,
            },
        )
        .await;
        assert!(matches!(
            r,
            Err(DispatchError::Handler(AppError::Domain(_)))
        ));

        // Ajuste para cima (7 → 50): exige custo das 43 unidades acrescentadas
        // e repondera o custo médio. (2000*7 + 3000*43) / 50 = 2860.
        dispatch(
            &*app.estoque,
            AjustarEstoque {
                produto_id,
                quantidade_nova: 50,
                custo_unitario_centavos: Some(3000),
                justificativa: "inventário físico".into(),
            },
        )
        .await
        .expect("ajuste");
        aguardar_projecoes().await;
        let saldo = query_dispatch(&*app.estoque, BuscarSaldo { produto_id })
            .await
            .expect("buscar")
            .expect("saldo deve existir");
        assert_eq!(saldo.quantidade, 50);
        assert_eq!(saldo.custo_medio, 2860);

        // Ajuste para cima SEM custo → erro de validação de domínio.
        let r = dispatch(
            &*app.estoque,
            AjustarEstoque {
                produto_id,
                quantidade_nova: 60,
                custo_unitario_centavos: None,
                justificativa: "sem custo".into(),
            },
        )
        .await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::Domain(_)))));

        // Ajuste para baixo dispensa custo e mantém o custo médio.
        dispatch(
            &*app.estoque,
            AjustarEstoque {
                produto_id,
                quantidade_nova: 20,
                custo_unitario_centavos: None,
                justificativa: "perda".into(),
            },
        )
        .await
        .expect("ajuste para baixo");
        aguardar_projecoes().await;
        let saldo = query_dispatch(&*app.estoque, BuscarSaldo { produto_id })
            .await
            .expect("buscar")
            .expect("saldo deve existir");
        assert_eq!(saldo.quantidade, 20);
        assert_eq!(saldo.custo_medio, 2860);

        // Estoque mínimo
        dispatch(
            &*app.estoque,
            DefinirEstoqueMinimo {
                produto_id,
                estoque_minimo: 5,
            },
        )
        .await
        .expect("definir mínimo");
        aguardar_projecoes().await;
        let saldo = query_dispatch(&*app.estoque, BuscarSaldo { produto_id })
            .await
            .expect("buscar")
            .expect("saldo deve existir");
        assert_eq!(saldo.estoque_minimo, 5);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn ajuste_sem_justificativa_retorna_erro_de_validacao() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let r = dispatch(
            &*app.estoque,
            AjustarEstoque {
                produto_id: Uuid::new_v4(),
                quantidade_nova: 10,
                custo_unitario_centavos: None,
                justificativa: "   ".into(),
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