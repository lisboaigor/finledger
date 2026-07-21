use pharos_core::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identificação do cliente de um orçamento.
///
/// Um orçamento pode apontar para um cadastro completo no CRM (`Cadastrado`)
/// ou, para atendimento de balcão, usar apenas um nome informal
/// (`Avulso`) sem exigir CPF/CNPJ — `Cliente::cadastrar` (crm) torna o
/// documento obrigatório, o que inviabiliza cadastro completo para venda
/// avulsa. Os dois casos são mutuamente exclusivos por construção: não há
/// como este tipo representar "cliente_id e nome avulso ao mesmo tempo",
/// eliminando a necessidade de checagem defensiva nas camadas acima.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IdentificacaoCliente {
    Cadastrado(Uuid),
    Avulso(String),
    NaoInformado,
}

impl IdentificacaoCliente {
    /// Limite de tamanho do nome avulso — apenas o suficiente para
    /// identificar o cliente numa lista, não um cadastro.
    const NOME_AVULSO_MAX_CARACTERES: usize = 120;

    /// Único ponto de construção: resolve a exclusividade mútua e valida o
    /// nome avulso. `cliente_id` tem prioridade sobre `nome_avulso` — um
    /// cadastro completo é sempre mais confiável que um texto livre, então
    /// se ambos chegarem informados (ex.: formulário que não limpou o campo
    /// ao selecionar um cliente do CRM) o cadastro prevalece silenciosamente.
    pub fn resolver(cliente_id: Option<Uuid>, nome_avulso: Option<String>) -> DomainResult<Self> {
        if let Some(id) = cliente_id {
            return Ok(Self::Cadastrado(id));
        }
        let nome_normalizado = nome_avulso
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());

        match nome_normalizado {
            None => Ok(Self::NaoInformado),
            Some(nome) if nome.chars().count() > Self::NOME_AVULSO_MAX_CARACTERES => {
                Err(DomainError::BusinessRule(format!(
                    "Nome do cliente avulso não pode ter mais que {} caracteres",
                    Self::NOME_AVULSO_MAX_CARACTERES
                )))
            }
            Some(nome) => Ok(Self::Avulso(nome)),
        }
    }

    pub fn cliente_id(&self) -> Option<Uuid> {
        match self {
            Self::Cadastrado(id) => Some(*id),
            Self::Avulso(_) | Self::NaoInformado => None,
        }
    }

    pub fn nome_avulso(&self) -> Option<&str> {
        match self {
            Self::Avulso(nome) => Some(nome.as_str()),
            Self::Cadastrado(_) | Self::NaoInformado => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cliente_id_gera_cadastrado_ignorando_nome_avulso() {
        let id = Uuid::new_v4();
        let ident =
            IdentificacaoCliente::resolver(Some(id), Some("João de balcão".into())).unwrap();
        assert_eq!(ident, IdentificacaoCliente::Cadastrado(id));
        assert_eq!(ident.cliente_id(), Some(id));
        assert_eq!(ident.nome_avulso(), None);
    }

    #[test]
    fn nome_avulso_e_aparado_e_aceito_sem_cliente_id() {
        let ident = IdentificacaoCliente::resolver(None, Some("  João  ".into())).unwrap();
        assert_eq!(ident, IdentificacaoCliente::Avulso("João".into()));
    }

    #[test]
    fn nome_avulso_vazio_ou_so_espacos_vira_nao_informado() {
        assert_eq!(
            IdentificacaoCliente::resolver(None, Some("   ".into())).unwrap(),
            IdentificacaoCliente::NaoInformado
        );
        assert_eq!(
            IdentificacaoCliente::resolver(None, None).unwrap(),
            IdentificacaoCliente::NaoInformado
        );
    }

    #[test]
    fn nome_avulso_acima_do_limite_e_rejeitado() {
        let nome = "a".repeat(121);
        assert!(matches!(
            IdentificacaoCliente::resolver(None, Some(nome)),
            Err(DomainError::BusinessRule(_))
        ));
    }
}
