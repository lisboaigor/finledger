#![allow(clippy::unwrap_used, clippy::expect_used)]

/// CRUD do módulo Orçamentos: fluxo completo de comandos, queries e repositório.
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, create_tenant, in_tenant, montar_app, new_tenant_id, setup_db,
    start_postgres,
};

use pharos_app::{DispatchError, dispatch, query_dispatch};
use finledger::error::AppError;
use finledger::orcamentos::application::commands::{
    AceitarOrcamento, AdicionarItemOrcamento, AplicarDescontoOrcamento, CriarOrcamento,
    EmitirOrcamento, RecusarOrcamento, RemoverItemOrcamento,
};
use finledger::orcamentos::application::queries::{BuscarOrcamento, ListarOrcamentos};
use finledger::vendas::application::queries::{BuscarVenda, ListarVendas};
use uuid::Uuid;

async fn criar_orcamento_com_item(
    app: &finledger::bootstrap::handlers::Handlers,
) -> (uuid::Uuid, uuid::Uuid) {
    let orcamento_id = dispatch(
        &*app.orcamentos,
        CriarOrcamento {
            vendedor_id: Uuid::new_v4(),
            cliente_id: None,
            cliente_avulso: None,
            validade_dias: 15,
        },
    )
    .await
    .expect("criar");

    let item_id = dispatch(
        &*app.orcamentos,
        AdicionarItemOrcamento {
            orcamento_id: orcamento_id.as_uuid(),
            produto_id: Uuid::new_v4(),
            sku: "SKU-1".into(),
            descricao: "Filtro de óleo".into(),
            quantidade: 4,
            preco_unitario_centavos: 2500,
        },
    )
    .await
    .expect("item");

    (orcamento_id.as_uuid(), item_id)
}

#[tokio::test]
async fn ciclo_completo_do_orcamento_ate_aceite() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let (orcamento_id, item1) = criar_orcamento_com_item(&app).await;

        // Item extra e remoção
        dispatch(
            &*app.orcamentos,
            AdicionarItemOrcamento {
                orcamento_id,
                produto_id: Uuid::new_v4(),
                sku: "SKU-2".into(),
                descricao: "Vela de ignição".into(),
                quantidade: 1,
                preco_unitario_centavos: 3000,
            },
        )
        .await
        .expect("item 2");
        dispatch(
            &*app.orcamentos,
            RemoverItemOrcamento {
                orcamento_id,
                item_id: item1,
            },
        )
        .await
        .expect("remover item");

        // Desconto + emissão + aceite
        dispatch(
            &*app.orcamentos,
            AplicarDescontoOrcamento {
                orcamento_id,
                desconto_centavos: 500,
            },
        )
        .await
        .expect("desconto");
        dispatch(&*app.orcamentos, EmitirOrcamento { orcamento_id })
            .await
            .expect("emitir");
        dispatch(&*app.orcamentos, AceitarOrcamento { orcamento_id })
            .await
            .expect("aceitar");
        aguardar_projecoes().await;

        // Queries
        let lista = query_dispatch(&*app.orcamentos, ListarOrcamentos { apenas_abertos: false })
            .await
            .expect("listar");
        assert_eq!(lista.len(), 1);
        let detalhes = query_dispatch(&*app.orcamentos, BuscarOrcamento { orcamento_id })
            .await
            .expect("buscar")
            .expect("orçamento deve existir");
        // Aceitar gera a venda e converte o orçamento (ver
        // VendaAPartirDeOrcamentoHandler + teste dedicado de conversão).
        assert_eq!(detalhes.orcamento.status, "ConvertidoEmVenda");
        assert_eq!(detalhes.orcamento.desconto_centavos, 500);
        assert_eq!(detalhes.itens.len(), 1);
        assert_eq!(detalhes.itens[0].sku, "SKU-2");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn recusar_orcamento_emitido() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let (orcamento_id, _) = criar_orcamento_com_item(&app).await;
        dispatch(&*app.orcamentos, EmitirOrcamento { orcamento_id })
            .await
            .expect("emitir");
        dispatch(
            &*app.orcamentos,
            RecusarOrcamento {
                orcamento_id,
                motivo: "preço alto".into(),
            },
        )
        .await
        .expect("recusar");
        aguardar_projecoes().await;

        let detalhes = query_dispatch(&*app.orcamentos, BuscarOrcamento { orcamento_id })
            .await
            .expect("buscar")
            .expect("orçamento deve existir");
        assert_eq!(detalhes.orcamento.status, "Recusado");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn aceitar_orcamento_nao_emitido_retorna_erro() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let (orcamento_id, _) = criar_orcamento_com_item(&app).await;
        let r = dispatch(&*app.orcamentos, AceitarOrcamento { orcamento_id }).await;
        assert!(matches!(
            r,
            Err(DispatchError::Handler(AppError::Domain(_)))
        ));

        let r = dispatch(
            &*app.orcamentos,
            EmitirOrcamento {
                orcamento_id: Uuid::new_v4(),
            },
        )
        .await;
        assert!(matches!(r, Err(DispatchError::Handler(AppError::NotFound))));
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn aceitar_orcamento_gera_venda_em_andamento_e_marca_convertido() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);

    in_tenant(new_tenant_id(), async move {
        let (orcamento_id, _) = criar_orcamento_com_item(&app).await;
        dispatch(&*app.orcamentos, EmitirOrcamento { orcamento_id })
            .await
            .expect("emitir");
        dispatch(&*app.orcamentos, AceitarOrcamento { orcamento_id })
            .await
            .expect("aceitar");
        aguardar_projecoes().await;

        // Orçamento marcado como convertido, ligado à venda gerada.
        let orc = query_dispatch(&*app.orcamentos, BuscarOrcamento { orcamento_id })
            .await
            .expect("buscar orçamento")
            .expect("orçamento deve existir");
        assert_eq!(orc.orcamento.status, "ConvertidoEmVenda");

        // Venda EmAndamento criada com o item do orçamento.
        let vendas = query_dispatch(
            &*app.vendas,
            ListarVendas {
                produto_busca: None,
                apenas_abertas: true,
            },
        )
        .await
        .expect("listar vendas abertas");
        assert_eq!(vendas.len(), 1, "deve existir uma venda em andamento");
        assert_eq!(vendas[0].status, "Em Andamento");

        let venda = query_dispatch(
            &*app.vendas,
            BuscarVenda {
                venda_id: vendas[0].venda_id,
            },
        )
        .await
        .expect("buscar venda")
        .expect("venda deve existir");
        assert_eq!(venda.itens.len(), 1);
        assert_eq!(venda.itens[0].sku, "SKU-1");
        assert_eq!(venda.itens[0].quantidade, 4);
        assert_eq!(venda.itens[0].preco_unitario_centavos, 2500);
    })
    .await;
    Ok(())
}

