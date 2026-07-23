#![allow(clippy::unwrap_used, clippy::expect_used)]

mod helpers;
use helpers::{TestResult, create_tenant, in_tenant, new_tenant_id, setup_db, start_postgres};

use std::sync::Arc;

use pharos_app::CommandHandler;
use pharos_app::EventBus;
use pharos_core::Entity;
use finledger::fiscal::{
    application::{commands::CancelarNotaFiscal, handler::FiscalHandlers},
    domain::value_objects::StatusNFe,
    infrastructure::{
        aliquotas::PostgresAliquotaProvider,
        repository::PostgresNotaFiscalRepository,
        sefaz::{SefazClient, SefazError, SefazResponse, StubSefazClient},
    },
};
use finledger::tenants::repository::TenantRepository;
use uuid::Uuid;

#[tokio::test]
async fn fiscal_gerar_e_transmitir_autoriza_nf() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(
        repo,
        sefaz,
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-001".into(),
        descricao: "Filtro".into(),
        quantidade: 1,
        preco_unitario_centavos: 5000,
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    Ok(())
}

#[tokio::test]
async fn fiscal_cancelar_nf_autorizada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(
        Arc::clone(&repo),
        sefaz,
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-002".into(),
        descricao: "Vela".into(),
        quantidade: 2,
        preco_unitario_centavos: 3000,
    };

    use pharos_core::Repository;
    use finledger::fiscal::domain::nota_fiscal::NotaFiscalId;

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("gerar e transmitir falhou");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let nf_id: Option<Uuid> =
            sqlx::query_scalar("SELECT nf_id FROM proj_notas_fiscais WHERE venda_id = $1")
                .bind(venda_id)
                .fetch_optional(&pool)
                .await
                .expect("query nf_id falhou");

        let id = nf_id.expect("deve existir exatamente uma NF para esta venda");
        let nf = repo
            .find_by_id(&NotaFiscalId::from(id))
            .await
            .expect("buscar NF falhou")
            .expect("NF não encontrada");

        assert_eq!(nf.status, StatusNFe::Autorizada);

        fiscal
            .handle(CancelarNotaFiscal {
                nf_id: nf.id().as_uuid(),
                motivo: "teste de cancelamento".into(),
            })
            .await
            .expect("cancelar NF falhou");
    })
    .await;

    Ok(())
}

#[tokio::test]
async fn fiscal_projecao_registra_nf_autorizada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let sefaz = Arc::new(StubSefazClient);
    let fiscal = FiscalHandlers::new(
        repo,
        sefaz,
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-003".into(),
        descricao: "Amortecedor".into(),
        quantidade: 1,
        preco_unitario_centavos: 12000,
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM proj_notas_fiscais WHERE venda_id = $1")
            .bind(venda_id)
            .fetch_optional(&pool)
            .await?;

    assert_eq!(
        status.as_deref(),
        Some("autorizada"),
        "projeção deve refletir status autorizada"
    );
    Ok(())
}

fn montar_fiscal(
    pool: &pharos_postgres::Pool,
    bus: EventBus,
) -> FiscalHandlers<StubSefazClient, PostgresAliquotaProvider> {
    FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(StubSefazClient),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    )
}

/// Oráculo independente dos valores esperados: espelha a LEI (fases da
/// transição + seed de alíquotas da migração 009), não o motor. O handler
/// emite com a data corrente (`Utc::now`), então os testes calculam o esperado
/// para o ano de hoje — nada quebra na virada de fase (2027, 2029, 2033...).
struct ImpostosEsperados {
    icms: i64,
    pis: i64,
    cofins: i64,
    cbs: i64,
    ibs_uf: i64,
    ibs_mun: i64,
}

