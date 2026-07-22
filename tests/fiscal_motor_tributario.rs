#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Resolução de alíquotas do motor tributário: vigências, especificidade,
//! override por tenant e validação do perfil fiscal — incluindo os caminhos
//! infelizes (perfil incompleto, valores inválidos, classe desconhecida).

mod helpers;
use helpers::{TestResult, create_tenant, in_tenant, new_tenant_id, setup_db, start_postgres};

use chrono::NaiveDate;
use finledger::fiscal::domain::tributacao::{ClasseTributaria, PerfilFiscal};
use finledger::fiscal::infrastructure::aliquotas::{AliquotaProvider, PostgresAliquotaProvider};
use finledger::tenants::repository::{PerfilFiscalDto, TenantRepository};

fn dia(ano: i32, mes: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(ano, mes, d).expect("data válida")
}

// ── Resolução de alíquotas (seed da migração 009) ────────────────────────────

#[tokio::test]
async fn seed_resolve_fases_da_transicao() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let provider = PostgresAliquotaProvider::new(pool.clone());
    let perfil = PerfilFiscal::padrao_legado();
    let classe = ClasseTributaria::padrao();

    in_tenant(new_tenant_id(), async move {
        // 2026 (ano-teste): legados + CBS 0,9% + IBS 0,05+0,05.
        let em_2026 = provider
            .resolver(dia(2026, 7, 21), &perfil, &classe, "84716053")
            .await
            .expect("resolver 2026");
        assert_eq!(em_2026.icms.expect("icms").bps(), 1800);
        assert_eq!(em_2026.pis.expect("pis").bps(), 65);
        assert_eq!(em_2026.cofins.expect("cofins").bps(), 300);
        assert_eq!(em_2026.cbs.expect("cbs").bps(), 90);
        assert_eq!(em_2026.ibs_uf.expect("ibs_uf").bps(), 5);
        assert_eq!(em_2026.ibs_mun.expect("ibs_mun").bps(), 5);

        // Fronteira de vigência: último dia de 2026 ainda tem PIS/COFINS…
        let ultimo_2026 = provider
            .resolver(dia(2026, 12, 31), &perfil, &classe, "84716053")
            .await
            .expect("resolver 2026-12-31");
        assert!(ultimo_2026.pis.is_some());
        assert_eq!(ultimo_2026.cbs.expect("cbs").bps(), 90);

        // …e no primeiro dia de 2027 eles somem e a CBS vira plena.
        let primeiro_2027 = provider
            .resolver(dia(2027, 1, 1), &perfil, &classe, "84716053")
            .await
            .expect("resolver 2027-01-01");
        assert!(primeiro_2027.pis.is_none(), "PIS extinto em 2027");
        assert!(primeiro_2027.cofins.is_none(), "COFINS extinto em 2027");
        assert_eq!(primeiro_2027.cbs.expect("cbs").bps(), 880);

        // 2033: IBS pleno.
        let em_2033 = provider
            .resolver(dia(2033, 6, 1), &perfil, &classe, "84716053")
            .await
            .expect("resolver 2033");
        assert_eq!(em_2033.ibs_uf.expect("ibs_uf").bps(), 1416);
        assert_eq!(em_2033.ibs_mun.expect("ibs_mun").bps(), 354);
    })
    .await;
    Ok(())
}

/// Seed da migração 016: PIS/COFINS por regime. Lucro real resolve as linhas
/// não-cumulativas (165/760); lucro presumido segue em 65/300 (linha específica
/// de mesmo valor); o fallback legado (Simples sem perfil) continua caindo nas
/// linhas genéricas 65/300 — a resolução por especificidade decide.
#[tokio::test]
async fn regime_do_perfil_resolve_pis_cofins_especificos() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let provider = PostgresAliquotaProvider::new(pool.clone());
    let classe = ClasseTributaria::padrao();

    in_tenant(new_tenant_id(), async move {
        let mut lucro_real = PerfilFiscal::padrao_legado();
        lucro_real.regime = finledger::fiscal::domain::tributacao::RegimeTributario::LucroReal;
        let r = provider
            .resolver(dia(2026, 7, 21), &lucro_real, &classe, "84716053")
            .await
            .expect("resolver lucro real");
        assert_eq!(r.pis.expect("pis").bps(), 165, "PIS não-cumulativo");
        assert_eq!(r.cofins.expect("cofins").bps(), 760, "COFINS não-cumulativo");

        let mut presumido = PerfilFiscal::padrao_legado();
        presumido.regime =
            finledger::fiscal::domain::tributacao::RegimeTributario::LucroPresumido;
        let r = provider
            .resolver(dia(2026, 7, 21), &presumido, &classe, "84716053")
            .await
            .expect("resolver lucro presumido");
        assert_eq!(r.pis.expect("pis").bps(), 65);
        assert_eq!(r.cofins.expect("cofins").bps(), 300);

        // Fallback legado (Simples, sem linha específica de regime): as linhas
        // genéricas continuam no caminho — trava do cliente sem perfil.
        let legado = PerfilFiscal::padrao_legado();
        let r = provider
            .resolver(dia(2026, 7, 21), &legado, &classe, "84716053")
            .await
            .expect("resolver fallback legado");
        assert_eq!(r.pis.expect("pis").bps(), 65);
        assert_eq!(r.cofins.expect("cofins").bps(), 300);
        assert_eq!(r.icms.expect("icms").bps(), 1800, "ICMS de SP intacto");
    })
    .await;
    Ok(())
}