/// Por padrão (`permite_orcamento_sem_estoque = TRUE`, o valor de fábrica da
/// coluna) orçar acima do saldo é permitido — comportamento histórico
/// preservado para tenants existentes. Um tenant real é necessário aqui
/// porque `TenantRepository::permite_orcamento_sem_estoque` lê a tabela
/// `tenants` pelo tenant da requisição atual.
#[tokio::test]
async fn adicionar_item_sem_estoque_permitido_por_padrao() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let tenant_id = create_tenant(&pool, "orc-flag-default").await?;

    in_tenant(tenant_id, async move {
        let orcamento_id = dispatch(
            &*app.orcamentos,
            CriarOrcamento {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
                cliente_avulso: None,
                validade_dias: 15,
            },
        )
        .await
        .expect("criar");

        dispatch(
            &*app.orcamentos,
            AdicionarItemOrcamento {
                orcamento_id: orcamento_id.as_uuid(),
                produto_id: Uuid::new_v4(),
                sku: "SKU-1".into(),
                descricao: "Sem estoque".into(),
                quantidade: 10,
                preco_unitario_centavos: 2500,
            },
        )
        .await
        .expect("flag ligada por padrão deve permitir item sem estoque");
    })
    .await;
    Ok(())
}

/// Com a feature flag desligada pelo admin do tenant, orçar acima do saldo
/// passa a ser bloqueado — mesma regra de domínio usada em vendas.
#[tokio::test]
async fn adicionar_item_sem_estoque_bloqueado_quando_flag_desligada() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let app = montar_app(&pool);
    let tenant_id = create_tenant(&pool, "orc-flag-off").await?;

    in_tenant(tenant_id, async move {
        app.tenants
            .atualizar_configuracoes(false)
            .await
            .expect("desligar a flag");

        let orcamento_id = dispatch(
            &*app.orcamentos,
            CriarOrcamento {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
                cliente_avulso: None,
                validade_dias: 15,
            },
        )
        .await
        .expect("criar");

        let r = dispatch(
            &*app.orcamentos,
            AdicionarItemOrcamento {
                orcamento_id: orcamento_id.as_uuid(),
                produto_id: Uuid::new_v4(),
                sku: "SKU-1".into(),
                descricao: "Sem estoque".into(),
                quantidade: 10,
                preco_unitario_centavos: 2500,
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