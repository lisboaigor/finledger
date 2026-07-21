#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do plano de controle: TenantRepository e BackofficeRepository/Handlers.
mod helpers;
use helpers::{TestResult, montar_app, setup_db, start_postgres};

use finledger::backoffice::domain::{BackofficePermission, TenantPlan};
use finledger::backoffice::handlers::{
    AlterarPermissoesCmd, ChangeAdminPasswordCmd, CriarAdminCmd, LoginBackofficeCmd,
};
use finledger::error::AppError;
use uuid::Uuid;

#[tokio::test]
async fn ciclo_completo_de_crud_do_tenant() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    // Create
    let tenant_id = app.tenants.criar("empresa-x", "Empresa X").await?;

    // Read
    let lista = app.tenants.listar().await?;
    assert_eq!(lista.len(), 1);
    let t = app
        .tenants
        .buscar_por_slug("empresa-x")
        .await?
        .expect("tenant existe");
    assert_eq!(t.nome, "Empresa X");
    assert_eq!(t.status, "ativo");

    // Update: nome e plano
    app.backoffice
        .atualizar_tenant(tenant_id, "Empresa X Premium".into())
        .await?;
    app.backoffice
        .alterar_plano(tenant_id, TenantPlan::Enterprise)
        .await?;
    let t = app
        .tenants
        .buscar_por_id(tenant_id)
        .await?
        .expect("tenant existe");
    assert_eq!(t.nome, "Empresa X Premium");
    assert_eq!(t.plano, "enterprise");

    // Suspensão / reativação
    app.tenants.suspender(tenant_id).await?;
    let t = app
        .tenants
        .buscar_por_id(tenant_id)
        .await?
        .expect("tenant existe");
    assert_eq!(t.status, "suspenso");
    app.tenants.reativar(tenant_id).await?;
    let t = app
        .tenants
        .buscar_por_id(tenant_id)
        .await?
        .expect("tenant existe");
    assert_eq!(t.status, "ativo");

    // Nome vazio → validação
    let r = app
        .backoffice
        .atualizar_tenant(tenant_id, "   ".into())
        .await;
    assert!(matches!(r, Err(AppError::Domain(_))));

    // Tenant inexistente → NotFound
    let r = app
        .backoffice
        .atualizar_tenant(Uuid::new_v4(), "X".into())
        .await;
    assert!(matches!(r, Err(AppError::NotFound)));

    // Slug duplicado no provisionamento estrito → erro de validação
    let r = app.tenants.create_strict("empresa-x", "Outra Empresa").await;
    assert!(matches!(r, Err(AppError::Domain(_))));

    // Compensação de provisionamento: tenant recém-criado pode ser removido
    let orphan_id = app.tenants.create_strict("empresa-y", "Empresa Y").await?;
    app.tenants.delete(orphan_id).await?;
    assert!(app.tenants.buscar_por_slug("empresa-y").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn faturamento_cross_tenant_no_backoffice() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    let t1 = app.tenants.create_strict("empresa-a", "Empresa A").await?;
    let t2 = app.tenants.create_strict("empresa-b", "Empresa B").await?;

    // Sales are written straight into the projection — the test pool is the
    // superuser, so RLS doesn't apply here.
    async fn insert_sale(
        pool: &pharos_postgres::Pool,
        tenant: Uuid,
        cents: i64,
        status: &str,
        confirmed_days_ago: Option<i64>,
    ) -> TestResult {
        sqlx::query(
            "INSERT INTO proj_vendas
               (venda_id, tenant_id, vendedor_id, total_centavos, status,
                criada_em, confirmada_em, atualizado_em)
             VALUES ($1, $2, $3, $4, $5, NOW(),
                     CASE WHEN $6::BIGINT IS NULL THEN NULL
                          ELSE NOW() - make_interval(days => $6::BIGINT::INT) END,
                     NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(tenant)
        .bind(Uuid::new_v4())
        .bind(cents)
        .bind(status)
        .bind(confirmed_days_ago)
        .execute(pool)
        .await?;
        Ok(())
    }

    insert_sale(&pool, t1, 10_000, "confirmada", Some(1)).await?;
    insert_sale(&pool, t1, 5_000, "confirmada", Some(2)).await?;
    insert_sale(&pool, t1, 99_999, "cancelada", None).await?;
    insert_sale(&pool, t2, 20_000, "confirmada", Some(45)).await?;

    let overview = app.backoffice.revenue_overview(12, 30).await?;

    let r1 = overview
        .tenants
        .iter()
        .find(|t| t.tenant_id == t1)
        .expect("tenant A presente");
    assert_eq!(r1.total_cents, 15_000);
    assert_eq!(r1.sales_count, 2);
    assert_eq!(r1.last_30d_cents, 15_000);
    assert_eq!(r1.prev_30d_cents, 0);
    assert_eq!(r1.avg_ticket_cents, 7_500);

    let r2 = overview
        .tenants
        .iter()
        .find(|t| t.tenant_id == t2)
        .expect("tenant B presente");
    assert_eq!(r2.total_cents, 20_000);
    assert_eq!(r2.last_30d_cents, 0);
    // Venda de 45 dias atrás cai na janela "30 dias anteriores" (30–60).
    assert_eq!(r2.prev_30d_cents, 20_000);

    // Série mensal global: soma tudo que está na janela de 12 meses.
    let monthly_total: i64 = overview.monthly.iter().map(|m| m.total_cents).sum();
    assert_eq!(monthly_total, 35_000);

    // Série diária (30 dias): só as vendas recentes do tenant A.
    let daily_total: i64 = overview.daily.iter().map(|d| d.total_cents).sum();
    assert_eq!(daily_total, 15_000);

    // Série mensal por tenant alimenta os sparklines.
    let t1_monthly: i64 = overview
        .monthly_by_tenant
        .iter()
        .filter(|m| m.tenant_id == t1)
        .map(|m| m.total_cents)
        .sum();
    assert_eq!(t1_monthly, 15_000);

    // Contadores de plataforma: nenhum usuário/produto/cliente nas projeções.
    assert_eq!(overview.stats.total_users, 0);
    assert_eq!(overview.stats.today_count, 0);

    Ok(())
}

#[tokio::test]
async fn ciclo_completo_de_crud_do_admin_backoffice() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    // Senha curta → validação
    let r = app
        .backoffice
        .criar_admin(CriarAdminCmd {
            username: "suporte1".into(),
            senha: "curta".into(),
            permissions: vec![BackofficePermission::TenantsRead],
        })
        .await;
    assert!(matches!(r, Err(AppError::Domain(_))));

    // Create
    let admin_id = app
        .backoffice
        .criar_admin(CriarAdminCmd {
            username: "suporte1".into(),
            senha: "senha-backoffice".into(),
            permissions: vec![BackofficePermission::TenantsRead],
        })
        .await?;

    // Read
    let admins = app.backoffice.listar_admins().await?;
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0].username, "suporte1");
    assert!(admins[0].ativo);

    // Login
    let token = app
        .backoffice
        .login(LoginBackofficeCmd {
            username: "suporte1".into(),
            senha: "senha-backoffice".into(),
        })
        .await?;
    assert!(!token.is_empty());
    let r = app
        .backoffice
        .login(LoginBackofficeCmd {
            username: "suporte1".into(),
            senha: "errada".into(),
        })
        .await;
    assert!(matches!(r, Err(AppError::Unauthorized)));

    // Update: permissões
    app.backoffice
        .alterar_permissoes(
            admin_id,
            AlterarPermissoesCmd {
                permissions: vec![
                    BackofficePermission::TenantsRead,
                    BackofficePermission::TenantsWrite,
                ],
            },
        )
        .await?;
    let admins = app.backoffice.listar_admins().await?;
    assert_eq!(admins[0].permissions.len(), 2);

    // Troca de senha: antiga deixa de valer, nova passa a valer
    app.backoffice
        .change_admin_password(
            admin_id,
            ChangeAdminPasswordCmd {
                password: "nova-senha-forte".into(),
            },
        )
        .await?;
    let r = app
        .backoffice
        .login(LoginBackofficeCmd {
            username: "suporte1".into(),
            senha: "senha-backoffice".into(),
        })
        .await;
    assert!(matches!(r, Err(AppError::Unauthorized)));
    app.backoffice
        .login(LoginBackofficeCmd {
            username: "suporte1".into(),
            senha: "nova-senha-forte".into(),
        })
        .await?;

    // Senha curta na troca → validação
    let r = app
        .backoffice
        .change_admin_password(
            admin_id,
            ChangeAdminPasswordCmd {
                password: "curta".into(),
            },
        )
        .await;
    assert!(matches!(r, Err(AppError::Domain(_))));

    // Troca de senha de admin inexistente → NotFound
    let r = app
        .backoffice
        .change_admin_password(
            Uuid::new_v4(),
            ChangeAdminPasswordCmd {
                password: "nova-senha-forte".into(),
            },
        )
        .await;
    assert!(matches!(r, Err(AppError::NotFound)));

    // Desativação / reativação
    app.backoffice.desativar_admin(admin_id).await?;
    let admins = app.backoffice.listar_admins().await?;
    assert!(!admins[0].ativo);

    // Login de admin desativado → Unauthorized (mesmo com a senha correta)
    let r = app
        .backoffice
        .login(LoginBackofficeCmd {
            username: "suporte1".into(),
            senha: "nova-senha-forte".into(),
        })
        .await;
    assert!(matches!(r, Err(AppError::Unauthorized)));

    app.backoffice.reativar_admin(admin_id).await?;
    let admins = app.backoffice.listar_admins().await?;
    assert!(admins[0].ativo);

    // Reativar admin inexistente → NotFound
    let r = app.backoffice.reativar_admin(Uuid::new_v4()).await;
    assert!(matches!(r, Err(AppError::NotFound)));

    Ok(())
}