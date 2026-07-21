use pharos_core::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Código de classificação tributária (cClassTrib) da NT 2025.002 — identifica
/// o enquadramento do item nos grupos de IBS/CBS do layout da NF-e.
/// Formato: exatamente 6 dígitos numéricos.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClasseTributaria(String);

impl ClasseTributaria {
    /// Classe padrão (tributação integral) usada quando o produto ainda não
    /// foi classificado — mantém o catálogo existente funcionando sem
    /// reclassificação em massa.
    pub fn padrao() -> Self {
        Self("000001".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ClasseTributaria {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 6 {
            return Err(DomainError::Validation(
                "Classe tributária (cClassTrib) deve ter exatamente 6 dígitos".into(),
            ));
        }
        Ok(Self(digits))
    }
}

impl From<ClasseTributaria> for String {
    fn from(c: ClasseTributaria) -> Self {
        c.0
    }
}

impl std::fmt::Display for ClasseTributaria {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Metadados de uma classe tributária, resolvidos de `ref_classes_tributarias`:
/// o CST de IBS/CBS correspondente e a redução de base/alíquota que ela
/// carrega (LC 214/2025 — ex.: 6000 bps = redução de 60%, 10000 = alíquota zero).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClasseTributariaInfo {
    pub classe: ClasseTributaria,
    pub cst_ibs_cbs: String,
    pub reducao_bps: i32,
}

impl ClasseTributariaInfo {
    /// Tributação integral — comportamento para produtos sem classe atribuída.
    pub fn integral() -> Self {
        Self {
            classe: ClasseTributaria::padrao(),
            cst_ibs_cbs: "000".into(),
            reducao_bps: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classe_exige_6_digitos() {
        assert!(ClasseTributaria::try_from("000001".to_string()).is_ok());
        assert!(ClasseTributaria::try_from("123".to_string()).is_err());
        assert!(ClasseTributaria::try_from("1234567".to_string()).is_err());
    }

    #[test]
    fn classe_padrao_e_integral() {
        let info = ClasseTributariaInfo::integral();
        assert_eq!(info.classe.as_str(), "000001");
        assert_eq!(info.reducao_bps, 0);
    }
}
