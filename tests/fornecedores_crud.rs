#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD completo do módulo Fornecedores: command handlers, query handlers
/// e o adaptador PostgresFornecedorRepository.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, setup_db, start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use pharos_core::Repository;
use finledger::error::AppError;
use finledger::fornecedores::application::commands::{
    AtualizarFornecedor, CadastrarFornecedor, DesativarFornecedor, ReativarFornecedor,
};
use finledger::fornecedores::application::queries::{BuscarFornecedor, ListarFornecedores};
use finledger::fornecedores::domain::fornecedor::FornecedorId;
use uuid::Uuid;

fn cmd_cadastrar() -> CadastrarFornecedor {
    CadastrarFornecedor {
        razao_social: "Distribuidora Brasil Ltda".into(),
        cnpj: "12.345.678/0001-95".into(),
        telefone: Some("(11) 4000-1234".into()),
        email: Some("contato@distribuidorabrasil.com".into()),
        prazo_pagamento_dias: 28,
    }
}

#[tokio::test]
async fn ciclo_completo_de_crud_do_fornecedor() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        // Create
        let id = dispatch(&*app.fornecedores, cmd_cadastrar())
            .await
            .expect("cadastrar");
        aguardar_projecoes().await;

        // Read
        let lista = query_dispatch(&*app.fornecedores, ListarFornecedores)
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let row = query_dispatch(
            &*app.fornecedores,
            BuscarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("fornecedor deve existir");
        assert_eq!(row.razao_social, "Distribuidora Brasil Ltda");
        assert_eq!(row.cnpj, "12345678000195");
        assert_eq!(row.prazo_pagamento_dias, 28);
        assert!(row.ativo);

        // Update
        dispatch(
            &*app.fornecedores,
            AtualizarFornecedor {
                fornecedor_id: id.as_uuid(),
                razao_social: "Nova Razão SA".into(),
                telefone: None,
                email: None,
                prazo_pagamento_dias: 45,
            },
        )
        .await
        .expect("atualizar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.fornecedores,
            BuscarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("fornecedor deve existir");
        assert_eq!(row.razao_social, "Nova Razão SA");
        assert_eq!(row.prazo_pagamento_dias, 45);

        // Delete lógico + reativação
        dispatch(
            &*app.fornecedores,
            DesativarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("desativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.fornecedores,
            BuscarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("fornecedor deve existir");
        assert!(!row.ativo);

        dispatch(
            &*app.fornecedores,
            ReativarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("reativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.fornecedores,
            BuscarFornecedor {
                fornecedor_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("fornecedor deve existir");
        assert!(row.ativo);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn comandos_sobre_fornecedor_inexistente_retornam_not_found() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let r = dispatch(
            &*app.fornecedores,
            DesativarFornecedor {
                fornecedor_id: Uuid::new_v4(),
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
        let id = dispatch(&*app.fornecedores, cmd_cadastrar())
            .await
            .expect("cadastrar");

        let repo =
            finledger::fornecedores::infrastructure::repository::PostgresFornecedorRepository::new(
                pool.clone(),
            );
        let fornecedor = repo
            .find_by_id(&FornecedorId::from_uuid(id.as_uuid()))
            .await
            .expect("find_by_id")
            .expect("agregado deve existir");
        assert_eq!(fornecedor.cnpj.as_str(), "12345678000195");
        assert!(fornecedor.ativo);
    })
    .await;
    Ok(())
}