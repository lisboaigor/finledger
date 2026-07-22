#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Testes de isolamento cross-tenant.
/// Garantem que dados de um tenant não são visíveis nem modificáveis por outro.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, create_tenant, in_tenant, new_tenant_id, seed_produto,
    setup_db, start_postgres, start_postgres_with_url, suspend_tenant,
};

use std::sync::Arc;

use chrono::Utc;
use pharos_app::{CommandHandler, EventBus};
use pharos_postgres::connect_pool;
use finledger::auth::AuthConfig;
use finledger::catalogo::application::commands::CadastrarProduto;
use finledger::catalogo::infrastructure::precificacao_repository::PostgresPrecificacaoRepository;
use finledger::catalogo::infrastructure::repository::PostgresProdutoRepository;
use finledger::crm::application::commands::CadastrarCliente;
use finledger::crm::infrastructure::repository::PostgresClienteRepository;
use finledger::error::AppError;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::estoque::infrastructure::repository::PostgresEstoqueRepository;
use finledger::financeiro::infrastructure::repository::{
    PostgresContaPagarRepository, PostgresContaReceberRepository,
};
use finledger::fiscal::infrastructure::{
    aliquotas::PostgresAliquotaProvider, repository::PostgresNotaFiscalRepository,
    sefaz::StubSefazClient,
};
use finledger::identity::application::commands::{Login, RegistrarUsuario};
use finledger::identity::application::handler::IdentityHandlers;
use finledger::identity::infrastructure::repository::PostgresIdentityRepository;
use finledger::projections::identity::IdentityProjection;
use finledger::tenants::repository::TenantRepository;
use finledger::vendas::application::commands::{
    AdicionarItemVenda, ConfirmarVenda, DefinirFormaPagamento, IniciarVenda,
};
use finledger::vendas::domain::value_objects::FormaPagamento;
use finledger::vendas::infrastructure::repository::PostgresVendaRepository;
use finledger::{
    catalogo::application::handler::CatalogoHandlers,
    crm::application::handler::CrmHandlers,
    estoque::application::{
        event_handlers::EstoqueVendaEventHandler, handler::EstoqueHandlers,
    },
    financeiro::application::{
        event_handlers::FinanceiroVendaEventHandler, handler::FinanceiroHandlers,
    },
    fiscal::application::{event_handlers::FiscalVendaEventHandler, handler::FiscalHandlers},
    projections::{
        catalogo::CatalogoProjection, crm::CrmProjection, estoque::EstoqueProjection,
        financeiro::FinanceiroProjection, fiscal::FiscalProjection,
    },
    vendas::application::handler::VendasHandlers,
};
use uuid::Uuid;

// ── Isolamento de leitura ─────────────────────────────────────────────────────

