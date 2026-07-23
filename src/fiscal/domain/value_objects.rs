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

/// Finalidade da NF-e (`finNFe`): normal (1) ou devolução/retorno (4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FinalidadeNFe {
    #[default]
    Normal,
    Devolucao,
}

impl FinalidadeNFe {
    /// Código `finNFe` do layout da NF-e.
    pub fn codigo(&self) -> &'static str {
        match self {
            Self::Normal => "1",
            Self::Devolucao => "4",
        }
    }
}

/// Sentido da operação (`tpNF`): saída (1, venda) ou entrada (0, devolução).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TipoNF {
    #[default]
    Saida,
    Entrada,
}

impl TipoNF {
    /// Código `tpNF` do layout da NF-e.
    pub fn codigo(&self) -> &'static str {
        match self {
            Self::Saida => "1",
            Self::Entrada => "0",
        }
    }
}

/// CSOSN "tributada pelo Simples Nacional sem permissão de crédito" — caso
/// geral do Simples sem substituição tributária.
pub const CSOSN_TRIBUTADA_SEM_ST: &str = "102";
/// CST do ICMS "tributada integralmente" — caso geral de regime normal sem ST.
pub const CST_ICMS_TRIBUTADA_INTEGRAL: &str = "00";

/// Impostos calculados de um item, congelados no evento `NotaFiscalGerada` —
/// replays nunca recalculam. Campos novos da reforma tributária (CBS/IBS/IS)
/// entram com `#[serde(default)]` para eventos gravados antes do motor
/// continuarem deserializando.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImpostoItem {
    pub icms_centavos: i64,
    pub pis_centavos: i64,
    pub cofins_centavos: i64,
    #[serde(default)]
    pub iss_centavos: i64,
    #[serde(default)]
    pub cbs_centavos: i64,
    #[serde(default)]
    pub ibs_uf_centavos: i64,
    #[serde(default)]
    pub ibs_mun_centavos: i64,
    #[serde(default)]
    pub is_centavos: i64,
    /// cClassTrib (NT 2025.002) usado no cálculo — preenche os grupos gIBSCBS
    /// do XML quando a emissão real entrar.
    #[serde(default)]
    pub c_class_trib: Option<String>,
    #[serde(default)]
    pub cst_ibs_cbs: Option<String>,
    /// CSOSN (Simples Nacional): "102" (tributada sem ST) quando o perfil é
    /// Simples — a NF usa CSOSN em vez de CST no grupo ICMS.
    #[serde(default)]
    pub csosn: Option<String>,
    /// CST do ICMS (regimes normais — Lucro Presumido/Real): "00" (tributada
    /// integralmente) no caso geral sem substituição tributária. O layout da
    /// NF-e exige CSOSN (Simples) OU CST (normal) no grupo ICMS.
    #[serde(default)]
    pub cst_icms: Option<String>,
    /// Custo do DAS (alíquota efetiva do Simples × base) — NÃO é destacado na
    /// NF; entra apenas no custo tributário do vendedor (precificação/BI).
    #[serde(default)]
    pub das_centavos: i64,
}

impl ImpostoItem {
    /// Cálculo legado hardcoded (SP Simples Nacional: ICMS 18%, PIS 0,65%,
    /// COFINS 3%) em aritmética inteira half-up. É o que o motor produz para
    /// tenants sem perfil fiscal configurado — preservado como fallback e para
    /// fixtures de teste.
    pub fn calcular_legado_simples(valor_total_centavos: i64) -> Self {
        let bps = |bps: i64| ((valor_total_centavos as i128 * bps as i128 + 5_000) / 10_000) as i64;
        Self {
            icms_centavos: bps(1800),
            pis_centavos: bps(65),
            cofins_centavos: bps(300),
            ..Self::default()
        }
    }

    /// Soma dos tributos que são custo efetivo do vendedor. ICMS/ISS/PIS/COFINS,
    /// o Imposto Seletivo e o DAS (Simples configurado) sempre entram; IBS/CBS
    /// destacados só contam quando NÃO são informativos — no Simples Nacional
    /// sem opção pelo regime regular eles são recolhidos por dentro do DAS
    /// (LC 214/2025, art. 41) e não incrementam o custo por fora. Fonte única
    /// reusada por precificação, BI e projeção (evita duplicar a regra em TS/SQL).
    /// Rateia todos os tributos proporcionalmente (`num`/`den`) — usado na
    /// NF-e de devolução, que espelha os impostos da nota original na razão da
    /// quantidade devolvida (reverte o que foi cobrado; não recalcula por
    /// alíquota). Aritmética inteira half-up. `den == 0` → tudo zero.
    pub fn ratear(&self, num: u32, den: u32) -> Self {
        if den == 0 {
            return Self::default();
        }
        let (num, den) = (num as i128, den as i128);
        let r = |v: i64| ((v as i128 * num + den / 2) / den) as i64;
        Self {
            icms_centavos: r(self.icms_centavos),
            pis_centavos: r(self.pis_centavos),
            cofins_centavos: r(self.cofins_centavos),
            iss_centavos: r(self.iss_centavos),
            cbs_centavos: r(self.cbs_centavos),
            ibs_uf_centavos: r(self.ibs_uf_centavos),
            ibs_mun_centavos: r(self.ibs_mun_centavos),
            is_centavos: r(self.is_centavos),
            das_centavos: r(self.das_centavos),
            c_class_trib: self.c_class_trib.clone(),
            cst_ibs_cbs: self.cst_ibs_cbs.clone(),
            csosn: self.csosn.clone(),
            cst_icms: self.cst_icms.clone(),
        }
    }

