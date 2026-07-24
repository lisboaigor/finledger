use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::queries::{ClasseTributariaResult, NotaFiscalResult};
use crate::fiscal::domain::nota_fiscal::{NotaFiscal, NotaFiscalId};
use crate::shared::normalizar_paginacao;

/// Produto ativo com os campos que definem sua tributação (NCM + classe), lido
/// da projeção para a alíquota efetiva. O cálculo em si (motor + provider) é
/// orquestração de aplicação — aqui fica só o acesso ao read model.
#[derive(sqlx::FromRow)]
pub struct ProdutoTributavel {
    pub produto_id: Uuid,
    pub ncm: String,
    pub c_class_trib: Option<String>,
}

pub struct PostgresNotaFiscalRepository {
    inner: TenantScopedRepository<NotaFiscal>,
    pool: Pool,
}

impl PostgresNotaFiscalRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "NotaFiscal"),
            pool,
        }
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub async fn listar(
        &self,
        limite: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<NotaFiscalResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        let (limite, offset) = normalizar_paginacao(limite, offset);
        sqlx::query_as(
            "SELECT nf_id, venda_id, cliente_id, modelo, serie, numero, chave, total_centavos, desconto_centavos, cancelamento_pendente,
                    icms_centavos, pis_centavos, cofins_centavos, iss_centavos,
                    cbs_centavos, ibs_uf_centavos, ibs_mun_centavos, is_centavos,
                    CASE status
                        WHEN 'gerada' THEN 'Gerada'
                        WHEN 'transmitida' THEN 'Transmitida'
                        WHEN 'autorizada' THEN 'Autorizada'
                        WHEN 'rejeitada' THEN 'Rejeitada'
                        WHEN 'cancelada' THEN 'Cancelada'
                        ELSE status
                    END AS status
             FROM proj_notas_fiscais WHERE tenant_id = $1
             ORDER BY gerada_em DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limite)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, nf_id: Uuid) -> Result<Option<NotaFiscalResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT nf_id, venda_id, cliente_id, modelo, serie, numero, chave, total_centavos, desconto_centavos, cancelamento_pendente,
                    icms_centavos, pis_centavos, cofins_centavos, iss_centavos,
                    cbs_centavos, ibs_uf_centavos, ibs_mun_centavos, is_centavos,
                    CASE status
                        WHEN 'gerada' THEN 'Gerada'
                        WHEN 'transmitida' THEN 'Transmitida'
                        WHEN 'autorizada' THEN 'Autorizada'
                        WHEN 'rejeitada' THEN 'Rejeitada'
                        WHEN 'cancelada' THEN 'Cancelada'
                        ELSE status
                    END AS status
             FROM proj_notas_fiscais WHERE nf_id = $1 AND tenant_id = $2",
        )
        .bind(nf_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Classes tributárias de referência (cClassTrib, NT 2025.002) — dado
    /// global, sem tenant. Alimenta o select de classificação do produto.
    pub async fn listar_classes_tributarias(&self) -> Result<Vec<ClasseTributariaResult>, AppError> {
        sqlx::query_as(
            "SELECT c_class_trib, descricao, cst_ibs_cbs, reducao_bps
             FROM ref_classes_tributarias ORDER BY c_class_trib",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    /// Produtos ativos do tenant com NCM e classe tributária — insumo do cálculo
    /// da alíquota efetiva (o motor tributário roda na camada de aplicação).
    pub async fn listar_produtos_tributaveis(&self) -> Result<Vec<ProdutoTributavel>, AppError> {
        sqlx::query_as(
            "SELECT produto_id, ncm, c_class_trib FROM proj_produtos
             WHERE ativo AND tenant_id = $1",
        )
        .bind(current_tenant_id()?)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<NotaFiscal> for PostgresNotaFiscalRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &NotaFiscalId) -> Result<Option<NotaFiscal>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut NotaFiscal) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &NotaFiscalId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
