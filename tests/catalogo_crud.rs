#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD completo do módulo Catálogo: command handlers, query handlers
/// e o adaptador PostgresProdutoRepository (event store + projeção).
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, setup_db, start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use pharos_core::Repository;
use finledger::catalogo::application::commands::{
    AtualizarPrecos, AtualizarProduto, CadastrarProduto, DesativarProduto, ReativarProduto,
};
use finledger::catalogo::application::queries::{BuscarProduto, ListarProdutos};
use finledger::catalogo::domain::produto::ProdutoId;
use finledger::error::AppError;
use uuid::Uuid;

fn cmd_cadastrar(sku: &str) -> CadastrarProduto {
    CadastrarProduto {
        sku: sku.into(),
        descricao: "Pastilha de freio".into(),
        ncm: "87083090".into(),
        unidade: "UN".into(),
        preco_custo_centavos: 5000,
        preco_venda_centavos: 9000,
        categoria: "Freios".into(),
        marca: None,
        controla_estoque: true,
        classe_trib: None,
    }
}

#[tokio::test]
async fn ciclo_completo_de_crud_do_produto() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        // Create
        let id = dispatch(&*app.catalogo, cmd_cadastrar("SKU-001"))
            .await
            .expect("cadastrar");
        aguardar_projecoes().await;

        // Read (query handlers sobre a projeção)
        let lista = query_dispatch(&*app.catalogo, ListarProdutos)
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);

        let row = query_dispatch(
            &*app.catalogo,
            BuscarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("produto deve existir");
        assert_eq!(row.sku, "SKU-001");
        assert_eq!(row.preco_venda, 9000);
        assert!(row.ativo);

        // Update (dados + preços)
        dispatch(
            &*app.catalogo,
            AtualizarProduto {
                produto_id: id.as_uuid(),
                sku: "SKU-002".into(),
                descricao: "Disco de freio".into(),
                ncm: "87083090".into(),
                unidade: "UN".into(),
                categoria: "Freios".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            },
        )
        .await
        .expect("atualizar");
        dispatch(
            &*app.catalogo,
            AtualizarPrecos {
                produto_id: id.as_uuid(),
                preco_custo_centavos: 6000,
                preco_venda_centavos: 12000,
            },
        )
        .await
        .expect("atualizar preços");
        aguardar_projecoes().await;

        let row = query_dispatch(
            &*app.catalogo,
            BuscarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("produto deve existir");
        assert_eq!(row.sku, "SKU-002");
        assert_eq!(row.descricao, "Disco de freio");
        assert_eq!(row.preco_venda, 12000);

        // Delete lógico + reativação
        dispatch(
            &*app.catalogo,
            DesativarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("desativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.catalogo,
            BuscarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("produto deve existir");
        assert!(!row.ativo);

        dispatch(
            &*app.catalogo,
            ReativarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("reativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.catalogo,
            BuscarProduto {
                produto_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("produto deve existir");
        assert!(row.ativo);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn atualizar_produto_inexistente_retorna_not_found() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let r = dispatch(
            &*app.catalogo,
            DesativarProduto {
                produto_id: Uuid::new_v4(),
            },
        )
        .await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::NotFound))));
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn repositorio_persiste_e_recarrega_agregado() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let id = dispatch(&*app.catalogo, cmd_cadastrar("SKU-REPO"))
            .await
            .expect("cadastrar");

        // Adaptador de infraestrutura: reidrata o agregado do event store
        let repo = finledger::catalogo::infrastructure::repository::PostgresProdutoRepository::new(
            pool.clone(),
        );
        let produto = repo
            .find_by_id(&ProdutoId::from_uuid(id.as_uuid()))
            .await
            .expect("find_by_id")
            .expect("agregado deve existir");
        assert_eq!(produto.sku().as_str(), "SKU-REPO");
        assert!(produto.ativo());

        let inexistente = repo
            .find_by_id(&ProdutoId::new())
            .await
            .expect("find_by_id");
        assert!(inexistente.is_none());
    })
    .await;
    Ok(())
}