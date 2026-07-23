use serde::{Deserialize, Serialize};

use super::aliquota::Aliquota;
use super::classe_tributaria::ClasseTributariaInfo;
use super::fase_transicao::FaseTransicao;
use super::perfil_fiscal::{PerfilFiscal, RegimeTributario};
use crate::fiscal::domain::value_objects::ImpostoItem;

/// Alíquotas já resolvidas (pela infraestrutura) para um item, na data de
/// emissão e para o perfil do tenant. `None` = nenhuma linha vigente na
/// tabela → tributo não incide (0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AliquotasItem {
    pub icms: Option<Aliquota>,
    pub iss: Option<Aliquota>,
    pub pis: Option<Aliquota>,
    pub cofins: Option<Aliquota>,
    pub cbs: Option<Aliquota>,
    pub ibs_uf: Option<Aliquota>,
    pub ibs_mun: Option<Aliquota>,
    pub is_seletivo: Option<Aliquota>,
}

/// Contexto fixo de uma emissão: fase da transição (derivada da data) e o
/// perfil fiscal do tenant. Resolvido uma vez por NF.
#[derive(Debug, Clone)]
pub struct ContextoFiscal {
    pub fase: FaseTransicao,
    pub perfil: PerfilFiscal,
}

/// Serviço de domínio puro: aplica as regras da transição (quais tributos
/// incidem, fator de phase-down do ICMS/ISS, redução de base da classe
/// tributária) sobre alíquotas já resolvidas. Nenhum I/O — testável
/// table-driven sem banco.
pub struct MotorTributario;

impl MotorTributario {
    pub fn calcular_item(
        ctx: &ContextoFiscal,
        aliquotas: &AliquotasItem,
        classe: &ClasseTributariaInfo,
        base_centavos: i64,
    ) -> ImpostoItem {
        // Simples Nacional CONFIGURADO sem regime regular: nada de legados no
        // documento (CSOSN 102, recolhimento por dentro do DAS). O fallback
        // sem perfil (`padrao_legado`) NÃO entra aqui de propósito — ele deve
        // continuar emitindo os valores históricos do cliente em produção; o
        // caminho correto é o tenant configurar o perfil fiscal.
        let simples_por_dentro = ctx.perfil.simples_recolhe_por_dentro();
        let e_simples = ctx.perfil.regime == RegimeTributario::SimplesNacional;

        let aplicar = |a: Option<Aliquota>| a.unwrap_or_else(Aliquota::zero).aplicar(base_centavos);

        // ICMS/ISS: integrais até 2028, reduzidos 2029–2032, extintos em 2033.
        let fator_legado = ctx.fase.fator_legado_bps();
        let legado = |a: Option<Aliquota>| {
            if simples_por_dentro || !ctx.fase.cobra_legado_estadual() {
                return 0;
            }
            a.unwrap_or_else(Aliquota::zero)
                .aplicar_reduzida(base_centavos, 10_000 - fator_legado)
        };

        // PIS/COFINS: extintos a partir de 2027. A vigência na tabela já
        // encerra em 2026-12-31, mas a fase é a fonte de verdade — cinto e
        // suspensório contra tabela mal semeada.
        let pis_cofins = |a: Option<Aliquota>| {
            if simples_por_dentro || !ctx.fase.cobra_pis_cofins() {
                0
            } else {
                aplicar(a)
            }
        };

        // CBS/IBS/IS. PREMISSA: o preço praticado é BRUTO — embute os tributos.
        //
        // - Até 2026 (e sempre que o destaque é informativo — Simples sem
        //   regime regular, incluindo o fallback legado): CBS/IBS calculados
        //   diretamente sobre o preço, como manda o ano-teste.
        // - De 2027 em diante, para perfis que destacam de verdade: os
        //   tributos são "por fora" da base, então a base é EXTRAÍDA por
        //   dentro do preço, tal que base + IS(base) + CBS/IBS(base+IS) =
        //   preço. O total da NF permanece = preço (retrocompatível).
        let (cbs, ibs_uf, ibs_mun, is_seletivo) =
            if ctx.fase.base_ibs_cbs_por_fora() && !ctx.perfil.ibs_cbs_informativo() {
                Self::destacar_por_fora(aliquotas, classe, base_centavos)
            } else {
                let ibs_cbs = |a: Option<Aliquota>| {
                    if !ctx.fase.destaca_ibs_cbs() {
                        return 0;
                    }
                    a.unwrap_or_else(Aliquota::zero)
                        .aplicar_reduzida(base_centavos, classe.reducao_bps)
                };
                // Imposto Seletivo: só existe a partir de 2027 (LC 214) e
                // incide por NCM — a resolução por NCM já aconteceu no
                // provider; a redução de classe NÃO se aplica a ele.
                let is_seletivo =
                    if matches!(ctx.fase, FaseTransicao::Legado | FaseTransicao::Teste2026) {
                        0
                    } else {
                        aplicar(aliquotas.is_seletivo)
                    };
                (
                    ibs_cbs(aliquotas.cbs),
                    ibs_cbs(aliquotas.ibs_uf),
                    ibs_cbs(aliquotas.ibs_mun),
                    is_seletivo,
                )
            };

        // Custo do DAS (Simples configurado): alíquota efetiva do anexo/faixa
        // sobre o preço. Sem alíquota configurada o custo fica 0 — avisa em
        // log em vez de falhar a emissão.
        let das = if simples_por_dentro {
            match ctx.perfil.aliquota_das_bps {
                Some(a) => a.aplicar(base_centavos),
                None => {
                    tracing::warn!(
                        "Simples Nacional configurado sem aliquota_das_bps: custo do DAS \
                         considerado 0 — configure a alíquota efetiva do Simples no perfil fiscal"
                    );
                    0
                }
            }
        } else {
            0
        };

        ImpostoItem {
            icms_centavos: legado(aliquotas.icms),
            iss_centavos: legado(aliquotas.iss),
            pis_centavos: pis_cofins(aliquotas.pis),
            cofins_centavos: pis_cofins(aliquotas.cofins),
            cbs_centavos: cbs,
            ibs_uf_centavos: ibs_uf,
            ibs_mun_centavos: ibs_mun,
            is_centavos: is_seletivo,
            c_class_trib: Some(classe.classe.as_str().to_string()),
            cst_ibs_cbs: Some(classe.cst_ibs_cbs.clone()),
            // Simples → CSOSN; regimes normais → CST do ICMS. Caso geral sem ST
            // (a marcação de ST por classe/NCM é deferida — issue #16).
            csosn: e_simples.then(|| "102".to_string()),
            cst_icms: (!e_simples).then(|| "00".to_string()),
            das_centavos: das,
        }
    }

