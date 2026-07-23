//! Resolução do CFOP (Código Fiscal de Operações e Prestações) — puro, sem I/O.
//!
//! O CFOP identifica a natureza da operação na NF-e. Varia por:
//! - **sentido**: venda (saída, 5xxx/6xxx) vs. devolução (entrada, 1xxx/2xxx);
//! - **destino**: intraestadual (5xxx/1xxx) vs. interestadual (6xxx/2xxx),
//!   comparando a UF do emitente com a do destinatário;
//! - **substituição tributária**: mercadoria sujeita a ICMS-ST usa CFOP próprio.
//!
//! Cobre venda e devolução de mercadoria adquirida de terceiros (o caso geral
//! de um revendedor). O cálculo do valor do ICMS-ST (MVA/base) é deferido — ver
//! issue #16; aqui só se escolhe o CFOP.

use super::value_objects::ModeloNF;

// CFOPs de mercadoria adquirida de terceiros (revenda). Nomeados para não
// espalhar códigos "mágicos" pelo motor/handler.
/// Venda dentro do estado.
pub const CFOP_VENDA_INTERNA: &str = "5102";
/// Venda dentro do estado, com ICMS-ST (substituído).
pub const CFOP_VENDA_INTERNA_ST: &str = "5405";
/// Venda interestadual.
pub const CFOP_VENDA_INTERESTADUAL: &str = "6102";
/// Venda interestadual, com ICMS-ST.
pub const CFOP_VENDA_INTERESTADUAL_ST: &str = "6404";
/// Devolução (entrada) dentro do estado.
pub const CFOP_DEVOLUCAO_INTERNA: &str = "1202";
/// Devolução (entrada) interestadual.
pub const CFOP_DEVOLUCAO_INTERESTADUAL: &str = "2202";

/// Sentido da operação para fins de CFOP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoOperacao {
    Venda,
    Devolucao,
}

/// Interestadual = há UF de destino e ela difere da do emitente. Sem UF de
/// destino (consumidor final sem endereço, típico de NFCe) → operação interna.
fn e_interestadual(uf_emitente: &str, uf_destinatario: Option<&str>) -> bool {
    uf_destinatario
        .map(|d| !d.eq_ignore_ascii_case(uf_emitente))
        .unwrap_or(false)
}

/// Resolve o CFOP da operação.
pub fn resolver_cfop(
    op: TipoOperacao,
    uf_emitente: &str,
    uf_destinatario: Option<&str>,
    _modelo: &ModeloNF,
    tem_st: bool,
) -> &'static str {
    let inter = e_interestadual(uf_emitente, uf_destinatario);
    match op {
        TipoOperacao::Venda => match (inter, tem_st) {
            (false, false) => CFOP_VENDA_INTERNA,
            (false, true) => CFOP_VENDA_INTERNA_ST,
            (true, false) => CFOP_VENDA_INTERESTADUAL,
            (true, true) => CFOP_VENDA_INTERESTADUAL_ST,
        },
        TipoOperacao::Devolucao => cfop_devolucao(inter),
    }
}

/// CFOP de devolução (entrada) conforme o sentido da operação.
pub fn cfop_devolucao(interestadual: bool) -> &'static str {
    if interestadual {
        CFOP_DEVOLUCAO_INTERESTADUAL
    } else {
        CFOP_DEVOLUCAO_INTERNA
    }
}

/// Um CFOP de saída interestadual começa por '6' (5xxx = interno). Usado para
/// derivar o CFOP de devolução a partir do CFOP da nota original.
pub fn cfop_saida_e_interestadual(cfop_saida: &str) -> bool {
    cfop_saida.starts_with('6')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venda_intra_e_interestadual() {
        assert_eq!(
            resolver_cfop(TipoOperacao::Venda, "SP", Some("SP"), &ModeloNF::NFe, false),
            "5102"
        );
        assert_eq!(
            resolver_cfop(TipoOperacao::Venda, "SP", Some("RJ"), &ModeloNF::NFe, false),
            "6102"
        );
    }

    #[test]
    fn consumidor_final_sem_uf_e_interno() {
        assert_eq!(
            resolver_cfop(TipoOperacao::Venda, "SP", None, &ModeloNF::NFCe, false),
            "5102"
        );
    }

    #[test]
    fn venda_com_st() {
        assert_eq!(
            resolver_cfop(TipoOperacao::Venda, "SP", Some("SP"), &ModeloNF::NFe, true),
            "5405"
        );
        assert_eq!(
            resolver_cfop(TipoOperacao::Venda, "SP", Some("MG"), &ModeloNF::NFe, true),
            "6404"
        );
    }

    #[test]
    fn devolucao_intra_e_interestadual() {
        assert_eq!(
            resolver_cfop(TipoOperacao::Devolucao, "SP", Some("SP"), &ModeloNF::NFe, false),
            "1202"
        );
        assert_eq!(
            resolver_cfop(TipoOperacao::Devolucao, "SP", Some("PR"), &ModeloNF::NFe, false),
            "2202"
        );
    }
}
