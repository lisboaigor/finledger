#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD completo do módulo CRM: command handlers, query handlers
/// e o adaptador PostgresClienteRepository.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, in_tenant, montar_app, new_tenant_id, setup_db, start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use pharos_core::Repository;
use finledger::crm::application::commands::{
    AtualizarCliente, BloquearCliente, CadastrarCliente, DesativarCliente, DesbloquearCliente,
    ReativarCliente,
};
use finledger::crm::application::queries::{BuscarCliente, ListarClientes};
use finledger::crm::domain::cliente::ClienteId;
use finledger::error::AppError;
use uuid::Uuid;

fn cmd_cadastrar() -> CadastrarCliente {
    CadastrarCliente {
        nome: "João da Silva".into(),
        cpf_cnpj: "123.456.789-09".into(),
        telefone: Some("(11) 99999-0000".into()),
        email: Some("joao@exemplo.com".into()),
    }
}

#[tokio::test]
async fn ciclo_completo_de_crud_do_cliente() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        // Create
        let id = dispatch(&*app.crm, cmd_cadastrar())
            .await
            .expect("cadastrar");
        aguardar_projecoes().await;

        // Read
        let lista = query_dispatch(&*app.crm, ListarClientes)
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let row = query_dispatch(
            &*app.crm,
            BuscarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("cliente deve existir");
        assert_eq!(row.nome, "João da Silva");
        assert_eq!(row.cpf_cnpj, "12345678909");
        assert!(!row.bloqueado);
        assert!(row.ativo);

        // Update
        dispatch(
            &*app.crm,
            AtualizarCliente {
                cliente_id: id.as_uuid(),
                nome: "Maria Souza".into(),
                telefone: None,
                email: Some("maria@exemplo.com".into()),
            },
        )
        .await
        .expect("atualizar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.crm,
            BuscarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("cliente deve existir");
        assert_eq!(row.nome, "Maria Souza");
        assert_eq!(row.email.as_deref(), Some("maria@exemplo.com"));

        // Bloqueio / desbloqueio
        dispatch(
            &*app.crm,
            BloquearCliente {
                cliente_id: id.as_uuid(),
                motivo: "inadimplente".into(),
            },
        )
        .await
        .expect("bloquear");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.crm,
            BuscarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("cliente deve existir");
        assert!(row.bloqueado);

        dispatch(
            &*app.crm,
            DesbloquearCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("desbloquear");

        // Delete lógico + reativação
        dispatch(
            &*app.crm,
            DesativarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("desativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.crm,
            BuscarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("cliente deve existir");
        assert!(!row.ativo);

        dispatch(
            &*app.crm,
            ReativarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("reativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.crm,
            BuscarCliente {
                cliente_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("cliente deve existir");
        assert!(row.ativo);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn comandos_sobre_cliente_inexistente_retornam_not_found() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let inexistente = Uuid::new_v4();
        let r = dispatch(
            &*app.crm,
            DesativarCliente {
                cliente_id: inexistente,
            },
        )
        .await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::NotFound))));
        let r = dispatch(
            &*app.crm,
            BloquearCliente {
                cliente_id: inexistente,
                motivo: "x".into(),
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
        let id = dispatch(&*app.crm, cmd_cadastrar())
            .await
            .expect("cadastrar");

        let repo =
            finledger::crm::infrastructure::repository::PostgresClienteRepository::new(pool.clone());
        let cliente = repo
            .find_by_id(&ClienteId::from_uuid(id.as_uuid()))
            .await
            .expect("find_by_id")
            .expect("agregado deve existir");
        assert_eq!(cliente.nome.to_string(), "João da Silva");
        assert!(cliente.ativo);
        assert!(!cliente.bloqueado);
    })
    .await;
    Ok(())
}