    pub fn custo_vendedor_centavos(&self, ibs_cbs_informativo: bool) -> i64 {
        let legado_e_seletivo = self.icms_centavos
            + self.iss_centavos
            + self.pis_centavos
            + self.cofins_centavos
            + self.is_centavos
            + self.das_centavos;
        if ibs_cbs_informativo {
            legado_e_seletivo
        } else {
            legado_e_seletivo + self.cbs_centavos + self.ibs_uf_centavos + self.ibs_mun_centavos
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
    #[serde(default)]
    pub iss_centavos: i64,
    #[serde(default)]
    pub cbs_centavos: i64,
    #[serde(default)]
    pub ibs_uf_centavos: i64,
    #[serde(default)]
    pub ibs_mun_centavos: i64,
    #[serde(default)]
    pub is_centavos: i64,
    /// Desconto global da venda destacado na NF (`#[serde(default)]`: eventos
    /// anteriores ao campo deserializam com zero — total = produtos, como era).
    #[serde(default)]
    pub desconto_centavos: i64,
    /// Total da nota: produtos − desconto.
    pub total_centavos: i64,
}

impl TotaisNF {
    pub fn calcular(itens: &[ItemNF]) -> Self {
        Self::calcular_com_desconto(itens, 0)
    }

    pub fn calcular_com_desconto(itens: &[ItemNF], desconto_centavos: i64) -> Self {
        let produtos = itens.iter().map(ItemNF::total_centavos).sum::<i64>();
        let soma = |f: fn(&ImpostoItem) -> i64| itens.iter().map(|i| f(&i.imposto)).sum::<i64>();
        Self {
            produtos_centavos: produtos,
            icms_centavos: soma(|i| i.icms_centavos),
            pis_centavos: soma(|i| i.pis_centavos),
            cofins_centavos: soma(|i| i.cofins_centavos),
            iss_centavos: soma(|i| i.iss_centavos),
            cbs_centavos: soma(|i| i.cbs_centavos),
            ibs_uf_centavos: soma(|i| i.ibs_uf_centavos),
            ibs_mun_centavos: soma(|i| i.ibs_mun_centavos),
            is_centavos: soma(|i| i.is_centavos),
            desconto_centavos,
            total_centavos: produtos - desconto_centavos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn novo_item(
        ncm: &str,
        cfop: &str,
        quantidade: u32,
        valor_unitario_centavos: i64,
    ) -> DomainResult<ItemNF> {
        ItemNF::novo(
            Uuid::new_v4(),
            "SKU-001".into(),
            "Produto".into(),
            ncm.into(),
            cfop.into(),
            quantidade,
            valor_unitario_centavos,
            ImpostoItem::default(),
        )
    }

    // As invariantes prometidas pelo construtor ("invariante por construção"):
    // cada rejeição precisa de prova, não só do comentário.

    #[test]
    fn item_quantidade_zero_rejeitado() {
        assert!(matches!(
            novo_item("84716053", "5102", 0, 1000),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn item_valor_negativo_rejeitado() {
        assert!(matches!(
            novo_item("84716053", "5102", 1, -1),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn item_cfop_vazio_rejeitado() {
        assert!(matches!(
            novo_item("84716053", "   ", 1, 1000),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn item_ncm_malformado_rejeitado() {
        assert!(matches!(
            novo_item("123", "5102", 1, 1000),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn item_valido_calcula_total() {
        let item = novo_item("84716053", "5102", 3, 2500).expect("item válido");
        assert_eq!(item.total_centavos(), 7500);
        assert_eq!(item.ncm(), "84716053");
    }

    #[test]
    fn totais_de_nf_sem_itens_sao_zero() {
        let totais = TotaisNF::calcular(&[]);
        assert_eq!(totais.produtos_centavos, 0);
        assert_eq!(totais.total_centavos, 0);
        assert_eq!(totais.cbs_centavos, 0);
    }

    fn imposto_pleno() -> ImpostoItem {
        ImpostoItem {
            icms_centavos: 1_000,
            iss_centavos: 100,
            pis_centavos: 65,
            cofins_centavos: 300,
            cbs_centavos: 880,
            ibs_uf_centavos: 700,
            ibs_mun_centavos: 175,
            is_centavos: 250,
            ..ImpostoItem::default()
        }
    }

    #[test]
    fn custo_vendedor_informativo_exclui_ibs_cbs() {
        // Simples informativo: IBS/CBS ficam de fora (recolhidos via DAS).
        let esperado = 1_000 + 100 + 65 + 300 + 250;
        assert_eq!(imposto_pleno().custo_vendedor_centavos(true), esperado);
    }

    #[test]
    fn custo_vendedor_nao_informativo_inclui_ibs_cbs() {
        // Regime regular / não-Simples: IBS/CBS são custo por fora.
        let esperado = 1_000 + 100 + 65 + 300 + 250 + 880 + 700 + 175;
        assert_eq!(imposto_pleno().custo_vendedor_centavos(false), esperado);
    }
}
