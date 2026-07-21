use serde::{Deserialize, Serialize};

use super::aliquota::Aliquota;
use super::classe_tributaria::ClasseTributariaInfo;
use super::fase_transicao::FaseTransicao;
use super::perfil_fiscal::PerfilFiscal;
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
        let aplicar = |a: Option<Aliquota>| a.unwrap_or_else(Aliquota::zero).aplicar(base_centavos);

        // ICMS/ISS: integrais até 2028, reduzidos 2029–2032, extintos em 2033.
        let fator_legado = ctx.fase.fator_legado_bps();
        let legado = |a: Option<Aliquota>| {
            if !ctx.fase.cobra_legado_estadual() {
                return 0;
            }
            aplicar(a.map(|al| al.reduzida(10_000 - fator_legado)))
        };

        // PIS/COFINS: extintos a partir de 2027. A vigência na tabela já
        // encerra em 2026-12-31, mas a fase é a fonte de verdade — cinto e
        // suspensório contra tabela mal semeada.
        let pis_cofins = |a: Option<Aliquota>| {
            if ctx.fase.cobra_pis_cofins() { aplicar(a) } else { 0 }
        };

        // CBS/IBS: destacados a partir de 2026, com a redução de base da
        // classe tributária do item (LC 214: redução 60%, alíquota zero, ...).
        // No Simples sem opção pelo regime regular os valores são meramente
        // informativos no documento — o montante calculado é o mesmo; o que
        // muda é o recolhimento (via DAS), fora do escopo da NF.
        let ibs_cbs = |a: Option<Aliquota>| {
            if !ctx.fase.destaca_ibs_cbs() {
                return 0;
            }
            aplicar(a.map(|al| al.reduzida(classe.reducao_bps)))
        };

        // Imposto Seletivo: só existe a partir de 2027 (LC 214) e incide por
        // NCM — a resolução por NCM já aconteceu no provider; a redução de
        // classe NÃO se aplica a ele.
        let is_seletivo = if matches!(ctx.fase, FaseTransicao::Legado | FaseTransicao::Teste2026) {
            0
        } else {
            aplicar(aliquotas.is_seletivo)
        };

        ImpostoItem {
            icms_centavos: legado(aliquotas.icms),
            iss_centavos: legado(aliquotas.iss),
            pis_centavos: pis_cofins(aliquotas.pis),
            cofins_centavos: pis_cofins(aliquotas.cofins),
            cbs_centavos: ibs_cbs(aliquotas.cbs),
            ibs_uf_centavos: ibs_cbs(aliquotas.ibs_uf),
            ibs_mun_centavos: ibs_cbs(aliquotas.ibs_mun),
            is_centavos: is_seletivo,
            c_class_trib: Some(classe.classe.as_str().to_string()),
            cst_ibs_cbs: Some(classe.cst_ibs_cbs.clone()),
        }
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
