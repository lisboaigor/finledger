#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Lixeira: a rotina `executar_arquivamento()` (migração 002) esconde das
/// listagens vendas/orçamentos não concretizados após o prazo do tenant, sem
/// excluir nada; o gestor restaura e o restaurado não volta a ser arquivado.
mod helpers;
use helpers::{TestResult, create_tenant, in_tenant, montar_app, setup_db, start_postgres};

use pharos_app::{dispatch, query_dispatch};
use finledger::orcamentos::application::commands::CriarOrcamento;
use finledger::orcamentos::application::queries::{ListarOrcamentos, ListarOrcamentosArquivados};
use finledger::vendas::application::commands::IniciarVenda;
use finledger::vendas::application::queries::{ListarVendas, ListarVendasArquivadas};
use uuid::Uuid;

#[tokio::test]
async fn arquiva_lista_na_lixeira_e_restaura() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let tenant_id = create_tenant(&pool, "lixeira").await?;
    // Prazo de 30 dias configurado pelo gestor.
    sqlx::query("UPDATE tenants SET arquivamento_dias = 30 WHERE tenant_id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await?;

    let app = montar_app(&pool);
    let pool2 = pool.clone();

    let (venda_id, orcamento_id) = in_tenant(tenant_id, async move {
        // Venda iniciada e abandonada + orçamento em rascunho nunca emitido.
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar venda");

        let orcamento_id = dispatch(
            &*app.orcamentos,
            CriarOrcamento {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
                cliente_avulso: Some("Balcão".into()),
                validade_dias: 7,
            },
        )
        .await
        .expect("criar orçamento");

        helpers::aguardar_projecoes().await;

        // Antes do prazo: nada arquivado, tudo listado normalmente.
        let vendas = query_dispatch(&*app.vendas, ListarVendas { produto_busca: None, apenas_abertas: false, limite: None, offset: None })
            .await
            .expect("listar vendas");
        assert_eq!(vendas.len(), 1);

        // Envelhece os registros além do prazo (simula 40 dias parados).
        sqlx::query(
            "UPDATE proj_vendas SET atualizado_em = NOW() - INTERVAL '40 days' WHERE venda_id = $1",
        )
        .bind(venda_id.as_uuid())
        .execute(&pool2)
        .await
        .expect("envelhecer venda");
        sqlx::query(
            "UPDATE proj_orcamentos SET atualizado_em = NOW() - INTERVAL '40 days' WHERE orcamento_id = $1",
        )
        .bind(orcamento_id.as_uuid())
        .execute(&pool2)
        .await
        .expect("envelhecer orçamento");

        // Roda a varredura (o job chama exatamente isto).
        let resultado: serde_json::Value =
            sqlx::query_scalar("SELECT executar_arquivamento()")
                .fetch_one(&pool2)
                .await
                .expect("executar arquivamento");
        assert_eq!(resultado["vendas"], 1, "venda abandonada deve ser arquivada");
        assert_eq!(resultado["orcamentos"], 1, "rascunho velho deve ser arquivado");

        // Listagens padrão escondem; a lixeira mostra — nada foi excluído.
        let vendas = query_dispatch(&*app.vendas, ListarVendas { produto_busca: None, apenas_abertas: false, limite: None, offset: None })
            .await
            .expect("listar vendas");
        assert!(vendas.is_empty());
        let lixeira_v = query_dispatch(&*app.vendas, ListarVendasArquivadas::default())
            .await
            .expect("lixeira vendas");
        assert_eq!(lixeira_v.len(), 1);

        let orcs = query_dispatch(&*app.orcamentos, ListarOrcamentos { apenas_abertos: false, limite: None, offset: None })
            .await
            .expect("listar orçamentos");
        assert!(orcs.is_empty());
        let lixeira_o = query_dispatch(&*app.orcamentos, ListarOrcamentosArquivados::default())
            .await
            .expect("lixeira orçamentos");
        assert_eq!(lixeira_o.len(), 1);

        // Restauração devolve à listagem e sai da lixeira.
        app.vendas
            .restaurar_arquivada(venda_id.as_uuid())
            .await
            .expect("restaurar venda");
        app.orcamentos
            .restaurar_arquivado(orcamento_id.as_uuid())
            .await
            .expect("restaurar orçamento");

        let vendas = query_dispatch(&*app.vendas, ListarVendas { produto_busca: None, apenas_abertas: false, limite: None, offset: None })
            .await
            .expect("listar vendas");
        assert_eq!(vendas.len(), 1);
        let lixeira_v = query_dispatch(&*app.vendas, ListarVendasArquivadas::default())
            .await
            .expect("lixeira vendas");
        assert!(lixeira_v.is_empty());

        (venda_id, orcamento_id)
    })
    .await;

    // Restaurado não volta a ser arquivado, mesmo continuando velho.
    let resultado: serde_json::Value = sqlx::query_scalar("SELECT executar_arquivamento()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(resultado["vendas"], 0, "restaurada não deve re-arquivar");
    assert_eq!(resultado["orcamentos"], 0, "restaurado não deve re-arquivar");

    let _ = (venda_id, orcamento_id);
    Ok(())
}

#[tokio::test]
async fn sem_prazo_configurado_nada_e_arquivado() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let tenant_id = create_tenant(&pool, "sem-prazo").await?;
    let app = montar_app(&pool);
    let pool2 = pool.clone();

    in_tenant(tenant_id, async move {
        let venda_id = dispatch(
            &*app.vendas,
            IniciarVenda {
                vendedor_id: Uuid::new_v4(),
                cliente_id: None,
            },
        )
        .await
        .expect("iniciar venda");
        helpers::aguardar_projecoes().await;

        sqlx::query(
            "UPDATE proj_vendas SET atualizado_em = NOW() - INTERVAL '400 days' WHERE venda_id = $1",
        )
        .bind(venda_id.as_uuid())
        .execute(&pool2)
        .await
        .expect("envelhecer");

        let resultado: serde_json::Value =
            sqlx::query_scalar("SELECT executar_arquivamento()")
                .fetch_one(&pool2)
                .await
                .expect("executar");
        assert_eq!(resultado["vendas"], 0, "lixeira desligada (dias NULL) não arquiva");
    })
    .await;
    Ok(())
}