fn impostos_esperados_hoje(base: i64, reducao_bps: i64) -> ImpostosEsperados {
    use chrono::Datelike;
    // Mesma referência de "hoje" do handler: dia fiscal em America/Sao_Paulo.
    let ano = finledger::fiscal::domain::tributacao::hoje_brasil().year();

    let aplicar = |bps: i64| (base * bps + 5_000) / 10_000;
    // Redução de classe (LC 214) só sobre CBS/IBS — arredondamento ÚNICO
    // half-up sobre o valor final (issue #17), não truncando a alíquota antes.
    let aplicar_reduzido = |bps: i64| (base * bps * (10_000 - reducao_bps) + 50_000_000) / 100_000_000;
    // Phase-down constitucional do ICMS: 100% até 2028, 90..60% em 2029-2032, 0 em 2033+.
    let fator_icms = match ano {
        ..=2028 => 10_000,
        2029 => 9_000,
        2030 => 8_000,
        2031 => 7_000,
        2032 => 6_000,
        _ => 0,
    };
    let (cbs_bps, ibs_uf_bps, ibs_mun_bps) = match ano {
        ..=2025 => (0, 0, 0),
        2026 => (90, 5, 5),
        2027..=2028 => (880, 5, 5),
        2029 => (880, 142, 35),
        2030 => (880, 283, 71),
        2031 => (880, 425, 106),
        2032 => (880, 566, 142),
        _ => (880, 1416, 354),
    };
    let pis_cofins_vigente = ano <= 2026;

    ImpostosEsperados {
        icms: aplicar(1800 * fator_icms / 10_000),
        pis: if pis_cofins_vigente { aplicar(65) } else { 0 },
        cofins: if pis_cofins_vigente { aplicar(300) } else { 0 },
        cbs: aplicar_reduzido(cbs_bps),
        ibs_uf: aplicar_reduzido(ibs_uf_bps),
        ibs_mun: aplicar_reduzido(ibs_mun_bps),
    }
}

/// SEFAZ que rejeita toda transmissão — exercita o desfecho infeliz que o
/// `StubSefazClient` (sempre autoriza) nunca alcança.
struct SefazQueRejeita;
impl SefazClient for SefazQueRejeita {
    async fn transmitir(&self, _xml: String) -> Result<SefazResponse, SefazError> {
        Err(SefazError::Rejeicao {
            codigo: "539".into(),
            motivo: "Duplicidade de NF-e".into(),
        })
    }
}

/// SEFAZ fora do ar — a NF deve ficar em `Transmitida` aguardando retransmissão.
struct SefazIndisponivel;
impl SefazClient for SefazIndisponivel {
    async fn transmitir(&self, _xml: String) -> Result<SefazResponse, SefazError> {
        Err(SefazError::Indisponivel("timeout".into()))
    }
}

fn item_teste(produto_id: Uuid, preco: i64) -> finledger::vendas::domain::events::ItemVendaSnapshot {
    finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-T".into(),
        descricao: "Produto teste".into(),
        quantidade: 1,
        preco_unitario_centavos: preco,
    }
}

#[tokio::test]
async fn rejeicao_da_sefaz_deixa_nf_rejeitada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(SefazQueRejeita),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let venda_id = Uuid::new_v4();
    let item = item_teste(Uuid::new_v4(), 5000);
    in_tenant(new_tenant_id(), async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("rejeição da SEFAZ não é erro do fluxo — vira status da NF");
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT status, rejeicao_codigo FROM proj_notas_fiscais WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_optional(&pool)
    .await?;
    let (status, codigo) = row.expect("NF projetada");
    assert_eq!(status, "rejeitada");
    assert_eq!(codigo.as_deref(), Some("539"));
    Ok(())
}

#[tokio::test]
async fn sefaz_indisponivel_deixa_nf_transmitida_para_retransmissao() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(SefazIndisponivel),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let venda_id = Uuid::new_v4();
    let item = item_teste(Uuid::new_v4(), 5000);
    in_tenant(new_tenant_id(), async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("indisponibilidade não derruba o fluxo");
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM proj_notas_fiscais WHERE venda_id = $1")
            .bind(venda_id)
            .fetch_optional(&pool)
            .await?;
    assert_eq!(
        status.as_deref(),
        Some("transmitida"),
        "NF fica transmitida aguardando retransmissão manual"
    );
    Ok(())
}

