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

/// Sentido da operação para fins de CFOP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoOperacao {
    Venda,
    Devolucao,
}

/// Resolve o CFOP da operação. `uf_destinatario = None` (consumidor final sem
/// endereço, típico de NFCe) é tratado como operação **interna**.
pub fn resolver_cfop(
    op: TipoOperacao,
    uf_emitente: &str,
    uf_destinatario: Option<&str>,
    _modelo: &ModeloNF,
    tem_st: bool,
) -> &'static str {
    let interestadual = uf_destinatario
        .map(|d| !d.eq_ignore_ascii_case(uf_emitente))
        .unwrap_or(false);
    match op {
        TipoOperacao::Venda => match (interestadual, tem_st) {
            (false, false) => "5102", // venda de merc. de terceiros, mesma UF
            (false, true) => "5405",  // idem, com ICMS-ST (substituído)
            (true, false) => "6102",  // venda interestadual
            (true, true) => "6404",   // venda interestadual com ICMS-ST
        },
        // Devolução de venda: entrada referenciando a NF original.
        TipoOperacao::Devolucao => {
            if interestadual {
                "2202"
            } else {
                "1202"
            }
        }
    }
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
