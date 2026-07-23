use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};

use super::events::IdentityEvent;

id_type!(UsuarioId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Usuario {
    #[id]
    id: UsuarioId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<IdentityEvent>,
    username: String,
    password_hash: String,
    roles: Vec<String>,
    ativo: bool,
}

impl Usuario {
    pub fn registrar(
        username: String,
        password_hash: String,
        roles: Vec<String>,
    ) -> DomainResult<Self> {
        let username = username.trim().to_string();
        if username.is_empty() {
            return Err(DomainError::Validation(
                "username não pode ser vazio".into(),
            ));
        }
        if username.chars().any(|c| !c.is_alphanumeric() && c != '_') {
            return Err(DomainError::Validation(
                "username deve conter apenas letras, números e underscore".into(),
            ));
        }

        let id = UsuarioId::new();
        let mut events = AggregateEvents::default();

        events.raise(IdentityEvent::UsuarioCriado {
            usuario_id: id.to_string(),
            username: username.clone(),
            password_hash: password_hash.clone(),
            roles: roles.join(","),
            occurred_at: Utc::now(),
        });

        Ok(Self {
            id,
            version: 0,
            events,
            username,
            password_hash,
            roles,
            ativo: true,
        })
    }

    pub fn alterar_senha(&mut self, nova_hash: String) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "usuário inativo não pode alterar senha".into(),
            ));
        }

        self.password_hash = nova_hash.clone();

        self.events.raise(IdentityEvent::SenhaAlterada {
            usuario_id: self.id.to_string(),
            password_hash: nova_hash,
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn desativar(&mut self) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule("usuário já está inativo".into()));
        }

        self.ativo = false;

        self.events.raise(IdentityEvent::UsuarioDesativado {
            usuario_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn reativar(&mut self) -> DomainResult<()> {
        if self.ativo {
            return Err(DomainError::BusinessRule("usuário já está ativo".into()));
        }

        self.ativo = true;

        self.events.raise(IdentityEvent::UsuarioReativado {
            usuario_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn alterar_roles(&mut self, roles: Vec<String>) -> DomainResult<()> {
        if !self.ativo {
            return Err(DomainError::BusinessRule(
                "usuário inativo não pode ter roles alteradas".into(),
            ));
        }
        if roles.is_empty() {
            return Err(DomainError::Validation(
                "usuário deve ter ao menos uma role".into(),
            ));
        }

        self.events.raise(IdentityEvent::RolesAlteradas {
            usuario_id: self.id.to_string(),
            roles: roles.join(","),
            occurred_at: Utc::now(),
        });
        self.roles = roles;

        Ok(())
    }

    /// Hash da senha (leitura para verificação em `AlterarSenha`).
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

#[cfg(test)]
mod tests {
    use pharos_core::AggregateRoot;

    use super::*;

    fn usuario_valido() -> Usuario {
        Usuario::registrar(
            "carlos_vendedor".into(),
            "$argon2id$hash".into(),
            vec!["vendedor".into()],
        )
        .expect("usuário válido")
    }

    #[test]
    fn registrar_gera_evento_e_usuario_ativo() {
        let u = usuario_valido();
        assert!(u.ativo);
        assert_eq!(u.username, "carlos_vendedor");
        assert!(matches!(
            u.pending_events()[0],
            IdentityEvent::UsuarioCriado { .. }
        ));
    }

    #[test]
    fn registrar_username_vazio_retorna_erro() {
        let r = Usuario::registrar("  ".into(), "hash".into(), vec![]);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn registrar_username_com_caracteres_invalidos_retorna_erro() {
        let r = Usuario::registrar("carlos vendedor!".into(), "hash".into(), vec![]);
        assert!(matches!(r, Err(DomainError::Validation(_))));
    }

    #[test]
    fn alterar_senha_atualiza_hash_e_gera_evento() {
        let mut u = usuario_valido();
        u.alterar_senha("novo_hash".into()).expect("alterar senha");
        assert_eq!(u.password_hash, "novo_hash");
        assert!(matches!(
            u.pending_events().last(),
            Some(IdentityEvent::SenhaAlterada { .. })
        ));
    }

    #[test]
    fn alterar_senha_usuario_inativo_retorna_erro() {
        let mut u = usuario_valido();
        u.desativar().expect("desativar");
        assert!(matches!(
            u.alterar_senha("hash".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn desativar_e_reativar_alternam_estado() {
        let mut u = usuario_valido();
        u.desativar().expect("desativar");
        assert!(!u.ativo);
        u.reativar().expect("reativar");
        assert!(u.ativo);
        assert!(matches!(
            u.pending_events().last(),
            Some(IdentityEvent::UsuarioReativado { .. })
        ));
    }

    #[test]
    fn desativar_usuario_ja_inativo_retorna_erro() {
        let mut u = usuario_valido();
        u.desativar().expect("desativar");
        assert!(matches!(u.desativar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn reativar_usuario_ativo_retorna_erro() {
        let mut u = usuario_valido();
        assert!(matches!(u.reativar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn alterar_roles_atualiza_e_gera_evento() {
        let mut u = usuario_valido();
        u.alterar_roles(vec!["vendedor".into(), "estoquista".into()])
            .expect("alterar roles");
        assert_eq!(u.roles, vec!["vendedor", "estoquista"]);
        assert!(matches!(
            u.pending_events().last(),
            Some(IdentityEvent::RolesAlteradas { .. })
        ));
    }

    #[test]
    fn alterar_roles_para_vazio_retorna_erro() {
        let mut u = usuario_valido();
        assert!(matches!(
            u.alterar_roles(vec![]),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn alterar_roles_usuario_inativo_retorna_erro() {
        let mut u = usuario_valido();
        u.desativar().expect("desativar");
        assert!(matches!(
            u.alterar_roles(vec!["admin".into()]),
            Err(DomainError::BusinessRule(_))
        ));
    }
}