/// Classe tributária corrompida na projeção (só alcançável por SQL manual —
/// os comandos validam na entrada): a emissão falha com erro de domínio em vez
/// de calcular imposto com classificação inválida.
#[tokio::test]
async fn classe_tributaria_corrompida_na_projecao_falha_emissao() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let tenant_id = new_tenant_id();
    let produto_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO proj_produtos
            (produto_id, tenant_id, sku, descricao, ncm, unidade, preco_custo, preco_venda,
             categoria, ativo, c_class_trib, criado_em, atualizado_em)
         VALUES ($1, $2, 'SKU-BAD', 'Produto corrompido', '84716053', 'UN', 1000, 5000,
                 'Teste', TRUE, '12A', NOW(), NOW())",
    )
    .bind(produto_id)
    .bind(tenant_id)
    .execute(&pool)
    .await?;

    let fiscal = montar_fiscal(&pool, EventBus::new());
    let venda_id = Uuid::new_v4();
    let item = item_teste(produto_id, 5000);
    let resultado = in_tenant(tenant_id, async move {
        fiscal.gerar_e_transmitir(venda_id, None, &[item], 0).await
    })
    .await;
    assert!(resultado.is_err(), "classe malformada deve falhar a emissão");
    Ok(())
}

/// Trava de regressão: tenant SEM perfil fiscal configurado emite NF com os
/// mesmos valores legados de sempre (ICMS 18%, PIS 0,65%, COFINS 3%, aritmética
/// inteira). Estando em 2026, a fase de teste da reforma acrescenta CBS/IBS
/// informativos — obrigação legal do documento, sem alterar os legados.
#[tokio::test]
async fn nf_sem_perfil_fiscal_mantem_valores_legados() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = montar_fiscal(&pool, bus);

    let tenant_id = new_tenant_id(); // não existe em `tenants` → perfil ausente
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-REG".into(),
        descricao: "Produto regressão".into(),
        quantidade: 1,
        preco_unitario_centavos: 100_000, // R$ 1.000,00
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let row: Option<(i64, i64, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT icms_centavos, pis_centavos, cofins_centavos,
                cbs_centavos, ibs_uf_centavos, ibs_mun_centavos
         FROM proj_notas_fiscais WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_optional(&pool)
    .await?;
    let (icms, pis, cofins, cbs, ibs_uf, ibs_mun) = row.expect("NF projetada");

    // Compara com o oráculo da fase vigente hoje (tributação integral).
    let e = impostos_esperados_hoje(100_000, 0);
    assert_eq!(icms, e.icms, "ICMS da fase vigente");
    assert_eq!(pis, e.pis, "PIS da fase vigente");
    assert_eq!(cofins, e.cofins, "COFINS da fase vigente");
    assert_eq!(cbs, e.cbs, "CBS da fase vigente");
    assert_eq!(ibs_uf, e.ibs_uf);
    assert_eq!(ibs_mun, e.ibs_mun);
    // Sanidade: os montantes não podem ser todos zero — pegaria um motor que
    // ignora as alíquotas por completo.
    assert!(icms + pis + cofins + cbs > 0, "algum imposto deve incidir");
    Ok(())
}

/// A projeção por item (proj_nf_itens) materializa os 8 tributos por produto e
/// congela o flag `ibs_cbs_informativo` do perfil — insumo da margem líquida do
/// BI. Tenant sem perfil = Simples legado ⇒ informativo verdadeiro.
#[tokio::test]
async fn nf_projeta_impostos_por_item_com_flag_informativo() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = montar_fiscal(&pool, bus);

    let tenant_id = new_tenant_id(); // sem perfil → Simples legado (informativo)
    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item_teste(produto_id, 100_000)], 0)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let row: Option<(Uuid, i32, i64, i64, i64, i64, bool)> = sqlx::query_as(
        "SELECT produto_id, quantidade, total_centavos, icms_centavos,
                cbs_centavos, ibs_uf_centavos, ibs_cbs_informativo
         FROM proj_nf_itens WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_optional(&pool)
    .await?;
    let (proj_produto, qtd, total, icms, cbs, ibs_uf, informativo) =
        row.expect("item da NF projetado");

    let e = impostos_esperados_hoje(100_000, 0);
    assert_eq!(proj_produto, produto_id);
    assert_eq!(qtd, 1);
    assert_eq!(total, 100_000);
    assert_eq!(icms, e.icms, "ICMS por item da fase vigente");
    assert_eq!(cbs, e.cbs, "CBS por item da fase vigente");
    assert_eq!(ibs_uf, e.ibs_uf);
    assert!(informativo, "Simples sem regime regular → IBS/CBS informativo");
    Ok(())
}

