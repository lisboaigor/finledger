use pharos_core::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::Ncm;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModeloNF {
    NFe,  // modelo 55 — pessoa jurídica
    NFCe, // modelo 65 — consumidor final
}

impl ModeloNF {
    pub fn codigo(&self) -> &'static str {
        match self {
            Self::NFe => "55",
            Self::NFCe => "65",
        }
    }

    pub fn cfop_padrao(&self) -> &'static str {
        // 5102 = venda de mercadoria adquirida de terceiros, mesma UF
        "5102"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusNFe {
    Gerada,
    Transmitida,
    Autorizada,
    Rejeitada,
    Cancelada,
}

/// Cálculo simplificado de impostos por item (alíquotas padrão SP Simples Nacional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpostoItem {
    pub icms_centavos: i64,
    pub pis_centavos: i64,
    pub cofins_centavos: i64,
}

impl ImpostoItem {
    pub fn calcular(valor_total_centavos: i64) -> Self {
        Self {
            icms_centavos: (valor_total_centavos as f64 * 0.18) as i64,
            pis_centavos: (valor_total_centavos as f64 * 0.0065) as i64,
            cofins_centavos: (valor_total_centavos as f64 * 0.03) as i64,
        }
    }
}

// `ncm`/`cfop`/`quantidade`/`valor_unitario_centavos` são privados: só se
// obtém um `ItemNF` via `ItemNF::novo`, que valida cada um — sem struct
// literal de fora deste módulo construindo um item com NCM malformado,
// quantidade zero ou valor negativo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemNF {
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    ncm: String,
    cfop: String,
    quantidade: u32,
    valor_unitario_centavos: i64,
    pub imposto: ImpostoItem,
}

impl ItemNF {
    #[allow(clippy::too_many_arguments)]
    pub fn novo(
        produto_id: Uuid,
        sku: String,
        descricao: String,
        ncm: String,
        cfop: String,
        quantidade: u32,
        valor_unitario_centavos: i64,
        imposto: ImpostoItem,
    ) -> DomainResult<Self> {
        if quantidade == 0 {
            return Err(DomainError::Validation(
                "Quantidade deve ser maior que zero".into(),
            ));
        }
        if valor_unitario_centavos < 0 {
            return Err(DomainError::Validation(
                "Valor unitário não pode ser negativo".into(),
            ));
        }
        if cfop.trim().is_empty() {
            return Err(DomainError::Validation("CFOP não pode ser vazio".into()));
        }
        // Reaproveita a validação de `shared::Ncm` (8 dígitos) em vez de
        // duplicar a checagem aqui.
        let ncm = Ncm::try_from(ncm)?;

        Ok(Self {
            produto_id,
            sku,
            descricao,
            ncm: ncm.into(),
            cfop,
            quantidade,
            valor_unitario_centavos,
            imposto,
        })
    }

    pub fn ncm(&self) -> &str {
        &self.ncm
    }

    pub fn cfop(&self) -> &str {
        &self.cfop
    }

    pub fn quantidade(&self) -> u32 {
        self.quantidade
    }

    pub fn valor_unitario_centavos(&self) -> i64 {
        self.valor_unitario_centavos
    }

    pub fn total_centavos(&self) -> i64 {
        self.valor_unitario_centavos * self.quantidade as i64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotaisNF {
    pub produtos_centavos: i64,
    pub icms_centavos: i64,
    pub pis_centavos: i64,
    pub cofins_centavos: i64,
    pub total_centavos: i64,
}

impl TotaisNF {
    pub fn calcular(itens: &[ItemNF]) -> Self {
        let produtos = itens.iter().map(ItemNF::total_centavos).sum::<i64>();
        let icms = itens.iter().map(|i| i.imposto.icms_centavos).sum::<i64>();
        let pis = itens.iter().map(|i| i.imposto.pis_centavos).sum::<i64>();
        let cofins = itens.iter().map(|i| i.imposto.cofins_centavos).sum::<i64>();
        Self {
            produtos_centavos: produtos,
            icms_centavos: icms,
            pis_centavos: pis,
            cofins_centavos: cofins,
            total_centavos: produtos,
        }
    }
}
