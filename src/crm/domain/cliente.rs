use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};

use super::events::CrmEvent;
use crate::shared::{CpfCnpj, Email, NomeNaoVazio, Telefone};

id_type!(ClienteId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Cliente {
    #[id]
    id: ClienteId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<CrmEvent>,
    pub nome: NomeNaoVazio,
    pub cpf_cnpj: CpfCnpj,
    pub telefone: Option<Telefone>,
    pub email: Option<Email>,
    pub ativo: bool,
    pub bloqueado: bool,
}

impl Cliente {
    pub fn cadastrar(
        nome: String,
        cpf_cnpj: String,
        telefone: Option<String>,
        email: Option<String>,
    ) -> DomainResult<Self> {
        let nome = NomeNaoVazio::try_from(nome)?;
        let cpf_cnpj = CpfCnpj::try_from(cpf_cnpj)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;

        let id = ClienteId::new();
        let mut events = AggregateEvents::default();
        events.raise(CrmEvent::ClienteCadastrado {
            cliente_id: id.to_string(),
            nome: nome.to_string(),
            cpf_cnpj: cpf_cnpj.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(Self {
            id,
            version: 0,
            events,
            nome,
            cpf_cnpj,
            telefone,
            email,
            ativo: true,
            bloqueado: false,
        })
    }

    pub fn atualizar(
        &mut self,
        nome: String,
        telefone: Option<String>,
        email: Option<String>,
    ) -> DomainResult<()> {
        let nome = NomeNaoVazio::try_from(nome)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;
        self.events.raise(CrmEvent::ClienteAtualizado {
            cliente_id: self.id.to_string(),
            nome: nome.to_string(),
            telefone: telefone.as_ref().map(|t| t.to_string()),
            email: email.as_ref().map(|e| e.to_string()),
            occurred_at: Utc::now(),
        });
        self.nome = nome;
        self.telefone = telefone;
        self.email = email;
        Ok(())
    }

    pub fn bloquear(&mut self, motivo: String) -> DomainResult<()> {
        if self.bloqueado {
            return Err(DomainError::BusinessRule(
                "Cliente já está bloqueado".into(),
            ));
        }
        self.bloqueado = true;
        self.events.raise(CrmEvent::ClienteBloqueado {
            cliente_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn desbloquear(&mut self) -> DomainResult<()> {
        if !self.bloqueado {
            return Err(DomainError::BusinessRule(
                "Cliente não está bloqueado".into(),
            ));
        }
        self.bloqueado = false;
        self.events.raise(CrmEvent::ClienteDesbloqueado {
            cliente_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn desativar(&mut self) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "Cliente já está desativado".into(),
            ));
        }
        self.ativo = false;
        self.events.raise(CrmEvent::ClienteDesativado {
            cliente_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn reativar(&mut self) -> DomainResult<()> {
        if self.ativo {
            return Err(DomainError::BusinessRule("Cliente já está ativo".into()));
        }
        self.ativo = true;
        self.events.raise(CrmEvent::ClienteReativado {
            cliente_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pharos_core::AggregateRoot;

    use super::*;

    fn cliente_valido() -> Cliente {
        Cliente::cadastrar(
            "João da Silva".into(),
            "123.456.789-09".into(),
            Some("(11) 99999-0000".into()),
            Some("joao@exemplo.com".into()),
        )
        .expect("cliente válido")
    }

    #[test]
    fn cadastrar_gera_evento_e_cliente_ativo_desbloqueado() {
        let c = cliente_valido();
        assert!(c.ativo);
        assert!(!c.bloqueado);
        assert_eq!(c.cpf_cnpj.as_str(), "12345678909");
        assert!(matches!(
            c.pending_events()[0],
            CrmEvent::ClienteCadastrado { .. }
        ));
    }

    #[test]
    fn cadastrar_cpf_cnpj_invalido_retorna_erro() {
        let r = Cliente::cadastrar("João".into(), "123".into(), None, None);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_nome_vazio_retorna_erro() {
        let r = Cliente::cadastrar("   ".into(), "12345678909".into(), None, None);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn atualizar_altera_dados_e_gera_evento() {
        let mut c = cliente_valido();
        c.atualizar("Maria Souza".into(), None, None)
            .expect("atualizar");
        assert_eq!(c.nome.to_string(), "Maria Souza");
        assert!(c.telefone.is_none());
        assert!(matches!(
            c.pending_events().last(),
            Some(CrmEvent::ClienteAtualizado { .. })
        ));
    }

    #[test]
    fn atualizar_email_invalido_retorna_erro() {
        let mut c = cliente_valido();
        assert!(matches!(
            c.atualizar("Maria".into(), None, Some("sem-arroba".into())),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn bloquear_e_desbloquear_alternam_estado() {
        let mut c = cliente_valido();
        c.bloquear("inadimplente".into()).expect("bloquear");
        assert!(c.bloqueado);
        c.desbloquear().expect("desbloquear");
        assert!(!c.bloqueado);
    }

    #[test]
    fn bloquear_cliente_ja_bloqueado_retorna_erro() {
        let mut c = cliente_valido();
        c.bloquear("motivo".into()).expect("bloquear");
        assert!(matches!(
            c.bloquear("de novo".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn desbloquear_cliente_nao_bloqueado_retorna_erro() {
        let mut c = cliente_valido();
        assert!(matches!(c.desbloquear(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn desativar_e_reativar_alternam_estado() {
        let mut c = cliente_valido();
        c.desativar().expect("desativar");
        assert!(!c.ativo);
        assert!(matches!(
            c.pending_events().last(),
            Some(CrmEvent::ClienteDesativado { .. })
        ));
        c.reativar().expect("reativar");
        assert!(c.ativo);
        assert!(matches!(
            c.pending_events().last(),
            Some(CrmEvent::ClienteReativado { .. })
        ));
    }

    #[test]
    fn desativar_cliente_ja_inativo_retorna_erro() {
        let mut c = cliente_valido();
        c.desativar().expect("desativar");
        assert!(matches!(c.desativar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn reativar_cliente_ativo_retorna_erro() {
        let mut c = cliente_valido();
        assert!(matches!(c.reativar(), Err(DomainError::BusinessRule(_))));
    }
}