    /// Extrai a base "por dentro" do preço bruto e destaca IS/CBS/IBS sobre
    /// ela (fases 2027+): com IS ad valorem `s` e soma efetiva de CBS+IBS `t`
    /// (bps, já com a redução de classe), a base fecha em
    /// `base = preço × 10⁸ / ((10⁴ + s) × (10⁴ + t))`, pois
    /// base + IS(base) + (CBS+IBS)(base + IS) = preço. Para não perder
    /// precisão da redução de classe, `t` é mantido escalado por 10⁴
    /// (bps × bps). Tudo em i128 com arredondamento half-up; o resíduo de
    /// arredondamento (≤ poucos centavos) vai para o maior tributo, de modo
    /// que base + tributos = preço EXATO.
    fn destacar_por_fora(
        aliquotas: &AliquotasItem,
        classe: &ClasseTributariaInfo,
        preco_centavos: i64,
    ) -> (i64, i64, i64, i64) {
        let half_up = |numerador: i128, divisor: i128| -> i128 {
            let q = (numerador.abs() + divisor / 2) / divisor;
            if numerador < 0 { -q } else { q }
        };

        let reducao = classe.reducao_bps.clamp(0, 10_000);
        // A redução de classe NÃO se aplica ao IS (LC 214).
        let s = aliquotas.is_seletivo.map_or(0, |a| a.bps() as i128);
        // Alíquota efetiva de cada CBS/IBS escalada por 10⁴: bps × (10⁴ − red).
        let efetiva_e4 =
            |a: Option<Aliquota>| a.map_or(0, |a| a.bps() as i128 * (10_000 - reducao) as i128);
        let t_e4 = efetiva_e4(aliquotas.cbs)
            + efetiva_e4(aliquotas.ibs_uf)
            + efetiva_e4(aliquotas.ibs_mun);

        // base = preço × 10¹² / ((10⁴ + s) × (10⁸ + t×10⁴)) — mesmo valor da
        // fórmula acima, com t escalado.
        let denominador = (10_000 + s) * (100_000_000 + t_e4);
        let base = half_up(preco_centavos as i128 * 1_000_000_000_000, denominador) as i64;

        let is_seletivo = half_up(base as i128 * s, 10_000) as i64;
        let base_ibs_cbs = base + is_seletivo;
        let destacar = |a: Option<Aliquota>| {
            a.unwrap_or_else(Aliquota::zero)
                .aplicar_reduzida(base_ibs_cbs, reducao)
        };
        let mut cbs = destacar(aliquotas.cbs);
        let mut ibs_uf = destacar(aliquotas.ibs_uf);
        let mut ibs_mun = destacar(aliquotas.ibs_mun);
        let mut is_v = is_seletivo;

        // Fechamento exato: o resíduo dos arredondamentos individuais vai para
        // o maior tributo (nunca distorce mais que ±poucos centavos; se todos
        // são zero o resíduo também é — base = preço quando não há tributo).
        let residuo = preco_centavos - base - is_v - cbs - ibs_uf - ibs_mun;
        if residuo != 0 {
            let maior = [&mut cbs, &mut ibs_uf, &mut ibs_mun, &mut is_v]
                .into_iter()
                .max_by_key(|t| **t);
            if let Some(maior) = maior
                && *maior > 0
            {
                *maior += residuo;
            }
        }
        (cbs, ibs_uf, ibs_mun, is_v)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn bps(v: i32) -> Option<Aliquota> {
        Some(Aliquota::try_from(v).expect("bps válido"))
    }

    /// Alíquotas típicas de cada fase, como o provider as resolveria.
    fn aliquotas_para(ano: i32) -> AliquotasItem {
        match ano {
            ..=2025 => AliquotasItem {
                icms: bps(1800),
                pis: bps(65),
                cofins: bps(300),
                ..Default::default()
            },
            2026 => AliquotasItem {
                icms: bps(1800),
                pis: bps(65),
                cofins: bps(300),
                cbs: bps(90),
                ibs_uf: bps(5),
                ibs_mun: bps(5),
                ..Default::default()
            },
            2027..=2028 => AliquotasItem {
                icms: bps(1800),
                cbs: bps(880),
                ibs_uf: bps(5),
                ibs_mun: bps(5),
                ..Default::default()
            },
            _ => AliquotasItem {
                icms: bps(1800),
                cbs: bps(880),
                ibs_uf: bps(1400),
                ibs_mun: bps(350),
                ..Default::default()
            },
        }
    }

    fn ctx(ano: i32) -> ContextoFiscal {
        ContextoFiscal {
            fase: FaseTransicao::de_data(NaiveDate::from_ymd_opt(ano, 6, 15).expect("data")),
            perfil: PerfilFiscal::padrao_legado(),
        }
    }

    const BASE: i64 = 100_000; // R$ 1.000,00

    #[test]
    fn tabela_de_fases_classe_integral() {
        let classe = ClasseTributariaInfo::integral();
        // (ano, icms, pis, cofins, cbs, ibs_uf, ibs_mun)
        let casos: &[(i32, i64, i64, i64, i64, i64, i64)] = &[
            (2025, 18_000, 650, 3_000, 0, 0, 0),
            (2026, 18_000, 650, 3_000, 900, 50, 50),
            (2027, 18_000, 0, 0, 8_800, 50, 50),
            (2028, 18_000, 0, 0, 8_800, 50, 50),
            (2029, 16_200, 0, 0, 8_800, 14_000, 3_500), // ICMS a 90%
            (2030, 14_400, 0, 0, 8_800, 14_000, 3_500), // 80%
            (2031, 12_600, 0, 0, 8_800, 14_000, 3_500), // 70%
            (2032, 10_800, 0, 0, 8_800, 14_000, 3_500), // 60%
            (2033, 0, 0, 0, 8_800, 14_000, 3_500),      // extinto
        ];
        for &(ano, icms, pis, cofins, cbs, ibs_uf, ibs_mun) in casos {
            let imposto =
                MotorTributario::calcular_item(&ctx(ano), &aliquotas_para(ano), &classe, BASE);
            assert_eq!(imposto.icms_centavos, icms, "icms {ano}");
            assert_eq!(imposto.pis_centavos, pis, "pis {ano}");
            assert_eq!(imposto.cofins_centavos, cofins, "cofins {ano}");
            assert_eq!(imposto.cbs_centavos, cbs, "cbs {ano}");
            assert_eq!(imposto.ibs_uf_centavos, ibs_uf, "ibs_uf {ano}");
            assert_eq!(imposto.ibs_mun_centavos, ibs_mun, "ibs_mun {ano}");
        }
    }

    #[test]
    fn reducao_de_classe_aplica_so_em_ibs_cbs() {
        let classe = ClasseTributariaInfo {
            classe: super::super::ClasseTributaria::try_from("200003".to_string()).expect("classe"),
            cst_ibs_cbs: "200".into(),
            reducao_bps: 6000, // redução de 60%
        };
        let imposto =
            MotorTributario::calcular_item(&ctx(2033), &aliquotas_para(2033), &classe, BASE);
        assert_eq!(imposto.cbs_centavos, 3_520); // 8,8% × 40%
        assert_eq!(imposto.ibs_uf_centavos, 5_600); // 14% × 40%
        assert_eq!(imposto.ibs_mun_centavos, 1_400); // 3,5% × 40%
    }

    #[test]
    fn aliquota_zero_da_classe_zera_ibs_cbs() {
        let classe = ClasseTributariaInfo {
            classe: super::super::ClasseTributaria::try_from("410001".to_string()).expect("classe"),
            cst_ibs_cbs: "410".into(),
            reducao_bps: 10_000,
        };
        let imposto =
            MotorTributario::calcular_item(&ctx(2026), &aliquotas_para(2026), &classe, BASE);
        assert_eq!(imposto.cbs_centavos, 0);
        assert_eq!(imposto.ibs_uf_centavos, 0);
        // Legados não são afetados pela classe da reforma.
        assert_eq!(imposto.icms_centavos, 18_000);
    }

    #[test]
    fn imposto_seletivo_so_a_partir_de_2027_e_sem_reducao_de_classe() {
        let classe = ClasseTributariaInfo {
            classe: super::super::ClasseTributaria::try_from("200003".to_string()).expect("classe"),
            cst_ibs_cbs: "200".into(),
            reducao_bps: 6000,
        };
        let mut aliquotas = aliquotas_para(2026);
        aliquotas.is_seletivo = bps(2500);
        let em_2026 = MotorTributario::calcular_item(&ctx(2026), &aliquotas, &classe, BASE);
        assert_eq!(em_2026.is_centavos, 0, "IS não existe no ano-teste");

        let mut aliquotas = aliquotas_para(2027);
        aliquotas.is_seletivo = bps(2500);
        let em_2027 = MotorTributario::calcular_item(&ctx(2027), &aliquotas, &classe, BASE);
        assert_eq!(em_2027.is_centavos, 25_000, "IS integral — redução de classe não se aplica");
    }

    /// A opção pelo regime regular de IBS/CBS no Simples muda o RECOLHIMENTO
    /// (DAS × por fora), não os montantes destacados no documento — o motor
    /// deve produzir exatamente o mesmo resultado com a flag ligada/desligada.
    #[test]
    fn simples_regime_regular_nao_altera_montantes() {
        let classe = ClasseTributariaInfo::integral();
        let base = ctx(2026);
        let mut com_opcao = base.clone();
        com_opcao.perfil.ibs_cbs_regime_regular = true;

        let sem = MotorTributario::calcular_item(&base, &aliquotas_para(2026), &classe, BASE);
        let com = MotorTributario::calcular_item(&com_opcao, &aliquotas_para(2026), &classe, BASE);
        assert_eq!(sem.cbs_centavos, com.cbs_centavos);
        assert_eq!(sem.ibs_uf_centavos, com.ibs_uf_centavos);
        assert_eq!(sem.icms_centavos, com.icms_centavos);
        // E os montantes não são triviais (zero) — o teste falharia se o motor
        // ignorasse as alíquotas por completo.
        assert_eq!(sem.cbs_centavos, 900);
    }

    #[test]
    fn base_zero_resulta_impostos_zero() {
        let imposto = MotorTributario::calcular_item(
            &ctx(2026),
            &aliquotas_para(2026),
            &ClasseTributariaInfo::integral(),
            0,
        );
        assert_eq!(imposto.icms_centavos, 0);
        assert_eq!(imposto.cbs_centavos, 0);
    }

    #[test]
    fn tributo_sem_linha_vigente_resulta_zero() {
        let imposto = MotorTributario::calcular_item(
            &ctx(2026),
            &AliquotasItem::default(),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.icms_centavos, 0);
        assert_eq!(imposto.cbs_centavos, 0);
    }

    #[test]
    fn classificacao_registrada_no_imposto() {
        let imposto = MotorTributario::calcular_item(
            &ctx(2026),
            &aliquotas_para(2026),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.c_class_trib.as_deref(), Some("000001"));
        assert_eq!(imposto.cst_ibs_cbs.as_deref(), Some("000"));
    }

    /// Perfil que destaca CBS/IBS de verdade (não informativo): regime regular.
    fn ctx_destaca(ano: i32) -> ContextoFiscal {
        let mut c = ctx(ano);
        c.perfil.ibs_cbs_regime_regular = true;
        c
    }

    /// Base "por fora" (2027+, perfil que destaca): o preço é BRUTO e a base é
    /// extraída por dentro — base + IS(base) + CBS/IBS(base+IS) = preço EXATO.
    /// Valores esperados conferidos manualmente com a fórmula
    /// base = preço × 10⁸ / ((10⁴+s) × (10⁴+t)).
    #[test]
    fn base_por_fora_fecha_exato_com_o_preco() {
        // (nome, ano, is_bps, reducao_bps, esperado (base, is, cbs, ibs_uf, ibs_mun))
        type CasoBaseForaFora = (&'static str, i32, i32, i32, (i64, i64, i64, i64, i64));
        let casos: &[CasoBaseForaFora] = &[
            // 2027: t = 880+5+5 = 890 bps → base = 100000/1,089 = 91.827.
            ("2027 sem IS", 2027, 0, 0, (91_827, 0, 8_081, 46, 46)),
            // 2033: t = 880+1400+350 = 2630 bps → base 79.177; resíduo −1 no maior (ibs_uf).
            ("2033 sem IS", 2033, 0, 0, (79_177, 0, 6_968, 11_084, 2_771)),
            // 2033 com IS 25%: base 63.341; IS 15.835 + resíduo +1 → 15.836.
            ("2033 com IS", 2033, 2500, 0, (63_341, 15_836, 6_967, 11_085, 2_771)),
            // 2033 com redução de classe 60%: t efetivo 1052 bps → base 90.481.
            ("2033 reduzido", 2033, 0, 6000, (90_481, 0, 3_185, 5_067, 1_267)),
        ];
        for &(nome, ano, is_bps, reducao_bps, (base, is_v, cbs, ibs_uf, ibs_mun)) in casos {
            let classe = ClasseTributariaInfo {
                reducao_bps,
                ..ClasseTributariaInfo::integral()
            };
            let mut aliquotas = aliquotas_para(ano);
            if is_bps > 0 {
                aliquotas.is_seletivo = bps(is_bps);
            }
            let imposto =
                MotorTributario::calcular_item(&ctx_destaca(ano), &aliquotas, &classe, BASE);
            assert_eq!(imposto.is_centavos, is_v, "IS {nome}");
            assert_eq!(imposto.cbs_centavos, cbs, "CBS {nome}");
            assert_eq!(imposto.ibs_uf_centavos, ibs_uf, "IBS UF {nome}");
            assert_eq!(imposto.ibs_mun_centavos, ibs_mun, "IBS mun {nome}");
            // Fechamento exato: base + tributos por fora = preço bruto.
            let soma = base
                + imposto.is_centavos
                + imposto.cbs_centavos
                + imposto.ibs_uf_centavos
                + imposto.ibs_mun_centavos;
            assert_eq!(soma, BASE, "base + tributos deve fechar no preço ({nome})");
        }
    }

    /// Em 2027–2032 o ICMS continua incidindo "por dentro" do preço bruto —
    /// a extração da base vale só para IS/CBS/IBS.
    #[test]
    fn base_por_fora_nao_afeta_o_icms() {
        let imposto = MotorTributario::calcular_item(
            &ctx_destaca(2027),
            &aliquotas_para(2027),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.icms_centavos, 18_000, "ICMS sobre o preço bruto");
    }

    /// Perfil informativo (Simples sem regime regular, incluindo o fallback
    /// legado) NÃO extrai base por fora em 2027+: CBS/IBS seguem calculados
    /// sobre o preço, como hoje — o fallback do cliente em produção não muda.
    #[test]
    fn perfil_informativo_nao_extrai_base_por_fora() {
        let imposto = MotorTributario::calcular_item(
            &ctx(2027), // padrao_legado → informativo
            &aliquotas_para(2027),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.cbs_centavos, 8_800, "CBS informativa sobre o preço");
        assert_eq!(imposto.ibs_uf_centavos, 50);
    }

    /// Simples Nacional CONFIGURADO sem regime regular: legados não são
    /// destacados (CSOSN 102), CBS/IBS informativos de 2026 permanecem e o
    /// custo do vendedor é o DAS.
    #[test]
    fn simples_configurado_zera_legados_e_usa_das() {
        let mut ctx = ctx(2026);
        ctx.perfil.configurado = true;
        ctx.perfil.aliquota_das_bps = Some(Aliquota::try_from(700).expect("7%"));
        let imposto = MotorTributario::calcular_item(
            &ctx,
            &aliquotas_para(2026),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.icms_centavos, 0, "Simples não destaca ICMS");
        assert_eq!(imposto.pis_centavos, 0);
        assert_eq!(imposto.cofins_centavos, 0);
        assert_eq!(imposto.cbs_centavos, 900, "CBS informativa de 2026 permanece");
        assert_eq!(imposto.csosn.as_deref(), Some("102"));
        assert_eq!(imposto.das_centavos, 7_000, "DAS 7% sobre a base");
        assert_eq!(
            imposto.custo_vendedor_centavos(true),
            7_000,
            "custo do vendedor = DAS (IBS/CBS informativos ficam fora)"
        );
    }

    /// Simples configurado SEM alíquota do DAS: legados continuam zerados e o
    /// custo cai para 0 (com aviso em log) — nunca um erro de emissão.
    #[test]
    fn simples_configurado_sem_das_tem_custo_zero() {
        let mut ctx = ctx(2026);
        ctx.perfil.configurado = true;
        let imposto = MotorTributario::calcular_item(
            &ctx,
            &aliquotas_para(2026),
            &ClasseTributariaInfo::integral(),
            BASE,
        );
        assert_eq!(imposto.icms_centavos, 0);
        assert_eq!(imposto.das_centavos, 0);
        assert_eq!(imposto.custo_vendedor_centavos(true), 0);
    }

    /// Trava de regressão do comportamento legado: mesmas alíquotas do antigo
    /// `ImpostoItem::calcular` hardcoded (18% / 0,65% / 3%), agora em
    /// aritmética inteira.
    #[test]
    fn comportamento_legado_preservado_em_2025() {
        let imposto = MotorTributario::calcular_item(
            &ctx(2025),
            &aliquotas_para(2025),
            &ClasseTributariaInfo::integral(),
            12_345, // valor "quebrado" para exercitar arredondamento
        );
        assert_eq!(imposto.icms_centavos, 2_222); // 12345 × 0,18 = 2222,1 → 2222
        assert_eq!(imposto.pis_centavos, 80); // 12345 × 0,0065 = 80,2 → 80
        assert_eq!(imposto.cofins_centavos, 370); // 12345 × 0,03 = 370,35 → 370
        assert_eq!(imposto.cbs_centavos, 0);
    }
}
