use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError, TransactionalRepository};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared::normalizar_paginacao;
use crate::vendas::application::queries::{
    VendaArquivadaResult, VendaDetalhes, VendaItemResult, VendaResult,
};
use crate::vendas::domain::venda::{Venda, VendaId};

pub struct PostgresVendaRepository {
    inner: TenantScopedRepository<Venda>,
    pool: Pool,
}

impl PostgresVendaRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Venda"),
            pool,
        }
    }

    pub async fn listar(
        &self,
        produto_busca: Option<String>,
        apenas_abertas: bool,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<VendaResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT venda_id, vendedor_id, cliente_id, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'iniciada' THEN 'Em Andamento'
                        WHEN 'confirmada' THEN 'Confirmada'
                        WHEN 'cancelada' THEN 'Cancelada'
                        ELSE status
                    END AS status,
                    forma_pagamento
             FROM proj_vendas v
             WHERE tenant_id = $1
               AND arquivada_em IS NULL
               AND (NOT $3 OR v.status = 'iniciada')
               AND ($2::text IS NULL OR EXISTS (
                   SELECT 1 FROM proj_vendas_itens vi
                   WHERE vi.tenant_id = v.tenant_id AND vi.venda_id = v.venda_id
                     AND (vi.sku ILIKE '%' || $2 || '%' OR vi.descricao ILIKE '%' || $2 || '%')
               ))
             ORDER BY criada_em DESC LIMIT $4 OFFSET $5",
        )
        .bind(tenant_id)
        .bind(produto_busca)
        .bind(apenas_abertas)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Lixeira: vendas arquivadas pela rotina de limpeza (não excluídas).
    pub async fn listar_lixeira(
        &self,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<VendaArquivadaResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT venda_id, vendedor_id, cliente_id, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'iniciada' THEN 'Em Andamento'
                        WHEN 'confirmada' THEN 'Confirmada'
                        WHEN 'cancelada' THEN 'Cancelada'
                        ELSE status
                    END AS status,
                    forma_pagamento, criada_em, arquivada_em
             FROM proj_vendas
             WHERE tenant_id = $1 AND arquivada_em IS NOT NULL
             ORDER BY arquivada_em DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Restaura uma venda arquivada; `restaurada_em` impede a rotina de
    /// arquivá-la de novo.
    pub async fn restaurar(&self, venda_id: Uuid) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE proj_vendas
             SET arquivada_em = NULL, restaurada_em = NOW()
             WHERE tenant_id = $1 AND venda_id = $2 AND arquivada_em IS NOT NULL",
        )
        .bind(tenant_id)
        .bind(venda_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();
        if n == 0 { Err(AppError::NotFound) } else { Ok(()) }
    }

    pub async fn buscar(&self, venda_id: Uuid) -> Result<Option<VendaDetalhes>, AppError> {
        let tenant_id = current_tenant_id()?;
        let venda: Option<VendaResult> = sqlx::query_as(
            "SELECT venda_id, vendedor_id, cliente_id, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'iniciada' THEN 'Em Andamento'
                        WHEN 'confirmada' THEN 'Confirmada'
                        WHEN 'cancelada' THEN 'Cancelada'
                        ELSE status
                    END AS status,
                    forma_pagamento
             FROM proj_vendas WHERE venda_id = $1 AND tenant_id = $2",
        )
        .bind(venda_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;

        let Some(venda) = venda else {
            return Ok(None);
        };

        let itens: Vec<VendaItemResult> = sqlx::query_as(
            "SELECT item_id, produto_id, sku, descricao, quantidade, preco_unitario_centavos
             FROM proj_vendas_itens WHERE venda_id = $1 AND tenant_id = $2",
        )
        .bind(venda_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)?;

        Ok(Some(VendaDetalhes { venda, itens }))
    }
}

impl Repository<Venda> for PostgresVendaRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &VendaId) -> Result<Option<Venda>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Venda) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &VendaId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}

/// Save durável (snapshot + outbox na mesma transação) — venda é contexto
/// produtor: seus eventos disparam CR/NF/baixa de estoque via relay (issue #3).
impl TransactionalRepository<Venda> for PostgresVendaRepository {
    type Error = PostgresRepositoryError;

    async fn save_in_tx(
        &self,
        conn: &mut sqlx::PgConnection,
        aggregate: &mut Venda,
    ) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save_in_tx(conn, aggregate).await
    }
}
