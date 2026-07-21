//! Repositório de agregados escopado por tenant.
//!
//! Lê o `CURRENT_TENANT` do task-local a cada operação e delega ao
//! `pharos_postgres::TenantJsonRepository`, que aplica isolamento por `tenant_id` na
//! tabela `pharos_tenant_aggregates`. Mantém o grafo de handlers construído uma vez no
//! startup: o tenant é resolvido por chamada, não por construção.

use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

use pharos_core::{AggregateRoot, Entity, Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError, TenantJsonRepository};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::shared::tenant::current_tenant;

pub struct TenantScopedRepository<A: AggregateRoot> {
    pool: Pool,
    aggregate_type: String,
    _marker: PhantomData<fn() -> A>,
}

impl<A: AggregateRoot> TenantScopedRepository<A> {
    pub fn new(pool: Pool, aggregate_type: impl Into<String>) -> Self {
        Self {
            pool,
            aggregate_type: aggregate_type.into(),
            _marker: PhantomData,
        }
    }

    fn scoped(&self) -> Result<TenantJsonRepository<A>, PostgresRepositoryError> {
        let tenant = current_tenant().ok_or_else(|| {
            PostgresRepositoryError::Storage(sqlx::Error::Protocol(
                "tenant context ausente na operação de persistência".into(),
            ))
        })?;
        Ok(TenantJsonRepository::new(
            self.pool.clone(),
            &tenant,
            self.aggregate_type.clone(),
        ))
    }
}

impl<A> Repository<A> for TenantScopedRepository<A>
where
    A: AggregateRoot + Serialize + DeserializeOwned + Send + Sync + 'static,
    <A as Entity>::Id: Display + FromStr + Send + Sync + 'static,
    <<A as Entity>::Id as FromStr>::Err: Display + Send + Sync + 'static,
{
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &A::Id) -> Result<Option<A>, Self::Error> {
        self.scoped()?.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut A) -> Result<(), RepositoryError<Self::Error>> {
        self.scoped()
            .map_err(RepositoryError::Storage)?
            .save(aggregate)
            .await
    }

    async fn delete(&self, id: &A::Id) -> Result<(), Self::Error> {
        self.scoped()?.delete(id).await
    }
}
