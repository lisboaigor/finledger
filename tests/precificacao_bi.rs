#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Precificação assistida + BI: custos fixos discriminados (soma sincronizada
/// com o total do tenant), giro por produto e score de saúde (bi.score_saude).
mod helpers;
use helpers::{
    TestResult, aguardar_projecoes, create_tenant, in_tenant, montar_app, setup_bi, setup_db,
    start_postgres,
};

use pharos_app::dispatch;
use finledger::catalogo::application::commands::CadastrarProduto;
use finledger::estoque::application::commands::RegistrarEntradaEstoque;
use finledger::tenants::repository::ConfigPrecificacao;
use finledger::vendas::application::commands::{
    AdicionarItemVenda, ConfirmarVenda, DefinirFormaPagamento, IniciarVenda,
};
use finledger::vendas::domain::value_objects::FormaPagamento;
use uuid::Uuid;

#[tokio::test]
async fn custos_fixos_discriminados_sincronizam_o_total() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let tenant_id = create_tenant(&pool, "custos").await?;
    let app = montar_app(&pool);

    in_tenant(tenant_id, async move {
        // Dois itens → total = soma.
        app.tenants
            .definir_custo_fixo("Aluguel", 150_000)
            .await
            .expect("aluguel");
        app.tenants
            .definir_custo_fixo("DAS-MEI", 8_205)
            .await
            .expect("das");
        let cfg = app.tenants.obter_config_precificacao().await.expect("obter");
        assert_eq!(cfg.custos_fixos_mensais_centavos, Some(158_205));

        // Guard: com itens cadastrados, o total enviado num PUT de config é
        // ignorado em favor da soma (regressão do bug NUMERIC→i64 inclusa).
        app.tenants
            .atualizar_config_precificacao(ConfigPrecificacao {
                custos_fixos_mensais_centavos: Some(1),
                vendas_mensais_estimadas: Some(70),
                // Denominador do rateio proporcional (custos ÷ faturamento).
                faturamento_mensal_estimado_centavos: Some(800_000),
                meta_faturamento_mensal_centavos: Some(1_000_000),
                ..Default::default()
            })
            .await
            .expect("atualizar config com itens presentes");
        let cfg = app.tenants.obter_config_precificacao().await.expect("obter");
        assert_eq!(cfg.custos_fixos_mensais_centavos, Some(158_205));
        assert_eq!(cfg.vendas_mensais_estimadas, Some(70));
        assert_eq!(cfg.faturamento_mensal_estimado_centavos, Some(800_000));
        assert_eq!(cfg.meta_faturamento_mensal_centavos, Some(1_000_000));

        // Remover o último item → total volta a NULL (modo manual).
        app.tenants.remover_custo_fixo("Aluguel").await.expect("remover");
        app.tenants.remover_custo_fixo("DAS-MEI").await.expect("remover");
        let cfg = app.tenants.obter_config_precificacao().await.expect("obter");
        assert_eq!(cfg.custos_fixos_mensais_centavos, None);

        let lista = app.tenants.listar_custos_fixos().await.expect("listar");
        assert!(lista.is_empty());
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn giro_reflete_vendas_e_saldo() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let tenant_id = create_tenant(&pool, "giro").await?;
    let app = montar_app(&pool);

    in_tenant(tenant_id, async move {
        let produto_id = dispatch(
            &*app.catalogo,
            CadastrarProduto {
                sku: "GIRO-1".into(),
                descricao: "Filtro de óleo".into(),
                ncm: "84212300".into(),
                unidade: "UN".into(),
                preco_custo_centavos: 1_000,
                preco_venda_centavos: 2_000,
                categoria: "Filtros".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            },
        )
        .await
        .expect("produto");

        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id: produto_id.as_uuid(),
                quantidade: 10,
                custo_unitario_centavos: 1_000,
                motivo: "estoque inicial".into(),
                nota_fiscal: None,
            },
        )
        .await
        .expect("entrada");

        // Uma venda confirmada de 3 unidades hoje.
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda { vendedor_id: Uuid::new_v4(), cliente_id: None },
        )
        .await
        .expect("iniciar");
        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id: produto_id.as_uuid(),
                sku: "GIRO-1".into(),
                descricao: "Filtro de óleo".into(),
                quantidade: 3,
                preco_unitario_centavos: 2_000,
                vender_sem_estoque: false,
            },
        )
        .await
        .expect("item");
        dispatch(
            &*app.vendas,
            DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
            },
        )
        .await
        .expect("pagamento");
        dispatch(&*app.vendas, ConfirmarVenda { venda_id: venda_id.as_uuid() })
            .await
            .expect("confirmar");
        aguardar_projecoes().await;

        let giro = app.precificacao.listar_giro_produtos().await.expect("giro");
        let linha = giro
            .iter()
            .find(|g| g.produto_id == produto_id.as_uuid())
            .expect("produto no giro");
        assert_eq!(linha.unidades_90d, 3);
        assert_eq!(linha.dias_sem_venda, Some(0), "vendeu hoje");
        assert_eq!(linha.saldo, 7, "10 entraram, 3 saíram");
    })
    .await;
    Ok(())
}

