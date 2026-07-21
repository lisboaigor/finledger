use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::queries::SaldoResult;
use crate::estoque::domain::item_estoque::{ItemEstoque, ItemEstoqueId};

pub struct PostgresEstoqueRepository {
    inner: TenantScopedRepository<ItemEstoque>,
    pool: Pool,
}

impl PostgresEstoqueRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "ItemEstoque"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<SaldoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT produto_id, quantidade, custo_medio, estoque_minimo FROM proj_saldo_estoque WHERE tenant_id = $1 ORDER BY produto_id",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, produto_id: Uuid) -> Result<Option<SaldoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT produto_id, quantidade, custo_medio, estoque_minimo FROM proj_saldo_estoque WHERE produto_id = $1 AND tenant_id = $2",
        )
        .bind(produto_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<ItemEstoque> for PostgresEstoqueRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &ItemEstoqueId) -> Result<Option<ItemEstoque>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut ItemEstoque) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &ItemEstoqueId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
