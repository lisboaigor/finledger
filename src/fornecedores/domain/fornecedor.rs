use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};

use super::events::FornecedorEvent;
use crate::shared::{Cnpj, Email, NomeNaoVazio, Telefone};

id_type!(FornecedorId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Fornecedor {
    #[id]
    id: FornecedorId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<FornecedorEvent>,
    pub razao_social: NomeNaoVazio,
    pub cnpj: Cnpj,
    pub telefone: Option<Telefone>,
    pub email: Option<Email>,
    pub prazo_pagamento_dias: u16,
    pub ativo: bool,
}

impl Fornecedor {
    pub fn cadastrar(
        razao_social: String,
        cnpj: String,
        telefone: Option<String>,
        email: Option<String>,
        prazo_pagamento_dias: u16,
    ) -> DomainResult<Self> {
        let razao_social = NomeNaoVazio::try_from(razao_social)?;
        let cnpj = Cnpj::try_from(cnpj)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;

        let id = FornecedorId::new();
        let mut events = AggregateEvents::default();
        events.raise(FornecedorEvent::FornecedorCadastrado {
            fornecedor_id: id.to_string(),
            razao_social: razao_social.to_string(),
            cnpj: cnpj.to_string(),
            telefone: telefone.as_ref().map(|t| t.to_string()),
            email: email.as_ref().map(|e| e.to_string()),
            prazo_pagamento_dias,
            occurred_at: Utc::now(),
        });

        Ok(Self {
            id,
            version: 0,
            events,
            razao_social,
            cnpj,
            telefone,
            email,
            prazo_pagamento_dias,
            ativo: true,
        })
    }

    pub fn atualizar(
        &mut self,
        razao_social: String,
        telefone: Option<String>,
        email: Option<String>,
        prazo_pagamento_dias: u16,
    ) -> DomainResult<()> {
        let razao_social = NomeNaoVazio::try_from(razao_social)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;

        self.events.raise(FornecedorEvent::FornecedorAtualizado {
            fornecedor_id: self.id.to_string(),
            razao_social: razao_social.to_string(),
            telefone: telefone.as_ref().map(|t| t.to_string()),
            email: email.as_ref().map(|e| e.to_string()),
            prazo_pagamento_dias,
            occurred_at: Utc::now(),
        });
        self.razao_social = razao_social;
        self.telefone = telefone;
        self.email = email;
        self.prazo_pagamento_dias = prazo_pagamento_dias;

        Ok(())
    }

    pub fn desativar(&mut self) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "Fornecedor já está inativo".into(),
            ));
        }

        self.ativo = false;
        self.events.raise(FornecedorEvent::FornecedorDesativado {
            fornecedor_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn reativar(&mut self) -> DomainResult<()> {
        if self.ativo {
            return Err(DomainError::BusinessRule("Fornecedor já está ativo".into()));
        }

        self.ativo = true;
        self.events.raise(FornecedorEvent::FornecedorReativado {
            fornecedor_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pharos_core::AggregateRoot;

    use super::*;

    fn fornecedor_valido() -> Fornecedor {
        Fornecedor::cadastrar(
            "Distribuidora Brasil Ltda".into(),
            "12.345.678/0001-95".into(),
            Some("(11) 4000-1234".into()),
            Some("contato@distribuidorabrasil.com".into()),
            28,
        )
        .expect("fornecedor válido")
    }

    #[test]
    fn cadastrar_gera_evento_e_fornecedor_ativo() {
        let f = fornecedor_valido();
        assert!(f.ativo);
        assert_eq!(f.cnpj.as_str(), "12345678000195");
        assert_eq!(f.prazo_pagamento_dias, 28);
        assert!(matches!(
            f.pending_events()[0],
            FornecedorEvent::FornecedorCadastrado { .. }
        ));
    }

    #[test]
    fn cadastrar_cnpj_invalido_retorna_erro() {
        let r = Fornecedor::cadastrar("Fornecedor".into(), "123".into(), None, None, 0);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_razao_social_vazia_retorna_erro() {
        let r = Fornecedor::cadastrar("  ".into(), "12345678000195".into(), None, None, 0);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn atualizar_altera_dados_e_gera_evento() {
        let mut f = fornecedor_valido();
        f.atualizar("Nova Razão SA".into(), None, None, 45)
            .expect("atualizar");
        assert_eq!(f.razao_social.to_string(), "Nova Razão SA");
        assert_eq!(f.prazo_pagamento_dias, 45);
        assert!(f.telefone.is_none());
        assert!(matches!(
            f.pending_events().last(),
            Some(FornecedorEvent::FornecedorAtualizado { .. })
        ));
    }

    #[test]
    fn atualizar_email_invalido_retorna_erro() {
        let mut f = fornecedor_valido();
        assert!(matches!(
            f.atualizar("Razão".into(), None, Some("invalido".into()), 10),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn desativar_e_reativar_alternam_estado() {
        let mut f = fornecedor_valido();
        f.desativar().expect("desativar");
        assert!(!f.ativo);
        f.reativar().expect("reativar");
        assert!(f.ativo);
    }

    #[test]
    fn desativar_fornecedor_ja_inativo_retorna_erro() {
        let mut f = fornecedor_valido();
        f.desativar().expect("desativar");
        assert!(matches!(f.desativar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn reativar_fornecedor_ativo_retorna_erro() {
        let mut f = fornecedor_valido();
        assert!(matches!(f.reativar(), Err(DomainError::BusinessRule(_))));
    }
}
