use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use serde::Serialize;
use uuid::Uuid;

use crate::catalogo::domain::{Produto, ProdutoId};
use crate::error::AppError;

#[derive(Serialize, sqlx::FromRow)]
pub struct ProdutoResult {
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub ncm: String,
    pub unidade: String,
    pub preco_custo: i64,
    pub preco_venda: i64,
    pub categoria: String,
    pub marca: Option<String>,
    pub ativo: bool,
    pub controla_estoque: bool,
    #[sqlx(default)]
    #[serde(default)]
    pub c_class_trib: Option<String>,
}

pub struct PostgresProdutoRepository {
    inner: TenantScopedRepository<Produto>,
    pool: Pool,
}

impl PostgresProdutoRepository {
    pub fn new(pool: Pool) -> Self {
        Self {
            inner: TenantScopedRepository::new(pool.clone(), "Produto"),
            pool,
        }
    }

    pub async fn listar(&self) -> Result<Vec<ProdutoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT produto_id, sku, descricao, ncm, unidade, preco_custo, preco_venda, categoria, marca, ativo, controla_estoque, c_class_trib
             FROM proj_produtos WHERE tenant_id = $1 ORDER BY descricao",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, produto_id: Uuid) -> Result<Option<ProdutoResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT produto_id, sku, descricao, ncm, unidade, preco_custo, preco_venda, categoria, marca, ativo, controla_estoque, c_class_trib
             FROM proj_produtos WHERE produto_id = $1 AND tenant_id = $2",
        )
        .bind(produto_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)
    }
}

impl Repository<Produto> for PostgresProdutoRepository {
    type Error = PostgresRepositoryError;

    async fn find_by_id(&self, id: &ProdutoId) -> Result<Option<Produto>, Self::Error> {
        self.inner.find_by_id(id).await
    }

    async fn save(&self, aggregate: &mut Produto) -> Result<(), RepositoryError<Self::Error>> {
        self.inner.save(aggregate).await
    }

    async fn delete(&self, id: &ProdutoId) -> Result<(), Self::Error> {
        self.inner.delete(id).await
    }
}