/// Tenant COM perfil fiscal e produto classificado com redução de 60%:
/// CBS/IBS saem reduzidos; legados intactos (a classe da reforma não afeta ICMS).
#[tokio::test]
async fn nf_com_perfil_e_classe_de_reducao_aplica_reducao_no_ibs_cbs() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let tenant_id = create_tenant(&pool, "fiscal-perfil").await?;
    sqlx::query(
        "UPDATE tenants SET regime_tributario = 'lucro_presumido', uf = 'SP',
                codigo_municipio = '3550308', crt = 3 WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .execute(&pool)
    .await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = montar_fiscal(&pool, bus);

    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    // Produto classificado com a classe de redução 60% (seed da migração 009).
    sqlx::query(
        "INSERT INTO proj_produtos
            (produto_id, tenant_id, sku, descricao, ncm, unidade, preco_custo, preco_venda,
             categoria, ativo, c_class_trib, criado_em, atualizado_em)
         VALUES ($1, $2, 'SKU-RED', 'Produto com redução', '84716053', 'UN', 1000, 100000,
                 'Teste', TRUE, '200003', NOW(), NOW())",
    )
    .bind(produto_id)
    .bind(tenant_id)
    .execute(&pool)
    .await?;

    let item = finledger::vendas::domain::events::ItemVendaSnapshot {
        item_id: produto_id.to_string(),
        produto_id: produto_id.to_string(),
        sku: "SKU-RED".into(),
        descricao: "Produto com redução".into(),
        quantidade: 1,
        preco_unitario_centavos: 100_000,
    };

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let row: Option<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT icms_centavos, cbs_centavos, ibs_uf_centavos, ibs_mun_centavos
         FROM proj_notas_fiscais WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_optional(&pool)
    .await?;
    let (icms, cbs, ibs_uf, ibs_mun) = row.expect("NF projetada");

    let integral = impostos_esperados_hoje(100_000, 0);
    let reduzido = impostos_esperados_hoje(100_000, 6_000);
    assert_eq!(icms, integral.icms, "classe da reforma não afeta o ICMS");
    assert_eq!(cbs, reduzido.cbs, "CBS com redução de 60%");
    assert_eq!(ibs_uf, reduzido.ibs_uf, "IBS UF com redução de 60%");
    assert_eq!(ibs_mun, reduzido.ibs_mun, "IBS municipal com redução de 60%");
    // A redução precisa ter efeito real (reduzido < integral) — senão o teste
    // passaria mesmo com a classe ignorada.
    assert!(cbs < integral.cbs, "redução deve diminuir a CBS");
    Ok(())
}
/// NF rejeitada pela SEFAZ pode ser retransmitida (issue #9): após a correção
/// da causa, o comando Retransmitir leva a nota de `rejeitada` a `autorizada`.
#[tokio::test]
async fn nf_rejeitada_pode_ser_retransmitida_e_autorizada() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    use finledger::projections::fiscal::FiscalProjection;
    let bus = EventBus::new();
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal_rejeita = FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(SefazQueRejeita),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus.clone(),
    );
    // Mesma fiação, agora com a SEFAZ saudável (a "correção" aconteceu).
    let fiscal_ok = montar_fiscal(&pool, bus);

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let item = item_teste(Uuid::new_v4(), 5000);

    in_tenant(tenant_id, async move {
        fiscal_rejeita
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("emissão com rejeição não é erro do fluxo");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let nf_id: Uuid = sqlx::query_scalar(
            "SELECT nf_id FROM proj_notas_fiscais WHERE venda_id = $1 AND status = 'rejeitada'",
        )
        .bind(venda_id)
        .fetch_one(&pool)
        .await
        .expect("NF rejeitada projetada");

        fiscal_ok
            .handle(finledger::fiscal::application::commands::RetransmitirNotaFiscal { nf_id })
            .await
            .expect("retransmitir NF rejeitada");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let status: String =
            sqlx::query_scalar("SELECT status FROM proj_notas_fiscais WHERE nf_id = $1")
                .bind(nf_id)
                .fetch_one(&pool)
                .await
                .expect("status da NF");
        assert_eq!(status, "autorizada", "retransmissão deve autorizar a NF");
    })
    .await;
    Ok(())
}