/// A venda confirmada gera a NF (fluxo de eventos), que projeta os impostos por
/// item; o ETL do BI desconta o imposto que é CUSTO do vendedor da margem bruta.
/// Tenant sem perfil = Simples legado: IBS/CBS informativos NÃO entram, mas os
/// legados (ICMS/PIS/COFINS da fase vigente) sim — então a líquida < bruta.
#[tokio::test]
async fn margem_liquida_desconta_impostos_da_nf() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    setup_bi(&pool).await?;
    let tenant_id = create_tenant(&pool, "margem-liq").await?;
    let app = montar_app(&pool);
    let pool2 = pool.clone();

    in_tenant(tenant_id, async move {
        let produto_id = dispatch(
            &*app.catalogo,
            CadastrarProduto {
                sku: "ML-1".into(),
                descricao: "Pastilha de freio".into(),
                ncm: "87083090".into(),
                unidade: "UN".into(),
                preco_custo_centavos: 5_000,
                preco_venda_centavos: 10_000,
                categoria: "Freios".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            },
        )
        .await
        .expect("produto");
        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id: produto_id.as_uuid(),
                quantidade: 5,
                custo_unitario_centavos: 5_000,
                motivo: "inicial".into(),
                nota_fiscal: None,
            },
        )
        .await
        .expect("entrada");
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda { vendedor_id: Uuid::new_v4(), cliente_id: None },
        )
        .await
        .expect("iniciar");
        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id: produto_id.as_uuid(),
                sku: "ML-1".into(),
                descricao: "Pastilha de freio".into(),
                quantidade: 2,
                preco_unitario_centavos: 10_000,
                vender_sem_estoque: false,
            },
        )
        .await
        .expect("item");
        dispatch(
            &*app.vendas,
            DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
            },
        )
        .await
        .expect("pagamento");
        dispatch(&*app.vendas, ConfirmarVenda { venda_id: venda_id.as_uuid() })
            .await
            .expect("confirmar");
        aguardar_projecoes().await;

        // ETL: carrega a venda (bruta) e reconcilia os impostos da NF (líquida).
        let _: serde_json::Value = sqlx::query_scalar("SELECT bi.executar_etl()")
            .fetch_one(&pool2)
            .await
            .expect("etl");

        let (bruta, impostos, liquida): (i64, i64, i64) = sqlx::query_as(
            "SELECT margem_centavos, impostos_centavos, margem_liquida_centavos
               FROM bi.fato_vendas_item
              WHERE tenant_id = $1 AND produto_id = $2",
        )
        .bind(tenant_id)
        .bind(produto_id.as_uuid())
        .fetch_one(&pool2)
        .await
        .expect("fato de venda");

        // Bruta = receita − custo = 20000 − 10000.
        assert_eq!(bruta, 10_000, "margem bruta");
        // Invariante central: líquida = bruta − impostos que são custo.
        assert_eq!(liquida, bruta - impostos, "líquida = bruta − impostos");
        // Na fase vigente há ao menos ICMS/PIS/COFINS legados incidindo.
        assert!(impostos > 0, "algum imposto legado deve incidir");
        assert!(liquida < bruta, "a líquida deve ser menor que a bruta");

        // O imposto do BI bate com o custo do vendedor da NF (legados + IS;
        // IBS/CBS ficam de fora por serem informativos no Simples).
        let esperado: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(icms_centavos + iss_centavos + pis_centavos
                        + cofins_centavos + is_centavos), 0)::bigint
               FROM proj_nf_itens WHERE tenant_id = $1 AND produto_id = $2",
        )
        .bind(tenant_id)
        .bind(produto_id.as_uuid())
        .fetch_one(&pool2)
        .await
        .expect("impostos da nf");
        assert_eq!(impostos, esperado, "BI usa o custo do vendedor da NF");
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn score_saude_compoe_metricas_do_tenant() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    setup_bi(&pool).await?;
    let tenant_id = create_tenant(&pool, "saude").await?;
    let app = montar_app(&pool);
    let pool2 = pool.clone();

    in_tenant(tenant_id, async move {
        // Sem nenhum dado: score nulo (nenhum componente inventado).
        let vazio: serde_json::Value = sqlx::query_scalar("SELECT bi.score_saude($1)")
            .bind(tenant_id)
            .fetch_one(&pool2)
            .await
            .expect("score vazio");
        assert!(vazio["score"].is_null());

        // Uma venda confirmada à vista alimenta receita (e o ETL, a margem).
        let produto_id = dispatch(
            &*app.catalogo,
            CadastrarProduto {
                sku: "SCORE-1".into(),
                descricao: "Correia".into(),
                ncm: "40103100".into(),
                unidade: "UN".into(),
                preco_custo_centavos: 5_000,
                preco_venda_centavos: 10_000,
                categoria: "Correias".into(),
                marca: None,
                controla_estoque: true,
                classe_trib: None,
            },
        )
        .await
        .expect("produto");
        dispatch(
            &*app.estoque,
            RegistrarEntradaEstoque {
                produto_id: produto_id.as_uuid(),
                quantidade: 5,
                custo_unitario_centavos: 5_000,
                motivo: "inicial".into(),
                nota_fiscal: None,
            },
        )
        .await
        .expect("entrada");
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda { vendedor_id: Uuid::new_v4(), cliente_id: None },
        )
        .await
        .expect("iniciar");
        dispatch(
            &*app.vendas,
            AdicionarItemVenda {
                venda_id: venda_id.as_uuid(),
                produto_id: produto_id.as_uuid(),
                sku: "SCORE-1".into(),
                descricao: "Correia".into(),
                quantidade: 2,
                preco_unitario_centavos: 10_000,
                vender_sem_estoque: false,
            },
        )
        .await
        .expect("item");
        dispatch(
            &*app.vendas,
            DefinirFormaPagamento {
                venda_id: venda_id.as_uuid(),
                forma: FormaPagamento::Dinheiro,
            },
        )
        .await
        .expect("pagamento");
        dispatch(&*app.vendas, ConfirmarVenda { venda_id: venda_id.as_uuid() })
            .await
            .expect("confirmar");
        aguardar_projecoes().await;

        // ETL popula fato_vendas_item/analise_produtos usados pelo score.
        let _: serde_json::Value = sqlx::query_scalar("SELECT bi.executar_etl()")
            .fetch_one(&pool2)
            .await
            .expect("etl");

        let saude: serde_json::Value = sqlx::query_scalar("SELECT bi.score_saude($1)")
            .bind(tenant_id)
            .fetch_one(&pool2)
            .await
            .expect("score");
        let score = saude["score"].as_i64().expect("score numérico");
        assert!((0..=100).contains(&score));

        let componentes = saude["componentes"].as_array().expect("componentes");
        let nomes: Vec<_> = componentes
            .iter()
            .map(|c| c["nome"].as_str().unwrap().to_string())
            .collect();
        // Com venda à vista e estoque: caixa, cobrança, margem e giro entram;
        // tendência fica de fora (sem período anterior) — pesos renormalizados.
        assert!(nomes.contains(&"Caixa (30 dias)".to_string()));
        assert!(nomes.contains(&"Cobrança".to_string()));
        assert!(nomes.contains(&"Margem de balcão".to_string()));
        assert!(nomes.contains(&"Giro de estoque".to_string()));
        assert!(!nomes.contains(&"Tendência de vendas".to_string()));
        assert!(!nomes.contains(&"Rumo à meta do mês".to_string()), "sem meta configurada");
        for c in componentes {
            let nota = c["nota"].as_f64().expect("nota");
            assert!((0.0..=100.0).contains(&nota));
            assert!(c["detalhe"].as_str().is_some_and(|d| !d.is_empty()));
        }

        // Com meta configurada, o componente "Rumo à meta" entra no score.
        sqlx::query("UPDATE tenants SET meta_faturamento_mensal_centavos = 100000 WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool2)
            .await
            .expect("definir meta");
        let saude: serde_json::Value = sqlx::query_scalar("SELECT bi.score_saude($1)")
            .bind(tenant_id)
            .fetch_one(&pool2)
            .await
            .expect("score com meta");
        let nomes: Vec<_> = saude["componentes"]
            .as_array()
            .expect("componentes")
            .iter()
            .map(|c| c["nome"].as_str().unwrap().to_string())
            .collect();
        assert!(nomes.contains(&"Rumo à meta do mês".to_string()));
    })
    .await;
    Ok(())
}
