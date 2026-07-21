#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD completo do módulo Identity: command handlers (registrar, login,
/// alterar senha, roles, desativar/reativar), query handlers e repositório.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, create_tenant, in_tenant, montar_app, new_tenant_id, setup_db,
    start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::error::AppError;
use finledger::identity::application::commands::{
    AlterarSenha, AtualizarUsuario, DesativarUsuario, Login, ReativarUsuario, RegistrarUsuario,
};
use finledger::identity::application::queries::{BuscarUsuario, ListarUsuarios};

fn cmd_registrar(username: &str) -> RegistrarUsuario {
    RegistrarUsuario {
        username: username.into(),
        senha: "senha-forte-123".into(),
        roles: vec!["vendedor".into()],
    }
}

#[tokio::test]
async fn ciclo_completo_de_crud_do_usuario() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        // Create
        let id = dispatch(&*app.identity, cmd_registrar("carlos"))
            .await
            .expect("registrar");
        aguardar_projecoes().await;

        // Read
        let lista = query_dispatch(&*app.identity, ListarUsuarios)
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let row = query_dispatch(
            &*app.identity,
            BuscarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("usuário deve existir");
        assert_eq!(row.username, "carlos");
        assert_eq!(row.roles, "vendedor");
        assert!(row.ativo);

        // Update (roles)
        dispatch(
            &*app.identity,
            AtualizarUsuario {
                usuario_id: id.as_uuid(),
                roles: vec!["vendedor".into(), "estoquista".into()],
            },
        )
        .await
        .expect("atualizar roles");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.identity,
            BuscarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("usuário deve existir");
        assert_eq!(row.roles, "vendedor,estoquista");

        // Alterar senha: senha atual errada → Unauthorized
        let r = dispatch(
            &*app.identity,
            AlterarSenha {
                usuario_id: id.as_uuid(),
                senha_atual: "errada".into(),
                nova_senha: "outra".into(),
            },
        )
        .await;
        assert!(matches!(
            r,
            Err(DispatchError::Handler(AppError::Unauthorized))
        ));

        // Alterar senha com senha atual correta
        dispatch(
            &*app.identity,
            AlterarSenha {
                usuario_id: id.as_uuid(),
                senha_atual: "senha-forte-123".into(),
                nova_senha: "nova-senha-456".into(),
            },
        )
        .await
        .expect("alterar senha");

        // Delete lógico + reativação
        dispatch(
            &*app.identity,
            DesativarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("desativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.identity,
            BuscarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("usuário deve existir");
        assert!(!row.ativo);

        dispatch(
            &*app.identity,
            ReativarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("reativar");
        aguardar_projecoes().await;
        let row = query_dispatch(
            &*app.identity,
            BuscarUsuario {
                usuario_id: id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("usuário deve existir");
        assert!(row.ativo);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn registrar_username_duplicado_retorna_conflict() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        dispatch(&*app.identity, cmd_registrar("carlos"))
            .await
            .expect("registrar");
        aguardar_projecoes().await;
        let r = dispatch(&*app.identity, cmd_registrar("carlos")).await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::Conflict))));
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn login_com_credenciais_validas_retorna_token() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let tenant_id = create_tenant(&pool, "acme").await?;
    in_tenant(tenant_id, async {
        dispatch(&*app.identity, cmd_registrar("carlos"))
            .await
            .expect("registrar");
        aguardar_projecoes().await;
    })
    .await;

    // Login resolve o tenant pelo slug — não precisa de escopo externo.
    let token = dispatch(
        &*app.identity,
        Login {
            slug: "acme".into(),
            username: "carlos".into(),
            senha: "senha-forte-123".into(),
        },
    )
    .await
    .expect("login");
    assert!(!token.is_empty());

    // Senha errada → Unauthorized
    let r = dispatch(
        &*app.identity,
        Login {
            slug: "acme".into(),
            username: "carlos".into(),
            senha: "errada".into(),
        },
    )
    .await;
    assert!(matches!(
        r,
        Err(DispatchError::Handler(AppError::Unauthorized))
    ));

    Ok(())
}