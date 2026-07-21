use chrono::{Datelike, NaiveDate};

/// Fase da transição da reforma tributária (EC 132/2023, art. 125-133 ADCT),
/// determinada exclusivamente pela data de emissão do documento.
///
/// O *fator* de redução do ICMS/ISS em 2029–2032 é regra constitucional
/// uniforme — por isso vive aqui, no código; as alíquotas em si são dados
/// (`ref_aliquotas`), com vigência própria por tributo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaseTransicao {
    /// Antes de 2026: apenas os tributos atuais (ICMS/ISS/PIS/COFINS).
    Legado,
    /// 2026: ano-teste — CBS 0,9% + IBS 0,1% destacados de forma informativa,
    /// tributos atuais integrais.
    Teste2026,
    /// 2027–2028: CBS plena, PIS/COFINS extintos, IBS simbólico (0,05% UF +
    /// 0,05% município).
    Cbs2027_2028,
    /// 2029–2032: ICMS/ISS reduzidos a 90/80/70/60% do valor cheio; IBS sobe
    /// proporcionalmente (alíquotas na tabela).
    ReducaoIcmsIss { fator_legado_bps: i32 },
    /// A partir de 2033: apenas CBS/IBS/IS.
    Plena2033,
}

impl FaseTransicao {
    pub fn de_data(data: NaiveDate) -> Self {
        match data.year() {
            ..=2025 => Self::Legado,
            2026 => Self::Teste2026,
            2027..=2028 => Self::Cbs2027_2028,
            2029 => Self::ReducaoIcmsIss { fator_legado_bps: 9000 },
            2030 => Self::ReducaoIcmsIss { fator_legado_bps: 8000 },
            2031 => Self::ReducaoIcmsIss { fator_legado_bps: 7000 },
            2032 => Self::ReducaoIcmsIss { fator_legado_bps: 6000 },
            _ => Self::Plena2033,
        }
    }

    /// ICMS/ISS ainda incidem nesta fase? (integral ou reduzido)
    pub fn cobra_legado_estadual(&self) -> bool {
        !matches!(self, Self::Plena2033)
    }

    /// PIS/COFINS ainda incidem nesta fase? (extintos a partir de 2027)
    pub fn cobra_pis_cofins(&self) -> bool {
        matches!(self, Self::Legado | Self::Teste2026)
    }

    /// CBS/IBS aparecem no documento nesta fase?
    pub fn destaca_ibs_cbs(&self) -> bool {
        !matches!(self, Self::Legado)
    }

    /// Fator (em bps de 10000) aplicado sobre o ICMS/ISS cheio.
    pub fn fator_legado_bps(&self) -> i32 {
        match self {
            Self::ReducaoIcmsIss { fator_legado_bps } => *fator_legado_bps,
            Self::Plena2033 => 0,
            _ => 10_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(ano: i32) -> NaiveDate {
        NaiveDate::from_ymd_opt(ano, 6, 15).expect("data válida")
    }

    #[test]
    fn fases_por_ano() {
        assert_eq!(FaseTransicao::de_data(d(2025)), FaseTransicao::Legado);
        assert_eq!(FaseTransicao::de_data(d(2026)), FaseTransicao::Teste2026);
        assert_eq!(FaseTransicao::de_data(d(2027)), FaseTransicao::Cbs2027_2028);
        assert_eq!(FaseTransicao::de_data(d(2028)), FaseTransicao::Cbs2027_2028);
        assert_eq!(
            FaseTransicao::de_data(d(2029)),
            FaseTransicao::ReducaoIcmsIss { fator_legado_bps: 9000 }
        );
        assert_eq!(
            FaseTransicao::de_data(d(2032)),
            FaseTransicao::ReducaoIcmsIss { fator_legado_bps: 6000 }
        );
        assert_eq!(FaseTransicao::de_data(d(2033)), FaseTransicao::Plena2033);
        assert_eq!(FaseTransicao::de_data(d(2040)), FaseTransicao::Plena2033);
    }

    #[test]
    fn fronteira_de_virada_de_ano() {
        let ultimo_dia_2026 = NaiveDate::from_ymd_opt(2026, 12, 31).expect("data");
        let primeiro_dia_2027 = NaiveDate::from_ymd_opt(2027, 1, 1).expect("data");
        assert_eq!(FaseTransicao::de_data(ultimo_dia_2026), FaseTransicao::Teste2026);
        assert_eq!(
            FaseTransicao::de_data(primeiro_dia_2027),
            FaseTransicao::Cbs2027_2028
        );
    }

    #[test]
    fn flags_por_fase() {
        assert!(FaseTransicao::Legado.cobra_pis_cofins());
        assert!(!FaseTransicao::Legado.destaca_ibs_cbs());
        assert!(FaseTransicao::Teste2026.cobra_pis_cofins());
        assert!(FaseTransicao::Teste2026.destaca_ibs_cbs());
        assert!(!FaseTransicao::Cbs2027_2028.cobra_pis_cofins());
        assert!(FaseTransicao::Plena2033.destaca_ibs_cbs());
        assert!(!FaseTransicao::Plena2033.cobra_legado_estadual());
        assert_eq!(FaseTransicao::Plena2033.fator_legado_bps(), 0);
        assert_eq!(FaseTransicao::Teste2026.fator_legado_bps(), 10_000);
    }
}
