#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Vendas: fluxo completo de comandos, queries e repositório.
mod helpers;
use helpers::{
    TestResult, drenar_outbox, in_tenant, montar_app, new_tenant_id, seed_produto, setup_db,
    start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::catalogo::application::commands::CadastrarProduto;
use finledger::error::AppError;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::vendas::application::commands::{
    AdicionarItemVenda, CancelarVenda, ConfirmarVenda, DefinirFormaPagamento, IniciarVenda,
    RemoverItemVenda,
};
use finledger::vendas::application::queries::{BuscarVenda, ListarVendas};
use finledger::vendas::domain::value_objects::FormaPagamento;
use uuid::Uuid;

#[tokio::test]
async fn ciclo_completo_da_venda_com_queries() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    // Preço de tabela = 8000 — AdicionarItemVenda usa SEMPRE o catálogo.
    seed_produto(&pool, tenant_id, produto_id, "SKU-1", 8000).await?;

    in_tenant(tenant_id, async move {
        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id,
                quantidade: 10,
                custo_unitario_centavos: 4000,
                motivo: "estoque inicial".into(),
                nota_fiscal: None,
            },
        )
        .await
        .expect("entrada estoque");

        // Iniciar + itens
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        let item1 = dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-1".into(),
                descricao: "Pastilha".into(),
                quantidade: 2,
                preco_unitario_centavos: 8000,
                vender_sem_estoque: false,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("item 1");
        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-1".into(),
                descricao: "Pastilha".into(),
                quantidade: 1,
                preco_unitario_centavos: 8000,
                vender_sem_estoque: false,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("item 2");

        // Remover o primeiro item
        dispatch(
            &*app.vendas,
            RemoverItemVenda {
                venda_id: venda_id.as_uuid(),
                item_id: item1,
            },
        )
        .await
        .expect("remover item");

        dispatch(
            &*app.vendas,
            DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Pix,
            },
        )
        .await
        .expect("forma pagamento");

        dispatch(
            &*app.vendas,
            ConfirmarVenda {
                venda_id: venda_id.as_uuid(),
            },
        )
        .await
        .expect("confirmar");
        drenar_outbox(&pool).await.expect("drenar outbox");

        // Queries
        let lista = query_dispatch(
            &*app.vendas,
            ListarVendas {
                produto_busca: None,
                apenas_abertas: false,
                limite: None,
                offset: None,
            },
        )
        .await
        .expect("listar");
        assert_eq!(lista.len(), 1);
        let detalhes = query_dispatch(
            &*app.vendas,
            BuscarVenda {
                venda_id: venda_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("venda deve existir");
        assert_eq!(detalhes.venda.status, "Confirmada");
        assert_eq!(detalhes.venda.total_centavos, 8000);
        assert_eq!(detalhes.itens.len(), 1);

        // Estoque baixado na confirmação
        let saldo: i32 =
            sqlx::query_scalar("SELECT quantidade FROM proj_saldo_estoque WHERE produto_id = $1")
                .bind(produto_id)
                .fetch_one(&pool)
                .await
                .expect("saldo");
        assert_eq!(saldo, 9);
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn cancelar_venda_em_andamento() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");
        dispatch(
            &*app.vendas,
            CancelarVenda {
                venda_id: venda_id.as_uuid(),
                motivo: "cliente desistiu".into(),
            },
        )
        .await
        .expect("cancelar");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let detalhes = query_dispatch(
            &*app.vendas,
            BuscarVenda {
                venda_id: venda_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("venda deve existir");
        assert_eq!(detalhes.venda.status, "Cancelada");

        // Cancelar de novo → regra de negócio
        let r = dispatch(
            &*app.vendas,
            CancelarVenda {
                venda_id: venda_id.as_uuid(),
                motivo: "de novo".into(),
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

/// Sem saldo em estoque e sem `vender_sem_estoque`, adicionar o item já
/// bloqueia — o vendedor fica ciente do problema antes de prosseguir com a
/// venda, em vez de descobrir depois que a baixa assíncrona falhou.
#[tokio::test]
async fn adicionar_item_sem_estoque_e_bloqueado() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let produto_id = Uuid::new_v4();
    let tenant_id = new_tenant_id();
    // Produto existe no catálogo (senão o erro seria "fora do catálogo", não
    // falta de estoque).
    seed_produto(&pool, tenant_id, produto_id, "SKU-X", 1000).await?;

    in_tenant(tenant_id, async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        let r = dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-X".into(),
                descricao: "Sem estoque".into(),
                quantidade: 5,
                preco_unitario_centavos: 1000,
                vender_sem_estoque: false,
                preservar_preco_informado: false,
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

/// `vender_sem_estoque: true` é a confirmação explícita do vendedor para uma
/// venda sob encomenda — ignora o saldo e permite confirmar a venda normalmente.
#[tokio::test]
async fn adicionar_item_sob_encomenda_ignora_falta_de_estoque() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let produto_id = Uuid::new_v4();
    let tenant_id = new_tenant_id();
    seed_produto(&pool, tenant_id, produto_id, "SKU-X", 1000).await?;

    in_tenant(tenant_id, async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-X".into(),
                descricao: "Sob encomenda".into(),
                quantidade: 5,
                preco_unitario_centavos: 1000,
                vender_sem_estoque: true,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("item sob encomenda deve ser aceito");

        dispatch(
            &*app.vendas,
            DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
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
        .expect("confirmar venda sob encomenda");
    })
    .await;
    Ok(())
}

/// Produtos com `controla_estoque = false` (serviços, mão de obra) nunca são
/// bloqueados, mesmo sem nenhuma entrada de estoque registrada.
#[tokio::test]
async fn adicionar_item_de_servico_ignora_saldo_de_estoque() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let produto_id = dispatch(
            &*app.catalogo,
            CadastrarProduto {
                sku: "SERVICO-001".into(),
                descricao: "Mão de obra".into(),
                ncm: "00000000".into(),
                unidade: "SV".into(),
                preco_custo_centavos: 0,
                preco_venda_centavos: 15_000,
                categoria: "Serviços".into(),
                marca: None,
                controla_estoque: false,
                classe_trib: None,
            },
        )
        .await
        .expect("cadastrar serviço")
        .as_uuid();

        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SERVICO-001".into(),
                descricao: "Mão de obra".into(),
                quantidade: 1,
                preco_unitario_centavos: 15_000,
                vender_sem_estoque: false,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("serviço não deve ser bloqueado por falta de estoque");
    })
    .await;
    Ok(())
}
/// Issue #2: o preço unitário do payload é ignorado — o item entra SEMPRE com
/// o preço de tabela do catálogo (proj_produtos.preco_venda). Um PDV adulterado
/// não consegue vender abaixo do preço.
#[tokio::test]
async fn preco_adulterado_no_payload_e_substituido_pelo_preco_de_catalogo() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    seed_produto(&pool, tenant_id, produto_id, "SKU-CAT", 8000).await?;

    in_tenant(tenant_id, async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-CAT".into(),
                descricao: "Pastilha".into(),
                quantidade: 2,
                preco_unitario_centavos: 1, // adulterado — deve ser ignorado
                vender_sem_estoque: true,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("adicionar item");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let detalhes = query_dispatch(
            &*app.vendas,
            BuscarVenda {
                venda_id: venda_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("venda existe");
        assert_eq!(
            detalhes.itens[0].preco_unitario_centavos, 8000,
            "preço gravado é o de catálogo, não o do payload"
        );
        assert_eq!(detalhes.venda.total_centavos, 16_000);
    })
    .await;
    Ok(())
}

/// Produto fora do catálogo → erro de negócio claro (não INSERT às cegas).
#[tokio::test]
async fn adicionar_item_de_produto_fora_do_catalogo_retorna_erro() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");

        let r = dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id: Uuid::new_v4(), // nunca cadastrado
                sku: "SKU-FANTASMA".into(),
                descricao: "Não existe".into(),
                quantidade: 1,
                preco_unitario_centavos: 1000,
                vender_sem_estoque: true,
                preservar_preco_informado: false,
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

/// Desconto maior que o total bruto dos itens → erro de validação do domínio.
#[tokio::test]
async fn desconto_maior_que_o_total_da_venda_retorna_erro() -> TestResult {
    use finledger::vendas::application::commands::AplicarDescontoVenda;

    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    seed_produto(&pool, tenant_id, produto_id, "SKU-D", 8000).await?;

    in_tenant(tenant_id, async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar");
        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id,
                sku: "SKU-D".into(),
                descricao: "Item".into(),
                quantidade: 1,
                preco_unitario_centavos: 8000,
                vender_sem_estoque: true,
                preservar_preco_informado: false,
            },
        )
        .await
        .expect("item");

        let r = dispatch(
            &*app.vendas,
            AplicarDescontoVenda {
                venda_id: venda_id.as_uuid(),
                desconto_centavos: 8001,
            },
        )
        .await;
        assert!(matches!(
            r,
            Err(DispatchError::Handler(AppError::Domain(_)))
        ));

        // Desconto válido funciona e reduz o total projetado.
        dispatch(
            &*app.vendas,
            AplicarDescontoVenda {
                venda_id: venda_id.as_uuid(),
                desconto_centavos: 500,
            },
        )
        .await
        .expect("desconto válido");
        drenar_outbox(&pool).await.expect("drenar outbox");

        let detalhes = query_dispatch(
            &*app.vendas,
            BuscarVenda {
                venda_id: venda_id.as_uuid(),
            },
        )
        .await
        .expect("buscar")
        .expect("venda existe");
        assert_eq!(detalhes.venda.desconto_centavos, 500);
        assert_eq!(detalhes.venda.total_centavos, 7500);
    })
    .await;
    Ok(())
}