/// Simples Nacional CONFIGURADO (issue #4): a NF não destaca ICMS/PIS/COFINS
/// (CSOSN 102), CBS/IBS informativos permanecem e o custo tributário do
/// vendedor passa a ser a alíquota efetiva do DAS.
#[tokio::test]
async fn simples_configurado_emite_sem_legados_com_csosn_e_das() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let tenant_id = create_tenant(&pool, "simples-conf").await?;
    sqlx::query(
        "UPDATE tenants SET regime_tributario = 'simples_nacional', uf = 'SP',
                codigo_municipio = '3550308', crt = 1, ibs_cbs_regime_regular = FALSE,
                aliquota_das_bps = 700 WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .execute(&pool)
    .await?;

    use finledger::projections::fiscal::FiscalProjection;
    let bus = EventBus::new();
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = montar_fiscal(&pool, bus);

    let venda_id = Uuid::new_v4();
    let produto_id = Uuid::new_v4();
    helpers::seed_produto(&pool, tenant_id, produto_id, "SKU-DAS", 100_000).await?;

    use finledger::fiscal::domain::nota_fiscal::NotaFiscalId;
    use pharos_core::Repository;
    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item_teste(produto_id, 100_000)], 0)
            .await
            .expect("gerar e transmitir falhou");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let (icms, pis, cofins, cbs): (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT icms_centavos, pis_centavos, cofins_centavos, cbs_centavos
             FROM proj_notas_fiscais WHERE venda_id = $1",
        )
        .bind(venda_id)
        .fetch_one(&pool)
        .await
        .expect("NF projetada");

        assert_eq!(icms, 0, "Simples configurado não destaca ICMS");
        assert_eq!(pis, 0, "Simples configurado não destaca PIS");
        assert_eq!(cofins, 0, "Simples configurado não destaca COFINS");
        let e = impostos_esperados_hoje(100_000, 0);
        assert_eq!(cbs, e.cbs, "CBS informativa permanece");

        // CSOSN e DAS congelados no agregado (itens da NF).
        let nf_id: Uuid =
            sqlx::query_scalar("SELECT nf_id FROM proj_notas_fiscais WHERE venda_id = $1")
                .bind(venda_id)
                .fetch_one(&pool)
                .await
                .expect("nf_id");
        let nf = repo
            .find_by_id(&NotaFiscalId::from(nf_id))
            .await
            .expect("buscar NF")
            .expect("NF existe");
        let imposto = &nf.itens[0].imposto;
        assert_eq!(imposto.csosn.as_deref(), Some("102"));
        assert_eq!(imposto.das_centavos, 7_000, "DAS 7% sobre R$ 1.000,00");

        // Precificação: alíquota efetiva do produto = alíquota do DAS.
        let efetivas = fiscal
            .listar_aliquota_efetiva_produtos()
            .await
            .expect("aliquotas efetivas");
        let efetiva = efetivas
            .iter()
            .find(|p| p.produto_id == produto_id)
            .expect("produto na lista");
        assert_eq!(efetiva.imposto_efetivo_bps, 700, "custo do vendedor = DAS");
    })
    .await;
    Ok(())
}

/// Numeração sequencial por tenant (issue #16): números 1, 2, 3 na ordem de
/// emissão, e cada tenant tem a própria sequência (isolamento).
#[tokio::test]
async fn numeracao_de_nf_e_sequencial_e_isolada_por_tenant() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    use finledger::projections::fiscal::FiscalProjection;
    let bus = EventBus::new();
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = Arc::new(montar_fiscal(&pool, bus));

    let tenant_a = new_tenant_id();
    let tenant_b = new_tenant_id();

    let mut vendas_a = Vec::new();
    for _ in 0..3 {
        let venda_id = Uuid::new_v4();
        vendas_a.push(venda_id);
        let f = Arc::clone(&fiscal);
        let item = item_teste(Uuid::new_v4(), 5000);
        in_tenant(tenant_a, async move {
            f.gerar_e_transmitir(venda_id, None, &[item], 0)
                .await
                .expect("emissão tenant A");
        })
        .await;
    }
    let venda_b = Uuid::new_v4();
    {
        let f = Arc::clone(&fiscal);
        let item = item_teste(Uuid::new_v4(), 5000);
        in_tenant(tenant_b, async move {
            f.gerar_e_transmitir(venda_b, None, &[item], 0)
                .await
                .expect("emissão tenant B");
        })
        .await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    for (idx, venda_id) in vendas_a.iter().enumerate() {
        let numero: i32 = sqlx::query_scalar(
            "SELECT numero FROM proj_notas_fiscais WHERE venda_id = $1 AND tenant_id = $2",
        )
        .bind(venda_id)
        .bind(tenant_a)
        .fetch_one(&pool)
        .await?;
        assert_eq!(numero as usize, idx + 1, "sequência 1,2,3 no tenant A");
    }
    let numero_b: i32 = sqlx::query_scalar(
        "SELECT numero FROM proj_notas_fiscais WHERE venda_id = $1 AND tenant_id = $2",
    )
    .bind(venda_b)
    .bind(tenant_b)
    .fetch_one(&pool)
    .await?;
    assert_eq!(numero_b, 1, "tenant B começa a própria sequência do 1");
    Ok(())
}

