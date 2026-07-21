use pharos_core::{Repository, RepositoryError};
use pharos_postgres::{Pool, PostgresRepositoryError};

use crate::shared::tenant::current_tenant_id;
use crate::shared::tenant_repository::TenantScopedRepository;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::queries::NotaFiscalResult;
use crate::fiscal::domain::nota_fiscal::{NotaFiscal, NotaFiscalId};

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

    pub async fn listar(&self) -> Result<Vec<NotaFiscalResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT nf_id, venda_id, cliente_id, modelo, serie, numero, chave, total_centavos, cancelamento_pendente,
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
             FROM proj_notas_fiscais WHERE tenant_id = $1 ORDER BY gerada_em DESC LIMIT 200",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::infra)
    }

    pub async fn buscar(&self, nf_id: Uuid) -> Result<Option<NotaFiscalResult>, AppError> {
        let tenant_id = current_tenant_id()?;
        sqlx::query_as(
            "SELECT nf_id, venda_id, cliente_id, modelo, serie, numero, chave, total_centavos, cancelamento_pendente,
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
