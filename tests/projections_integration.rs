#![allow(clippy::unwrap_used, clippy::expect_used)]

mod helpers;
use helpers::{TestResult, in_tenant, new_tenant_id, setup_db, start_postgres};

use std::sync::Arc;

use pharos_app::{CommandHandler, EventBus};
use finledger::catalogo::application::commands::CadastrarProduto;
use finledger::catalogo::infrastructure::precificacao_repository::PostgresPrecificacaoRepository;
use finledger::catalogo::infrastructure::repository::PostgresProdutoRepository;
use finledger::crm::application::commands::CadastrarCliente;
use finledger::crm::infrastructure::repository::PostgresClienteRepository;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::estoque::infrastructure::repository::PostgresEstoqueRepository;
use finledger::{
    catalogo::application::handler::CatalogoHandlers,
    crm::application::handler::CrmHandlers,
    estoque::application::handler::EstoqueHandlers,
    projections::{catalogo::CatalogoProjection, crm::CrmProjection, estoque::EstoqueProjection},
};
use uuid::Uuid;

#[tokio::test]
async fn proj_produto_reflete_cadastro() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(CatalogoProjection::new(pool.clone()));
    let repo = Arc::new(PostgresProdutoRepository::new(pool.clone()));
    let precificacao = Arc::new(PostgresPrecificacaoRepository::new(pool.clone()));
    let catalogo = CatalogoHandlers::new(repo, precificacao, bus);

    let tenant_id = new_tenant_id();
    let produto_id = in_tenant(tenant_id, async {
        catalogo
            .handle(CadastrarProduto {
                sku: "TEST-001".into(),
                descricao: "Pastilha de freio traseira".into(),
                ncm: "87083000".into(),
                unidade: "PC".into(),
                preco_custo_centavos: 3500,
                preco_venda_centavos: 7000,
                categoria: "Freios".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            })
            .await
            .expect("cadastrar produto falhou")
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let sku: Option<String> =
        sqlx::query_scalar("SELECT sku FROM proj_produtos WHERE produto_id = $1")
            .bind(produto_id.as_uuid())
            .fetch_optional(&pool)
            .await?;

    assert_eq!(sku.as_deref(), Some("TEST-001"));
    Ok(())
}

#[tokio::test]
async fn proj_cliente_reflete_cadastro() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(CrmProjection::new(pool.clone()));
    let repo = Arc::new(PostgresClienteRepository::new(pool.clone()));
    let crm = CrmHandlers::new(repo, bus);

    let tenant_id = new_tenant_id();
    in_tenant(tenant_id, async {
        crm.handle(CadastrarCliente {
            nome: "João da Silva".into(),
            cpf_cnpj: "12345678909".into(),
            telefone: None,
            email: None,
        })
        .await
        .expect("cadastrar cliente falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let nome: Option<String> =
        sqlx::query_scalar("SELECT nome FROM proj_clientes WHERE cpf_cnpj = $1")
            .bind("12345678909")
            .fetch_optional(&pool)
            .await?;

    assert_eq!(nome.as_deref(), Some("João da Silva"));
    Ok(())
}

#[tokio::test]
async fn proj_saldo_estoque_atualiza_na_entrada() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    bus.register(EstoqueProjection::new(pool.clone()));
    let repo = Arc::new(PostgresEstoqueRepository::new(pool.clone()));
    let estoque = EstoqueHandlers::new(repo, bus);

    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    in_tenant(tenant_id, async move {
        estoque
            .handle(RegistrarEntradaEstoque {
                produto_id,
                quantidade: 50,
                custo_unitario_centavos: 2000,
                motivo: "compra inicial".into(),
                nota_fiscal: None,
            })
            .await
            .expect("registrar entrada falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let qty: Option<i32> =
        sqlx::query_scalar("SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1")
            .bind(produto_id)
            .fetch_optional(&pool)
            .await?;

    assert_eq!(qty, Some(50));
    Ok(())
}