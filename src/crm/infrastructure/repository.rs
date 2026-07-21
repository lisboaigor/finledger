use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::crm::application::queries::ClienteResult;
use crate::crm::domain::cliente::{Cliente, ClienteId};
use crate::error::AppError;

pub struct PostgresClienteRepository {
    inner: TenantScopedRepository<Cliente>,
    pool: Pool,
}

impl PostgresClienteRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Cliente"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<ClienteResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT cliente_id, nome, cpf_cnpj, telefone, email, bloqueado, ativo
             FROM proj_clientes WHERE tenant_id = $1 ORDER BY nome",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, cliente_id: Uuid) -> Result<Option<ClienteResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT cliente_id, nome, cpf_cnpj, telefone, email, bloqueado, ativo
             FROM proj_clientes WHERE cliente_id = $1 AND tenant_id = $2",
        )
        .bind(cliente_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<Cliente> for PostgresClienteRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &ClienteId) -> Result<Option<Cliente>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Cliente) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &ClienteId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
