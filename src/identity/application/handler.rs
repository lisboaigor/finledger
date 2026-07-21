use std::sync::Arc;

use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use pharos_app::EventBus;

use crate::auth::AuthConfig;
use crate::error::AppError;
use crate::identity::domain::user::{Usuario, UsuarioId};
use crate::identity::infrastructure::repository::PostgresIdentityRepository;
use crate::shared::{load_aggregate, salvar_aggregate};
use crate::tenants::repository::TenantRepository;

pub struct IdentityHandlers {
    pub(crate) repo: Arc<PostgresIdentityRepository>,
    pub(crate) tenants: Arc<TenantRepository>,
    pub(crate) bus: EventBus,
    pub(crate) auth: Arc<AuthConfig>,
}

impl IdentityHandlers {
    pub fn new(
        repo: Arc<PostgresIdentityRepository>,
        tenants: Arc<TenantRepository>,
        bus: EventBus,
        auth: Arc<AuthConfig>,
    ) -> Self {
        Self {
            repo,
            tenants,
            bus,
            auth,
        }
    }

    pub(crate) async fn carregar(&self, id: UsuarioId) -> Result<Usuario, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, usuario: &mut Usuario) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, usuario).await
    }
}

pub(crate) fn hash_password(senha: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(senha.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::infra(e.to_string()))
}

pub(crate) fn verify_password(senha: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(senha.as_bytes(), &parsed)
        .is_ok()
}
