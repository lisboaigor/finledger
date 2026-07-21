use pharos_core::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Tributos que o motor conhece. Cada variante corresponde a um valor da
/// coluna `ref_aliquotas.tributo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TributoTipo {
    Icms,
    Iss,
    Pis,
    Cofins,
    Cbs,
    IbsUf,
    IbsMun,
    /// Imposto Seletivo (LC 214/2025) — incide por NCM sobre bens específicos.
    Is,
}

impl TributoTipo {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Icms => "icms",
            Self::Iss => "iss",
            Self::Pis => "pis",
            Self::Cofins => "cofins",
            Self::Cbs => "cbs",
            Self::IbsUf => "ibs_uf",
            Self::IbsMun => "ibs_mun",
            Self::Is => "is",
        }
    }
}

/// Alíquota em basis points (1 bps = 0,01%). Aritmética inteira — nada de
/// `f64` em dinheiro (mesma razão de `shared::Dinheiro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Aliquota(i32);

impl Aliquota {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn bps(&self) -> i32 {
        self.0
    }

    /// Aplica a alíquota sobre uma base em centavos, arredondamento half-up.
    /// i128 no intermediário para nunca estourar (base máx. i64 × 10⁴ bps).
    pub fn aplicar(&self, base_centavos: i64) -> i64 {
        let produto = base_centavos as i128 * self.0 as i128;
        ((produto + 5_000) / 10_000) as i64
    }

    /// Reduz a alíquota por um fator em bps (ex.: redução de 60% da LC 214 →
    /// `reducao_bps = 6000`; fator de transição do ICMS 2029–2032 → 9000..6000).
    pub fn reduzida(&self, reducao_bps: i32) -> Self {
        let restante = (10_000 - reducao_bps).clamp(0, 10_000);
        Self(((self.0 as i64 * restante as i64) / 10_000) as i32)
    }
}

impl TryFrom<i32> for Aliquota {
    type Error = DomainError;

    fn try_from(bps: i32) -> DomainResult<Self> {
        // Teto folgado (200%): pega erro de unidade (% digitado como bps × 100)
        // sem impedir tributos altos como o IS sobre bens específicos.
        if !(0..=20_000).contains(&bps) {
            return Err(DomainError::Validation(format!(
                "Alíquota em bps fora da faixa 0..=20000: {bps}"
            )));
        }
        Ok(Self(bps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aplicar_arredonda_half_up() {
        let a = Aliquota::try_from(1800).expect("18%");
        assert_eq!(a.aplicar(10_000), 1_800); // R$ 100,00 → R$ 18,00
        // 0,65% de R$ 100,00 = 65 centavos exatos
        assert_eq!(Aliquota::try_from(65).expect("bps").aplicar(10_000), 65);
        // 0,65% de R$ 1,00 = 0,65 centavo → arredonda para 1
        assert_eq!(Aliquota::try_from(65).expect("bps").aplicar(100), 1);
        // 0,4 centavo → arredonda para 0
        assert_eq!(Aliquota::try_from(40).expect("bps").aplicar(100), 0);
    }

    #[test]
    fn reduzida_aplica_fator() {
        let cheia = Aliquota::try_from(1800).expect("18%");
        assert_eq!(cheia.reduzida(0).bps(), 1800);
        assert_eq!(cheia.reduzida(6000).bps(), 720); // redução de 60%
        assert_eq!(cheia.reduzida(10_000).bps(), 0); // alíquota zero
    }

    #[test]
    fn faixa_validada() {
        assert!(Aliquota::try_from(-1).is_err());
        assert!(Aliquota::try_from(20_001).is_err());
        assert!(Aliquota::try_from(0).is_ok());
    }

    #[test]
    fn aplicar_base_zero_e_negativa() {
        let a = Aliquota::try_from(1800).expect("18%");
        assert_eq!(a.aplicar(0), 0);
        // Base negativa (estorno/devolução): resultado proporcional negativo,
        // sem estourar nem inverter sinal.
        assert_eq!(a.aplicar(-10_000), -1_799); // -1800 + half-up de +5000/10000
    }

    #[test]
    fn reduzida_clampa_fora_da_faixa() {
        let cheia = Aliquota::try_from(1800).expect("18%");
        // Redução acima de 100% não pode inverter o sinal da alíquota.
        assert_eq!(cheia.reduzida(15_000).bps(), 0);
        // Redução negativa (dado corrompido) não pode aumentar a alíquota.
        assert_eq!(cheia.reduzida(-5_000).bps(), 1800);
    }
}