/// Seed da migração 016: ICMS interno modal por UF — um perfil em outra UF
/// resolve a alíquota daquela UF, sem afetar SP.
#[tokio::test]
async fn icms_interno_por_uf_do_seed_016() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let provider = PostgresAliquotaProvider::new(pool.clone());
    let classe = ClasseTributaria::padrao();

    in_tenant(new_tenant_id(), async move {
        let casos: &[(&str, i32)] = &[("RJ", 2200), ("MG", 1800), ("PI", 2250), ("SP", 1800)];
        for &(uf, esperado) in casos {
            let mut perfil = PerfilFiscal::padrao_legado();
            perfil.uf = finledger::fiscal::domain::tributacao::Uf::try_from(uf.to_string())
                .expect("UF válida");
            let r = provider
                .resolver(dia(2026, 7, 21), &perfil, &classe, "84716053")
                .await
                .expect("resolver UF");
            assert_eq!(r.icms.expect("icms").bps(), esperado, "ICMS interno de {uf}");
        }
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn linha_mais_especifica_vence_a_generica() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    // Linha específica por UF+NCM: cerveja (2203) com IS de 25% — mais
    // específica que qualquer curinga.
    sqlx::query(
        "INSERT INTO ref_aliquotas (tributo, uf, ncm_prefixo, aliquota_bps, vigencia_inicio)
         VALUES ('is', NULL, '2203', 2500, '2027-01-01'),
                ('cbs', NULL, '2203', 500, '2027-01-01')",
    )
    .execute(&pool)
    .await?;

    let provider = PostgresAliquotaProvider::new(pool.clone());
    let perfil = PerfilFiscal::padrao_legado();
    let classe = ClasseTributaria::padrao();

    in_tenant(new_tenant_id(), async move {
        // NCM de cerveja casa com o prefixo → CBS específica (500) vence a
        // genérica (880); IS aparece.
        let cerveja = provider
            .resolver(dia(2027, 6, 1), &perfil, &classe, "22030000")
            .await
            .expect("resolver cerveja");
        assert_eq!(cerveja.cbs.expect("cbs").bps(), 500);
        assert_eq!(cerveja.is_seletivo.expect("is").bps(), 2500);

        // NCM qualquer não casa com o prefixo → fica na genérica, sem IS.
        let generico = provider
            .resolver(dia(2027, 6, 1), &perfil, &classe, "84716053")
            .await
            .expect("resolver genérico");
        assert_eq!(generico.cbs.expect("cbs").bps(), 880);
        assert!(generico.is_seletivo.is_none());
    })
    .await;
    Ok(())
}

#[tokio::test]
async fn override_do_tenant_vence_a_referencia_e_nao_vaza_para_outro_tenant() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    let tenant_a = create_tenant(&pool, "tenant-a").await?;
    let tenant_b = create_tenant(&pool, "tenant-b").await?;

    // Benefício fiscal próprio do tenant A: ICMS reduzido a 12%.
    sqlx::query(
        "INSERT INTO aliquotas_tenant (tenant_id, tributo, aliquota_bps, vigencia_inicio)
         VALUES ($1, 'icms', 1200, '2000-01-01')",
    )
    .bind(tenant_a)
    .execute(&pool)
    .await?;

    let perfil = PerfilFiscal::padrao_legado();
    let classe = ClasseTributaria::padrao();

    let provider_a = PostgresAliquotaProvider::new(pool.clone());
    let icms_a = in_tenant(tenant_a, async {
        provider_a
            .resolver(dia(2026, 7, 21), &perfil, &classe, "84716053")
            .await
            .expect("resolver tenant A")
    })
    .await;
    assert_eq!(icms_a.icms.expect("icms A").bps(), 1200, "override do tenant vence");

    let provider_b = PostgresAliquotaProvider::new(pool.clone());
    let perfil_b = PerfilFiscal::padrao_legado();
    let classe_b = ClasseTributaria::padrao();
    let icms_b = in_tenant(tenant_b, async {
        provider_b
            .resolver(dia(2026, 7, 21), &perfil_b, &classe_b, "84716053")
            .await
            .expect("resolver tenant B")
    })
    .await;
    assert_eq!(
        icms_b.icms.expect("icms B").bps(),
        1800,
        "override de A não pode vazar para B"
    );
    Ok(())
}

// ── Caminhos infelizes ───────────────────────────────────────────────────────

