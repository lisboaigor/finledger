use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::application::queries::{ContaPagarResult, ContaReceberResult};
use crate::financeiro::domain::conta_pagar::{ContaPagar, ContaPagarId};
use crate::financeiro::domain::conta_receber::{ContaReceber, ContaReceberId};

pub struct PostgresContaReceberRepository {
    inner: TenantScopedRepository<ContaReceber>,
    pool: Pool,
}

impl PostgresContaReceberRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "ContaReceber"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<ContaReceberResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT conta_id, venda_id, cliente_id, valor_original, valor_recebido,
                    CASE status
                        WHEN 'pendente' THEN 'Pendente'
                        WHEN 'parcial' THEN 'Parcial'
                        WHEN 'liquidada' THEN 'Liquidada'
                        WHEN 'estornada' THEN 'Estornada'
                        ELSE status
                    END AS status
             FROM proj_contas_receber WHERE tenant_id = $1 ORDER BY vencimento ASC LIMIT 200",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Contas em aberto (pendente/parcial) de uma venda — usadas para estorno
    /// automático quando a venda é desfeita por devolução total.
    pub async fn contas_abertas_por_venda(&self, venda_id: Uuid) -> Result<Vec<Uuid>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_scalar(
            "SELECT conta_id FROM proj_contas_receber
             WHERE venda_id = $1 AND tenant_id = $2 AND status IN ('pendente', 'parcial')",
        )
        .bind(venda_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, conta_id: Uuid) -> Result<Option<ContaReceberResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT conta_id, venda_id, cliente_id, valor_original, valor_recebido,
                    CASE status
                        WHEN 'pendente' THEN 'Pendente'
                        WHEN 'parcial' THEN 'Parcial'
                        WHEN 'liquidada' THEN 'Liquidada'
                        WHEN 'estornada' THEN 'Estornada'
                        ELSE status
                    END AS status
             FROM proj_contas_receber WHERE conta_id = $1 AND tenant_id = $2",
        )
        .bind(conta_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<ContaReceber> for PostgresContaReceberRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &ContaReceberId) -> Result<Option<ContaReceber>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut ContaReceber) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &ContaReceberId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}

pub struct PostgresContaPagarRepository {
    inner: TenantScopedRepository<ContaPagar>,
    pool: Pool,
}

impl PostgresContaPagarRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "ContaPagar"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<ContaPagarResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT conta_id, pedido_id, fornecedor_id, valor_original, valor_pago,
                    CASE status
                        WHEN 'pendente' THEN 'Pendente'
                        WHEN 'parcial' THEN 'Parcial'
                        WHEN 'liquidada' THEN 'Liquidada'
                        ELSE status
                    END AS status
             FROM proj_contas_pagar WHERE tenant_id = $1 ORDER BY vencimento ASC LIMIT 200",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, conta_id: Uuid) -> Result<Option<ContaPagarResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT conta_id, pedido_id, fornecedor_id, valor_original, valor_pago,
                    CASE status
                        WHEN 'pendente' THEN 'Pendente'
                        WHEN 'parcial' THEN 'Parcial'
                        WHEN 'liquidada' THEN 'Liquidada'
                        ELSE status
                    END AS status
             FROM proj_contas_pagar WHERE conta_id = $1 AND tenant_id = $2",
        )
        .bind(conta_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<ContaPagar> for PostgresContaPagarRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &ContaPagarId) -> Result<Option<ContaPagar>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut ContaPagar) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &ContaPagarId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