/// Devolução sobre NF presa em `transmitida` (issue #9): em vez de ficar
/// invisível (só 'autorizada' era buscada), a nota é marcada com cancelamento
/// pendente.
#[tokio::test]
async fn devolucao_sobre_nf_presa_em_transmitida_marca_cancelamento_pendente() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    use finledger::projections::fiscal::FiscalProjection;
    let bus = EventBus::new();
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = FiscalHandlers::new(
        Arc::new(PostgresNotaFiscalRepository::new(pool.clone())),
        Arc::new(SefazIndisponivel),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let tenant_id = new_tenant_id();
    let venda_id = Uuid::new_v4();
    let item = item_teste(Uuid::new_v4(), 5000);
    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &[item], 0)
            .await
            .expect("emissão com SEFAZ fora não derruba o fluxo");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        fiscal
            .processar_devolucao(venda_id, None, &[], true, "cliente desistiu")
            .await
            .expect("devolução sobre NF transmitida");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let (status, pendente): (String, bool) = sqlx::query_as(
            "SELECT status, cancelamento_pendente FROM proj_notas_fiscais WHERE venda_id = $1",
        )
        .bind(venda_id)
        .fetch_one(&pool)
        .await
        .expect("NF projetada");
        assert_eq!(status, "transmitida");
        assert!(pendente, "NF presa deve ficar com cancelamento pendente");
    })
    .await;
    Ok(())
}

/// Desconto global da venda destacado na NF: o total sai líquido (produtos −
/// desconto) e a base de cálculo de cada item é o subtotal menos a quota do
/// desconto rateada proporcionalmente (sobra de arredondamento no último item).
/// O desconto 999 sobre 60.000/40.000 não divide exato de propósito: quota do
/// 1º item = ⌊999×60000/100000⌋ = 599, o último absorve os 400 restantes —
/// Σ bases = produtos − desconto, sem centavo perdido.
#[tokio::test]
async fn nf_com_desconto_rateia_a_base_de_calculo_por_item() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = montar_fiscal(&pool, bus);

    let tenant_id = new_tenant_id(); // sem perfil → legado Simples
    let venda_id = Uuid::new_v4();
    let produto_a = Uuid::new_v4();
    let produto_b = Uuid::new_v4();
    let itens = vec![item_teste(produto_a, 60_000), item_teste(produto_b, 40_000)];
    let desconto = 999i64;

    in_tenant(tenant_id, async move {
        fiscal
            .gerar_e_transmitir(venda_id, None, &itens, desconto)
            .await
            .expect("gerar e transmitir falhou");
    })
    .await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Totais da NF: produtos brutos, desconto destacado, total líquido.
    let (total, desc, icms_total): (i64, i64, i64) = sqlx::query_as(
        "SELECT total_centavos, desconto_centavos, icms_centavos
         FROM proj_notas_fiscais WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(desc, 999, "desconto destacado na NF");
    assert_eq!(total, 100_000 - 999, "total da NF é o líquido");

    // Bases rateadas: 60.000 − 599 = 59.401 e 40.000 − 400 = 39.600.
    let esperado_a = impostos_esperados_hoje(59_401, 0);
    let esperado_b = impostos_esperados_hoje(39_600, 0);
    assert_eq!(
        icms_total,
        esperado_a.icms + esperado_b.icms,
        "ICMS total calculado sobre as bases líquidas rateadas"
    );

    let rows: Vec<(uuid::Uuid, i64, i64)> = sqlx::query_as(
        "SELECT produto_id, icms_centavos, cbs_centavos
         FROM proj_nf_itens WHERE venda_id = $1",
    )
    .bind(venda_id)
    .fetch_all(&pool)
    .await?;
    assert_eq!(rows.len(), 2);
    for (produto_id, icms, cbs) in rows {
        let esperado = if produto_id == produto_a {
            &esperado_a
        } else {
            &esperado_b
        };
        assert_eq!(icms, esperado.icms, "ICMS por item sobre a base líquida");
        assert_eq!(cbs, esperado.cbs, "CBS por item sobre a base líquida");
    }
    Ok(())
}

