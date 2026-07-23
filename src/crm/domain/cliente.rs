use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};

use super::events::CrmEvent;
use crate::shared::{CpfCnpj, Email, NomeNaoVazio, Telefone};

id_type!(ClienteId);

const UFS: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

/// UF do destinatário (opcional): usada pelo fiscal para o CFOP (operação
/// intra/interestadual). Normaliza para maiúsculas e valida contra as 27 UFs.
fn normalizar_uf(uf: Option<String>) -> DomainResult<Option<String>> {
    match uf {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => {
            let up = s.trim().to_uppercase();
            if UFS.contains(&up.as_str()) {
                Ok(Some(up))
            } else {
                Err(DomainError::Validation(format!("UF inválida: {s}")))
            }
        }
    }
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Cliente {
    #[id]
    id: ClienteId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<CrmEvent>,
    nome: NomeNaoVazio,
    cpf_cnpj: CpfCnpj,
    telefone: Option<Telefone>,
    email: Option<Email>,
    #[serde(default)]
    uf: Option<String>,
    ativo: bool,
    bloqueado: bool,
}

impl Cliente {
    pub fn cadastrar(
        nome: String,
        cpf_cnpj: String,
        telefone: Option<String>,
        email: Option<String>,
        uf: Option<String>,
    ) -> DomainResult<Self> {
        let nome = NomeNaoVazio::try_from(nome)?;
        let cpf_cnpj = CpfCnpj::try_from(cpf_cnpj)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;
        let uf = normalizar_uf(uf)?;

        let id = ClienteId::new();
        let mut events = AggregateEvents::default();
        events.raise(CrmEvent::ClienteCadastrado {
            cliente_id: id.to_string(),
            nome: nome.to_string(),
            cpf_cnpj: cpf_cnpj.to_string(),
            uf: uf.clone(),
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
            uf,
            ativo: true,
            bloqueado: false,
        })
    }

    pub fn atualizar(
        &mut self,
        nome: String,
        telefone: Option<String>,
        email: Option<String>,
        uf: Option<String>,
    ) -> DomainResult<()> {
        let nome = NomeNaoVazio::try_from(nome)?;
        let telefone = telefone.map(Telefone::try_from).transpose()?;
        let email = email.map(Email::try_from).transpose()?;
        let uf = normalizar_uf(uf)?;
        self.events.raise(CrmEvent::ClienteAtualizado {
            cliente_id: self.id.to_string(),
            nome: nome.to_string(),
            telefone: telefone.as_ref().map(|t| t.to_string()),
            email: email.as_ref().map(|e| e.to_string()),
            uf: uf.clone(),
            occurred_at: Utc::now(),
        });
        self.nome = nome;
        self.telefone = telefone;
        self.email = email;
        self.uf = uf;
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

    // Getters (leitura).

    pub fn nome(&self) -> &NomeNaoVazio {
        &self.nome
    }

    pub fn ativo(&self) -> bool {
        self.ativo
    }

    pub fn bloqueado(&self) -> bool {
        self.bloqueado
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
            Some("sp".into()),
        )
        .expect("cliente válido")
    }

    #[test]
    fn uf_e_normalizada_e_validada() {
        let c = cliente_valido();
        assert_eq!(c.uf.as_deref(), Some("SP"), "normaliza para maiúsculas");

        let r = Cliente::cadastrar(
            "Ana".into(),
            "12345678909".into(),
            None,
            None,
            Some("XX".into()),
        );
        assert!(matches!(r, Err(DomainError::Validation(_))), "UF inexistente");

        let sem_uf =
            Cliente::cadastrar("Ana".into(), "12345678909".into(), None, None, None).expect("ok");
        assert!(sem_uf.uf.is_none());
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
        let r = Cliente::cadastrar("João".into(), "123".into(), None, None, None);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cadastrar_nome_vazio_retorna_erro() {
        let r = Cliente::cadastrar("   ".into(), "12345678909".into(), None, None, None);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn atualizar_altera_dados_e_gera_evento() {
        let mut c = cliente_valido();
        c.atualizar("Maria Souza".into(), None, None, None)
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
            c.atualizar("Maria".into(), None, Some("sem-arroba".into()), None),
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
