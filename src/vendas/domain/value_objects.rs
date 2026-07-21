use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormaPagamento {
    Dinheiro,
    CartaoDebito,
    CartaoCredito { parcelas: u8 },
    Pix,
    Prazo { dias: u16 },
}

/// Rótulo humano usado nas projeções de leitura (o que a UI exibe). O enum em
/// si continua sendo serializado como JSON apenas dentro dos eventos.
impl std::fmt::Display for FormaPagamento {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dinheiro => write!(f, "Dinheiro"),
            Self::CartaoDebito => write!(f, "Cartão de débito"),
            Self::CartaoCredito { parcelas } => write!(f, "Cartão de crédito ({parcelas}x)"),
            Self::Pix => write!(f, "Pix"),
            Self::Prazo { dias } => write!(f, "A prazo ({dias} dias)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusVenda {
    EmAndamento,
    Confirmada,
    Cancelada,
}
