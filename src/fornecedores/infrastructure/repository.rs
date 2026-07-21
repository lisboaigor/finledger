use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::fornecedores::application::queries::FornecedorResult;
use crate::fornecedores::domain::fornecedor::{Fornecedor, FornecedorId};

pub struct PostgresFornecedorRepository {
    inner: TenantScopedRepository<Fornecedor>,
    pool: Pool,
}

impl PostgresFornecedorRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Fornecedor"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<FornecedorResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT fornecedor_id, razao_social, cnpj, telefone, email, prazo_pagamento_dias, ativo
             FROM proj_fornecedores WHERE tenant_id = $1 ORDER BY razao_social",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, fornecedor_id: Uuid) -> Result<Option<FornecedorResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT fornecedor_id, razao_social, cnpj, telefone, email, prazo_pagamento_dias, ativo
             FROM proj_fornecedores WHERE fornecedor_id = $1 AND tenant_id = $2",
        )
        .bind(fornecedor_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<Fornecedor> for PostgresFornecedorRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &FornecedorId) -> Result<Option<Fornecedor>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Fornecedor) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &FornecedorId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
