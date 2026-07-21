use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::identity::application::queries::UsuarioResult;
use crate::identity::domain::user::{Usuario, UsuarioId};

pub struct PostgresIdentityRepository {
    inner: TenantScopedRepository<Usuario>,
    pool: Pool,
}

impl PostgresIdentityRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Usuarios"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<UsuarioResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT usuario_id, username, roles, ativo FROM proj_usuarios WHERE tenant_id = $1 ORDER BY username",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, usuario_id: Uuid) -> Result<Option<UsuarioResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT usuario_id, username, roles, ativo FROM proj_usuarios WHERE usuario_id = $1 AND tenant_id = $2",
        )
        .bind(usuario_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar_para_login(
        &self,
        username: &str,
    ) -> Result<Option<(Uuid, String, String, bool)>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT usuario_id, password_hash, roles, ativo FROM proj_usuarios WHERE username = $1 AND tenant_id = $2",
        )
        .bind(username)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn username_existe(&self, username: &str) -> Result<bool, AppError> {
        let tenant_id = current_tenant_id()?;
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT usuario_id FROM proj_usuarios WHERE username = $1 AND tenant_id = $2",
        )
        .bind(username)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;
        Ok(exists.is_some())
    }
}

impl Repository<Usuario> for PostgresIdentityRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &UsuarioId) -> Result<Option<Usuario>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Usuario) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &UsuarioId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
