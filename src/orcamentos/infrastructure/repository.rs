use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError, TransactionalRepository};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared::normalizar_paginacao;
use crate::orcamentos::application::queries::{
    OrcamentoArquivadoResult, OrcamentoDetalhes, OrcamentoItemResult, OrcamentoResult,
};
use crate::orcamentos::domain::orcamento::{Orcamento, OrcamentoId};

pub struct PostgresOrcamentoRepository {
    inner: TenantScopedRepository<Orcamento>,
    pool: Pool,
}

impl PostgresOrcamentoRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Orcamento"),
            pool,
        }
    }

    pub async fn listar(
        &self,
        apenas_abertos: bool,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<OrcamentoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT orcamento_id, vendedor_id, cliente_id, cliente_avulso, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'rascunho' THEN 'Rascunho'
                        WHEN 'emitido' THEN 'Emitido'
                        WHEN 'aceito' THEN 'Aceito'
                        WHEN 'recusado' THEN 'Recusado'
                        WHEN 'expirado' THEN 'Expirado'
                        WHEN 'convertido' THEN 'ConvertidoEmVenda'
                        WHEN 'cancelado' THEN 'Cancelado'
                        ELSE status
                    END AS status,
                    validade_dias
             FROM proj_orcamentos
             WHERE tenant_id = $1 AND arquivado_em IS NULL
               AND (NOT $2 OR status IN ('rascunho', 'emitido'))
             ORDER BY criado_em DESC LIMIT $3 OFFSET $4",
        )
        .bind(tenant_id)
        .bind(apenas_abertos)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Lixeira: orçamentos arquivados pela rotina de limpeza (não excluídos).
    pub async fn listar_lixeira(
        &self,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<OrcamentoArquivadoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT orcamento_id, vendedor_id, cliente_id, cliente_avulso, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'rascunho' THEN 'Rascunho'
                        WHEN 'emitido' THEN 'Emitido'
                        WHEN 'aceito' THEN 'Aceito'
                        WHEN 'recusado' THEN 'Recusado'
                        WHEN 'expirado' THEN 'Expirado'
                        WHEN 'convertido' THEN 'ConvertidoEmVenda'
                        WHEN 'cancelado' THEN 'Cancelado'
                        ELSE status
                    END AS status,
                    validade_dias, criado_em, arquivado_em
             FROM proj_orcamentos
             WHERE tenant_id = $1 AND arquivado_em IS NOT NULL
             ORDER BY arquivado_em DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Restaura um orçamento arquivado; `restaurado_em` impede a rotina de
    /// arquivá-lo de novo.
    pub async fn restaurar(&self, orcamento_id: Uuid) -> Result<(), AppError> {
        let tenant_id = current_tenant_id()?;
        let n = sqlx::query(
            "UPDATE proj_orcamentos
             SET arquivado_em = NULL, restaurado_em = NOW()
             WHERE tenant_id = $1 AND orcamento_id = $2 AND arquivado_em IS NOT NULL",
        )
        .bind(tenant_id)
        .bind(orcamento_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::infra)?
        .rows_affected();
        if n == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn buscar(&self, orcamento_id: Uuid) -> Result<Option<OrcamentoDetalhes>, AppError> {
        let tenant_id = current_tenant_id()?;
        let orcamento: Option<OrcamentoResult> = sqlx::query_as(
            "SELECT orcamento_id, vendedor_id, cliente_id, cliente_avulso, total_centavos, desconto_centavos,
                    CASE status
                        WHEN 'rascunho' THEN 'Rascunho'
                        WHEN 'emitido' THEN 'Emitido'
                        WHEN 'aceito' THEN 'Aceito'
                        WHEN 'recusado' THEN 'Recusado'
                        WHEN 'expirado' THEN 'Expirado'
                        WHEN 'convertido' THEN 'ConvertidoEmVenda'
                        WHEN 'cancelado' THEN 'Cancelado'
                        ELSE status
                    END AS status,
                    validade_dias
             FROM proj_orcamentos WHERE orcamento_id = $1 AND tenant_id = $2",
        )
        .bind(orcamento_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;

        let Some(orcamento) = orcamento else {
            return Ok(None);
        };

        let itens: Vec<OrcamentoItemResult> = sqlx::query_as(
            "SELECT item_id, produto_id, sku, descricao, quantidade, preco_unitario_centavos
             FROM proj_orcamentos_itens WHERE orcamento_id = $1 AND tenant_id = $2",
        )
        .bind(orcamento_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)?;

        Ok(Some(OrcamentoDetalhes { orcamento, itens }))
    }
}

impl Repository<Orcamento> for PostgresOrcamentoRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &OrcamentoId) -> Result<Option<Orcamento>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Orcamento) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &OrcamentoId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}

/// Save durável (snapshot + outbox atômicos) — orçamento é contexto produtor:
/// aceitar um orçamento gera uma venda via relay (issue #3).
impl TransactionalRepository<Orcamento> for PostgresOrcamentoRepository {
    type Error = PostgresRepositoryError;

    async fn save_in_tx(
        &self,
        conn: &mut sqlx::PgConnection,
        aggregate: &mut Orcamento,
    ) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save_in_tx(conn, aggregate).await
    }
}
