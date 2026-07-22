use pharos_core::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Regime tributário do tenant (art. 146 CF + LC 123/2006 / LC 214/2025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegimeTributario {
    SimplesNacional,
    LucroPresumido,
    LucroReal,
}

impl RegimeTributario {
    /// Valor persistido em `tenants.regime_tributario` (e chave de lookup em
    /// `ref_aliquotas.regime`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SimplesNacional => "simples_nacional",
            Self::LucroPresumido => "lucro_presumido",
            Self::LucroReal => "lucro_real",
        }
    }
}

impl TryFrom<&str> for RegimeTributario {
    type Error = DomainError;

    fn try_from(s: &str) -> DomainResult<Self> {
        match s {
            "simples_nacional" => Ok(Self::SimplesNacional),
            "lucro_presumido" => Ok(Self::LucroPresumido),
            "lucro_real" => Ok(Self::LucroReal),
            outro => Err(DomainError::Validation(format!(
                "Regime tributário inválido: {outro}"
            ))),
        }
    }
}

/// Sigla de UF: exatamente 2 letras maiúsculas, dentre as 27 válidas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uf(String);

const UFS_VALIDAS: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

impl Uf {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Uf {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let sigla = s.trim().to_uppercase();
        if !UFS_VALIDAS.contains(&sigla.as_str()) {
            return Err(DomainError::Validation(format!("UF inválida: {s}")));
        }
        Ok(Self(sigla))
    }
}

/// Código IBGE do município: exatamente 7 dígitos numéricos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CodigoMunicipio(String);

impl CodigoMunicipio {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CodigoMunicipio {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 7 {
            return Err(DomainError::Validation(
                "Código IBGE do município deve ter exatamente 7 dígitos".into(),
            ));
        }
        Ok(Self(digits))
    }
}

/// Código de Regime Tributário da NF-e (campo CRT): 1 = Simples Nacional,
/// 2 = Simples com sublimite excedido, 3 = Regime Normal, 4 = MEI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Crt(u8);

impl Crt {
    pub fn valor(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Crt {
    type Error = DomainError;

    fn try_from(v: u8) -> DomainResult<Self> {
        if !(1..=4).contains(&v) {
            return Err(DomainError::Validation(format!(
                "CRT deve estar entre 1 e 4 (recebido {v})"
            )));
        }
        Ok(Self(v))
    }
}

/// Perfil fiscal do tenant — determina como o motor tributário calcula os
/// impostos das notas emitidas por ele.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfilFiscal {
    pub regime: RegimeTributario,
    pub uf: Uf,
    pub codigo_municipio: CodigoMunicipio,
    pub crt: Crt,
    /// Simples Nacional pode optar pelo regime regular de IBS/CBS (LC 214/2025,
    /// art. 41) para gerar crédito aos clientes; `false` = recolhe por dentro
    /// do DAS e os valores de IBS/CBS na NF são informativos.
    pub ibs_cbs_regime_regular: bool,
}

impl PerfilFiscal {
    /// Comportamento histórico do sistema para tenants que ainda não
    /// configuraram o perfil: Simples Nacional em São Paulo/SP, CRT 1.
    /// Mantém as notas do cliente em produção idênticas às de antes do motor.
    pub fn padrao_legado() -> Self {
        Self {
            regime: RegimeTributario::SimplesNacional,
            uf: Uf("SP".into()),
            codigo_municipio: CodigoMunicipio("3550308".into()),
            crt: Crt(1),
            ibs_cbs_regime_regular: false,
        }
    }

    /// IBS/CBS destacados são meramente informativos para este perfil? Verdadeiro
    /// no Simples Nacional que NÃO optou pelo regime regular — nesse caso o
    /// IBS/CBS é recolhido por dentro do DAS e não é custo por fora do vendedor
    /// (LC 214/2025, art. 41). Demais regimes recolhem por fora → não informativo.
    pub fn ibs_cbs_informativo(&self) -> bool {
        self.regime == RegimeTributario::SimplesNacional && !self.ibs_cbs_regime_regular
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uf_valida_normaliza_caixa() {
        let uf = Uf::try_from("sp".to_string()).expect("SP é válida");
        assert_eq!(uf.as_str(), "SP");
    }

    #[test]
    fn uf_inexistente_rejeitada() {
        assert!(Uf::try_from("XX".to_string()).is_err());
    }

    #[test]
    fn municipio_exige_7_digitos() {
        assert!(CodigoMunicipio::try_from("3550308".to_string()).is_ok());
        assert!(CodigoMunicipio::try_from("123".to_string()).is_err());
    }

    #[test]
    fn crt_fora_da_faixa_rejeitado() {
        assert!(Crt::try_from(0).is_err());
        assert!(Crt::try_from(5).is_err());
        assert!(Crt::try_from(1).is_ok());
        assert!(Crt::try_from(4).is_ok());
    }

    #[test]
    fn simples_sem_regime_regular_e_informativo() {
        let p = PerfilFiscal::padrao_legado(); // Simples Nacional, regime_regular = false
        assert!(p.ibs_cbs_informativo());
    }

    #[test]
    fn simples_com_regime_regular_nao_e_informativo() {
        let mut p = PerfilFiscal::padrao_legado();
        p.ibs_cbs_regime_regular = true;
        assert!(!p.ibs_cbs_informativo());
    }

    #[test]
    fn lucro_real_nunca_e_informativo() {
        let mut p = PerfilFiscal::padrao_legado();
        p.regime = RegimeTributario::LucroReal;
        assert!(!p.ibs_cbs_informativo());
    }

    #[test]
    fn regime_round_trip() {
        for r in [
            RegimeTributario::SimplesNacional,
            RegimeTributario::LucroPresumido,
            RegimeTributario::LucroReal,
        ] {
            assert_eq!(RegimeTributario::try_from(r.as_str()).expect("round trip"), r);
        }
    }
}
