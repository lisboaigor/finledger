use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::compras::application::queries::{
    PedidoCompraDetalhes, PedidoCompraItemResult, PedidoCompraResult,
};
use crate::compras::domain::pedido_compra::{PedidoCompra, PedidoCompraId};
use crate::error::AppError;
use crate::shared::normalizar_paginacao;

pub struct PostgresPedidoCompraRepository {
    inner: TenantScopedRepository<PedidoCompra>,
    pool: Pool,
}

impl PostgresPedidoCompraRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "PedidoCompra"),
            pool,
        }
    }

    pub async fn listar(
        &self,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<PedidoCompraResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT pedido_id, comprador_id, fornecedor_id, total_centavos, prazo_pagamento_dias,
                    CASE status
                        WHEN 'gerado' THEN 'Gerado'
                        WHEN 'aprovado' THEN 'Aprovado'
                        WHEN 'enviado' THEN 'Enviado'
                        WHEN 'recebido_parcial' THEN 'RecebidoParcial'
                        WHEN 'recebido_total' THEN 'RecebidoTotal'
                        WHEN 'cancelado' THEN 'Cancelado'
                        ELSE status
                    END AS status
             FROM proj_pedidos_compra WHERE tenant_id = $1
             ORDER BY criado_em DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, pedido_id: Uuid) -> Result<Option<PedidoCompraDetalhes>, AppError> {
        let tenant_id = current_tenant_id()?;
        let pedido: Option<PedidoCompraResult> = sqlx::query_as(
            "SELECT pedido_id, comprador_id, fornecedor_id, total_centavos, prazo_pagamento_dias,
                    CASE status
                        WHEN 'gerado' THEN 'Gerado'
                        WHEN 'aprovado' THEN 'Aprovado'
                        WHEN 'enviado' THEN 'Enviado'
                        WHEN 'recebido_parcial' THEN 'RecebidoParcial'
                        WHEN 'recebido_total' THEN 'RecebidoTotal'
                        WHEN 'cancelado' THEN 'Cancelado'
                        ELSE status
                    END AS status
             FROM proj_pedidos_compra WHERE pedido_id = $1 AND tenant_id = $2",
        )
        .bind(pedido_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;

        let Some(pedido) = pedido else {
            return Ok(None);
        };

        let itens: Vec<PedidoCompraItemResult> = sqlx::query_as(
            "SELECT produto_id, quantidade, custo_unitario_centavos
             FROM proj_pedidos_compra_itens WHERE pedido_id = $1 AND tenant_id = $2",
        )
        .bind(pedido_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)?;

        Ok(Some(PedidoCompraDetalhes { pedido, itens }))
    }
}

impl Repository<PedidoCompra> for PostgresPedidoCompraRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &PedidoCompraId) -> Result<Option<PedidoCompra>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut PedidoCompra) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &PedidoCompraId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