#[tokio::test]
async fn resolver_sem_tenant_em_escopo_falha() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let provider = PostgresAliquotaProvider::new(pool.clone());

    // Fora de in_tenant não há CURRENT_TENANT — deve falhar com Unauthorized,
    // nunca resolver silenciosamente sem escopo.
    let r = provider
        .resolver(
            dia(2026, 7, 21),
            &PerfilFiscal::padrao_legado(),
            &ClasseTributaria::padrao(),
            "84716053",
        )
        .await;
    assert!(r.is_err(), "resolução sem tenant em escopo deve falhar");
    Ok(())
}

#[tokio::test]
async fn aliquota_absurda_na_tabela_e_rejeitada_na_resolucao() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;

    // 2_000_000 bps (20.000%) passa no CHECK >= 0 do banco, mas viola o teto
    // do value object Aliquota (20_000) — o provider deve recusar em vez de
    // calcular um imposto absurdo. Vigência mais recente que o seed para esta
    // linha ser a vencedora na resolução.
    sqlx::query(
        "INSERT INTO ref_aliquotas (tributo, aliquota_bps, vigencia_inicio)
         VALUES ('cbs', 2000000, '2026-06-01')",
    )
    .execute(&pool)
    .await?;

    let provider = PostgresAliquotaProvider::new(pool.clone());
    let r = in_tenant(new_tenant_id(), async {
        provider
            .resolver(
                dia(2026, 7, 21),
                &PerfilFiscal::padrao_legado(),
                &ClasseTributaria::padrao(),
                "84716053",
            )
            .await
    })
    .await;
    assert!(r.is_err(), "alíquota fora da faixa deve ser rejeitada");
    Ok(())
}

#[tokio::test]
async fn classe_desconhecida_cai_em_tributacao_integral() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let provider = PostgresAliquotaProvider::new(pool.clone());

    let inexistente = ClasseTributaria::try_from("999999".to_string()).expect("classe válida");
    let info = provider
        .classe_info(Some(&inexistente))
        .await
        .expect("classe_info");
    // Não falha a emissão: trata como integral (sem redução).
    assert_eq!(info.reducao_bps, 0);

    let conhecida = ClasseTributaria::try_from("200003".to_string()).expect("classe válida");
    let info = provider
        .classe_info(Some(&conhecida))
        .await
        .expect("classe_info");
    assert_eq!(info.reducao_bps, 6000, "classe seeded de redução 60%");
    assert_eq!(info.cst_ibs_cbs, "200");
    Ok(())
}

#[tokio::test]
async fn perfil_fiscal_incompleto_e_invalido_sao_rejeitados() -> TestResult {
    let (_c, pool) = start_postgres().await?;
    setup_db(&pool).await?;
    let tenant = create_tenant(&pool, "perfil-x").await?;
    let repo = TenantRepository::new(pool.clone());

    in_tenant(tenant, async move {
        // Regime sem UF/município/CRT → erro de validação, não fallback mudo.
        let incompleto = PerfilFiscalDto {
            regime_tributario: Some("lucro_real".into()),
            ..Default::default()
        };
        assert!(incompleto.para_dominio().is_err(), "perfil incompleto deve falhar");
        assert!(
            repo.atualizar_perfil_fiscal(incompleto).await.is_err(),
            "atualização com perfil incompleto deve falhar"
        );

        // Regime inexistente → erro.
        let regime_invalido = PerfilFiscalDto {
            regime_tributario: Some("lucro_imaginario".into()),
            uf: Some("SP".into()),
            codigo_municipio: Some("3550308".into()),
            crt: Some(3),
            ..Default::default()
        };
        assert!(regime_invalido.para_dominio().is_err());

        // UF inexistente → erro.
        let uf_invalida = PerfilFiscalDto {
            regime_tributario: Some("lucro_real".into()),
            uf: Some("XX".into()),
            codigo_municipio: Some("3550308".into()),
            crt: Some(3),
            ..Default::default()
        };
        assert!(uf_invalida.para_dominio().is_err());

        // CRT fora de 1..=4 → erro.
        let crt_invalido = PerfilFiscalDto {
            regime_tributario: Some("lucro_real".into()),
            uf: Some("SP".into()),
            codigo_municipio: Some("3550308".into()),
            crt: Some(9),
            ..Default::default()
        };
        assert!(crt_invalido.para_dominio().is_err());

        // Perfil completo e válido → round-trip persiste e volta igual.
        let valido = PerfilFiscalDto {
            regime_tributario: Some("lucro_real".into()),
            uf: Some("MG".into()),
            codigo_municipio: Some("3106200".into()),
            crt: Some(3),
            ibs_cbs_regime_regular: false,
            aliquota_das_bps: None,
        };
        repo.atualizar_perfil_fiscal(valido).await.expect("perfil válido persiste");
        let lido = repo.obter_perfil_fiscal().await.expect("obter perfil");
        assert_eq!(lido.regime_tributario.as_deref(), Some("lucro_real"));
        assert_eq!(lido.uf.as_deref(), Some("MG"));

        // Ausente (regime None) → Ok(None): fallback legado explícito.
        let vazio = PerfilFiscalDto::default();
        assert!(vazio.para_dominio().expect("perfil vazio é válido").is_none());
    })
    .await;
    Ok(())
}