/// CFOP dinâmico por UF do destinatário: venda interestadual (destino ≠ SP do
/// emitente) usa 6102; intraestadual (destino SP) usa 5102. A UF vem de
/// proj_clientes; o emitente é o SP do perfil legado padrão.
#[tokio::test]
async fn cfop_dinamico_por_uf_do_destinatario() -> TestResult {
    let (_container, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let repo = Arc::new(PostgresNotaFiscalRepository::new(pool.clone()));
    let bus = EventBus::new();
    use finledger::projections::fiscal::FiscalProjection;
    bus.register(FiscalProjection::new(pool.clone()));
    let fiscal = FiscalHandlers::new(
        repo,
        Arc::new(StubSefazClient),
        Arc::new(PostgresAliquotaProvider::new(pool.clone())),
        Arc::new(TenantRepository::new(pool.clone())),
        bus,
    );

    let tenant_id = new_tenant_id();
    let pool2 = pool.clone();
    in_tenant(tenant_id, async move {
        let seed_cliente = |cid: Uuid, cpf: &'static str, uf: &'static str| {
            let p = pool2.clone();
            async move {
                sqlx::query(
                    "INSERT INTO proj_clientes
                        (cliente_id, nome, cpf_cnpj, uf, bloqueado, ativo, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, 'Cliente', $2, $3, false, true, now(), now(), $4)",
                )
                .bind(cid)
                .bind(cpf)
                .bind(uf)
                .bind(tenant_id)
                .execute(&p)
                .await
                .expect("seed cliente");
            }
        };
        let item = |prod: Uuid| finledger::vendas::domain::events::ItemVendaSnapshot {
            item_id: prod.to_string(),
            produto_id: prod.to_string(),
            sku: "SKU".into(),
            descricao: "Peça".into(),
            quantidade: 1,
            preco_unitario_centavos: 5000,
        };

        // Interestadual: destino RJ.
        let cli_rj = Uuid::new_v4();
        seed_cliente(cli_rj, "11111111111", "RJ").await;
        let venda_rj = Uuid::new_v4();
        fiscal
            .gerar_e_transmitir(venda_rj, Some(cli_rj), &[item(Uuid::new_v4())], 0)
            .await
            .expect("gerar RJ");

        // Intraestadual: destino SP (= UF do emitente legado).
        let cli_sp = Uuid::new_v4();
        seed_cliente(cli_sp, "22222222222", "SP").await;
        let venda_sp = Uuid::new_v4();
        fiscal
            .gerar_e_transmitir(venda_sp, Some(cli_sp), &[item(Uuid::new_v4())], 0)
            .await
            .expect("gerar SP");

        tokio::time::sleep(std::time::Duration::from_millis(80)).await;

        let cfop_rj: Option<String> =
            sqlx::query_scalar("SELECT cfop FROM proj_nf_itens WHERE venda_id = $1")
                .bind(venda_rj)
                .fetch_optional(&pool2)
                .await
                .expect("cfop rj");
        assert_eq!(cfop_rj.as_deref(), Some("6102"), "interestadual → 6102");

        let cfop_sp: Option<String> =
            sqlx::query_scalar("SELECT cfop FROM proj_nf_itens WHERE venda_id = $1")
                .bind(venda_sp)
                .fetch_optional(&pool2)
                .await
                .expect("cfop sp");
        assert_eq!(cfop_sp.as_deref(), Some("5102"), "intraestadual → 5102");
    })
    .await;
    Ok(())
}