/// Produto cadastrado no tenant A não aparece na listagem do tenant B.
#[tokio::test]
async fn produto_de_a_invisivel_para_b() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(CatalogoProjection::new(pool.clone()));
    let repo = Arc::new(PostgresProdutoRepository::new(pool.clone()));
    let precificacao = Arc::new(PostgresPrecificacaoRepository::new(pool.clone()));
    let catalogo = CatalogoHandlers::new(repo.clone(), precificacao, bus);

    let id_a = new_tenant_id();
    let id_b = new_tenant_id();

    in_tenant(id_a, async {
        catalogo
            .handle(CadastrarProduto {
                sku: "SKU-A".into(),
                descricao: "Produto do tenant A".into(),
                ncm: "87083000".into(),
                unidade: "UN".into(),
                preco_custo_centavos: 1000,
                preco_venda_centavos: 2000,
                categoria: "Teste".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            })
            .await
            .expect("cadastrar produto A falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let produtos_b = in_tenant(id_b, async { repo.listar().await.expect("listar falhou") }).await;

    assert!(
        produtos_b.is_empty(),
        "tenant B não deve ver produtos do tenant A"
    );
    Ok(())
}

/// Cliente cadastrado no tenant A não aparece na listagem do tenant B.
#[tokio::test]
async fn cliente_de_a_invisivel_para_b() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(CrmProjection::new(pool.clone()));
    let repo = Arc::new(PostgresClienteRepository::new(pool.clone()));
    let crm = CrmHandlers::new(repo.clone(), bus);

    let id_a = new_tenant_id();
    let id_b = new_tenant_id();

    in_tenant(id_a, async {
        crm.handle(CadastrarCliente {
            nome: "Cliente do tenant A".into(),
            cpf_cnpj: "11111111111".into(),
            telefone: None,
            email: None,
        })
        .await
        .expect("cadastrar cliente A falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let clientes_b = in_tenant(id_b, async { repo.listar().await.expect("listar falhou") }).await;

    assert!(
        clientes_b.is_empty(),
        "tenant B não deve ver clientes do tenant A"
    );
    Ok(())
}

/// Saldo de estoque do tenant A é zero para o tenant B (mesmo produto_id).
#[tokio::test]
async fn estoque_de_a_invisivel_para_b() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    let repo = Arc::new(PostgresEstoqueRepository::new(pool.clone()));
    let estoque = EstoqueHandlers::new(repo.clone(), bus);

    let id_a = new_tenant_id();
    let id_b = new_tenant_id();
    let produto_id = Uuid::new_v4();

    in_tenant(id_a, async move {
        estoque
            .handle(RegistrarEntradaEstoque {
                produto_id,
                quantidade: 100,
                custo_unitario_centavos: 500,
                motivo: "entrada A".into(),
                nota_fiscal: None,
            })
            .await
            .expect("entrada estoque A falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // tenant B busca pelo mesmo produto_id — deve retornar None
    let saldo_b = in_tenant(id_b, async move {
        repo.buscar(produto_id).await.expect("buscar falhou")
    })
    .await;

    assert!(
        saldo_b.is_none(),
        "tenant B não deve ver saldo de estoque do tenant A"
    );
    Ok(())
}

// ── Isolamento em fluxos cross-BC ─────────────────────────────────────────────

/// Venda confirmada no tenant A gera conta a receber e NF apenas para A,
/// não vazando nenhuma linha para o tenant B.
#[tokio::test]
async fn venda_de_a_nao_gera_dados_em_b() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();

    let estoque_a = Arc::new(EstoqueHandlers::new(
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

    bus.register(FinanceiroVendaEventHandler {
        financeiro: Arc::clone(&financeiro),
    });
    bus.register(FiscalVendaEventHandler {
        fiscal: Arc::clone(&fiscal),
    });
    bus.register(EstoqueVendaEventHandler {
        estoque: Arc::clone(&estoque_a),
    });
    bus.register(FinanceiroProjection::new(pool.clone()));
    bus.register(FiscalProjection::new(pool.clone()));
    // Necessário para a checagem de disponibilidade de estoque em
    // AdicionarItemVenda ler proj_saldo_estoque atualizado.
    bus.register(EstoqueProjection::new(pool.clone()));

    let id_a = new_tenant_id();
    let id_b = new_tenant_id();
    let produto_id = Uuid::new_v4();
    // AdicionarItemVenda usa o preço de tabela do catálogo (5000).
    seed_produto(&pool, id_a, produto_id, "X", 5000).await?;

    in_tenant(id_a, async move {
        estoque_a
            .handle(RegistrarEntradaEstoque {
                produto_id,
                quantidade: 5,
                custo_unitario_centavos: 1000,
                motivo: "entrada A".into(),
                nota_fiscal: None,
            })
            .await
            .expect("entrada estoque falhou");
        aguardar_projecoes().await;

        let venda_id = vendas
            .handle(IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            })
            .await
            .expect("iniciar venda falhou");

        vendas
            .handle(AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "X".into(),
                descricao: "item".into(),
                quantidade: 1,
                preco_unitario_centavos: 5000,
                vender_sem_estoque: false,
                preservar_preco_informado: false,
            })
            .await
            .expect("adicionar item falhou");

        vendas
            .handle(DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
            })
            .await
            .expect("forma pagamento falhou");

        vendas
            .handle(ConfirmarVenda {
                venda_id: venda_id.as_uuid(),
            })
            .await
            .expect("confirmar venda falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // tenant B não deve ter nenhuma conta a receber nem NF
    let cr_b: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM proj_contas_receber WHERE tenant_id = $1")
            .bind(id_b)
            .fetch_one(&pool)
            .await?;

    let nf_b: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM proj_notas_fiscais WHERE tenant_id = $1")
            .bind(id_b)
            .fetch_one(&pool)
            .await?;

    assert_eq!(
        cr_b, 0,
        "tenant B não deve ter contas a receber do tenant A"
    );
    assert_eq!(nf_b, 0, "tenant B não deve ter NFs do tenant A");
    Ok(())
}

// ── Control plane ─────────────────────────────────────────────────────────────

/// Tenant suspenso tem status correto no banco — o handler de login checará isso.
#[tokio::test]
async fn tenant_suspenso_tem_status_suspenso() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(TenantRepository::new(pool.clone()));

    let tenant_id = create_tenant(&pool, "acme").await?;
    suspend_tenant(&pool, tenant_id).await?;

    let row = repo
        .buscar_por_slug("acme")
        .await
        .map_err(|e| format!("{e}"))?
        .expect("tenant não encontrado");

    assert_eq!(row.status, "suspenso");
    Ok(())
}

/// Tenant reativado após suspensão volta ao status ativo.
#[tokio::test]
async fn tenant_reativado_volta_a_ativo() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(TenantRepository::new(pool.clone()));

    let tenant_id = create_tenant(&pool, "beta").await?;
    suspend_tenant(&pool, tenant_id).await?;
    repo.reativar(tenant_id).await.map_err(|e| format!("{e}"))?;

    let row = repo
        .buscar_por_slug("beta")
        .await
        .map_err(|e| format!("{e}"))?
        .expect("tenant não encontrado");

    assert_eq!(row.status, "ativo");
    Ok(())
}

/// Dois tenants com o mesmo slug não podem coexistir (UNIQUE constraint).
#[tokio::test]
async fn slug_duplicado_retorna_tenant_existente() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(TenantRepository::new(pool.clone()));

    let id1 = repo
        .criar("mesmo-slug", "Empresa 1")
        .await
        .map_err(|e| format!("{e}"))?;
    let id2 = repo
        .criar("mesmo-slug", "Empresa 2")
        .await
        .map_err(|e| format!("{e}"))?;

    // ON CONFLICT DO UPDATE retorna o mesmo tenant_id
    assert_eq!(id1, id2, "ON CONFLICT deve retornar o tenant_id existente");
    Ok(())
}

// ── Login de tenant suspenso ──────────────────────────────────────────────────

/// Login com credenciais válidas mas tenant suspenso deve retornar Unauthorized.
#[tokio::test]
async fn login_negado_para_tenant_suspenso() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(IdentityProjection::new(pool.clone()));

    let identity = Arc::new(IdentityHandlers::new(
        Arc::new(PostgresIdentityRepository::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
        Arc::new(AuthConfig::new("segredo-teste".into())),
    ));

    let tenant_id = create_tenant(&pool, "empresa-suspensa").await?;

    in_tenant(tenant_id, async {
        identity
            .handle(RegistrarUsuario {
                username: "operador".into(),
                senha: "senha123".into(),
                roles: vec!["operador".into()],
            })
            .await
            .expect("registrar usuário falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    suspend_tenant(&pool, tenant_id).await?;

    let result = identity
        .handle(Login {
            slug: "empresa-suspensa".into(),
            username: "operador".into(),
            senha: "senha123".into(),
        })
        .await;

    assert!(
        matches!(result, Err(AppError::Unauthorized)),
        "login de tenant suspenso deve retornar Unauthorized, obteve: {result:?}"
    );
    Ok(())
}

// ── RLS ──────────────────────────────────────────────────────────────────────

/// Conexão sem BYPASSRLS e sem `app.tenant_id` não enxerga nenhuma linha.
/// Com `SET LOCAL app.tenant_id = '<uuid>'` dentro de uma transação, a política
/// permite leitura apenas das linhas do tenant correto.
#[tokio::test]
async fn rls_sem_tenant_id_retorna_zero_linhas() -> TestResult {
    let (_c, pool, url) = start_postgres_with_url().await?;
    setup_db(&pool).await?;

    // Cria um role sem BYPASSRLS para exercitar as políticas RLS.
    sqlx::query("CREATE ROLE rls_tester LOGIN PASSWORD 'testpass'")
        .execute(&pool)
        .await?;
    sqlx::query("GRANT SELECT, INSERT, UPDATE ON proj_clientes TO rls_tester")
        .execute(&pool)
        .await?;

    // Insere uma linha via superuser (que bypassa RLS).
    let tenant_id = Uuid::new_v4();
    let cliente_id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO proj_clientes
             (cliente_id, tenant_id, nome, cpf_cnpj, bloqueado, criado_em, atualizado_em)
         VALUES ($1, $2, 'Empresa RLS', '00000000000', false, $3, $3)",
    )
    .bind(cliente_id)
    .bind(tenant_id)
    .bind(now)
    .execute(&pool)
    .await?;

    // Conecta como role restrito.
    let restricted_url = url.replacen("postgres:postgres@", "rls_tester:testpass@", 1);
    let restricted_pool = connect_pool(&restricted_url, 2)?;

    // Sem app.tenant_id → zero linhas.
    let count_sem: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proj_clientes")
        .fetch_one(&restricted_pool)
        .await?;
    assert_eq!(
        count_sem, 0,
        "RLS deve bloquear leitura sem app.tenant_id definido"
    );

    // Com app.tenant_id correto (dentro de uma transação) → a linha aparece.
    let mut tx = restricted_pool.begin().await?;
    // Box::leak é aceitável em testes — o processo termina logo e o valor é pequeno.
    let set_local: &'static str =
        Box::leak(format!("SET LOCAL app.tenant_id = '{tenant_id}'").into_boxed_str());
    sqlx::raw_sql(set_local).execute(&mut *tx).await?;
    let count_com: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proj_clientes")
        .fetch_one(&mut *tx)
        .await?;
    tx.rollback().await?;

    assert_eq!(
        count_com, 1,
        "com app.tenant_id correto, a linha do tenant deve aparecer"
    );
    Ok(())
}