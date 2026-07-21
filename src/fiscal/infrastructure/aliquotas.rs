use chrono::NaiveDate;
use pharos_postgres::Pool;
use sqlx::Row;

use crate::error::AppError;
use crate::fiscal::domain::tributacao::{
    Aliquota, AliquotasItem, ClasseTributaria, ClasseTributariaInfo, PerfilFiscal,
};
use crate::shared::tenant::current_tenant_id;

/// Port de resolução de alíquotas: dado o contexto (data de emissão, perfil
/// fiscal, classe/NCM do item), devolve o snapshot de alíquotas que o
/// `MotorTributario` (puro) aplica. Mesmo estilo de trait do `SefazClient`
/// (`impl Future`, não object-safe — handlers são genéricos sobre ele).
pub trait AliquotaProvider: Send + Sync + 'static {
    fn resolver(
        &self,
        data: NaiveDate,
        perfil: &PerfilFiscal,
        classe: &ClasseTributaria,
        ncm: &str,
    ) -> impl Future<Output = Result<AliquotasItem, AppError>> + Send;

    /// Metadados da classe tributária do produto; `None`/desconhecida →
    /// tributação integral (classe padrão).
    fn classe_info(
        &self,
        classe: Option<&ClasseTributaria>,
    ) -> impl Future<Output = Result<ClasseTributariaInfo, AppError>> + Send;
}

/// Resolve contra `aliquotas_tenant` (override, RLS por tenant) e
/// `ref_aliquotas` (referência global): para cada tributo vence a linha
/// vigente de MAIOR especificidade, com override de tenant prevalecendo
/// sobre a referência em qualquer especificidade.
pub struct PostgresAliquotaProvider {
    pool: Pool,
}

impl PostgresAliquotaProvider {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Colunas-curinga casam com NULL ("qualquer") ou com o valor do contexto;
    /// `especificidade` conta quantas chaves não-NULL casaram — a maior vence.
    /// A escolha do vencedor por tributo acontece em Rust (primeira linha de
    /// cada tributo na ordenação), não em SQL, para manter a query legível.
    const SQL_REF: &'static str = "
        SELECT tributo, aliquota_bps,
               (uf IS NOT NULL)::int + (codigo_municipio IS NOT NULL)::int +
               (regime IS NOT NULL)::int + (c_class_trib IS NOT NULL)::int +
               (ncm_prefixo IS NOT NULL)::int AS especificidade
          FROM ref_aliquotas
         WHERE vigencia_inicio <= $1 AND (vigencia_fim IS NULL OR vigencia_fim >= $1)
           AND (uf IS NULL OR uf = $2)
           AND (codigo_municipio IS NULL OR codigo_municipio = $3)
           AND (regime IS NULL OR regime = $4)
           AND (c_class_trib IS NULL OR c_class_trib = $5)
           AND (ncm_prefixo IS NULL OR $6 LIKE ncm_prefixo || '%')
         ORDER BY tributo, especificidade DESC, vigencia_inicio DESC";

    const SQL_TENANT: &'static str = "
        SELECT tributo, aliquota_bps,
               (uf IS NOT NULL)::int + (codigo_municipio IS NOT NULL)::int +
               (regime IS NOT NULL)::int + (c_class_trib IS NOT NULL)::int +
               (ncm_prefixo IS NOT NULL)::int AS especificidade
          FROM aliquotas_tenant
         WHERE tenant_id = $7
           AND vigencia_inicio <= $1 AND (vigencia_fim IS NULL OR vigencia_fim >= $1)
           AND (uf IS NULL OR uf = $2)
           AND (codigo_municipio IS NULL OR codigo_municipio = $3)
           AND (regime IS NULL OR regime = $4)
           AND (c_class_trib IS NULL OR c_class_trib = $5)
           AND (ncm_prefixo IS NULL OR $6 LIKE ncm_prefixo || '%')
         ORDER BY tributo, especificidade DESC, vigencia_inicio DESC";

    fn preencher(resultado: &mut AliquotasItem, tributo: &str, bps: i32) -> Result<(), AppError> {
        let aliquota = Some(Aliquota::try_from(bps).map_err(AppError::Domain)?);
        let slot = match tributo {
            "icms" => &mut resultado.icms,
            "iss" => &mut resultado.iss,
            "pis" => &mut resultado.pis,
            "cofins" => &mut resultado.cofins,
            "cbs" => &mut resultado.cbs,
            "ibs_uf" => &mut resultado.ibs_uf,
            "ibs_mun" => &mut resultado.ibs_mun,
            "is" => &mut resultado.is_seletivo,
            _ => return Ok(()), // CHECK do banco impede; ignora por robustez
        };
        // Primeira linha de cada tributo na ordenação = vencedora; e o
        // override do tenant é aplicado antes da referência global.
        if slot.is_none() {
            *slot = aliquota;
        }
        Ok(())
    }
}

impl AliquotaProvider for PostgresAliquotaProvider {
    async fn resolver(
        &self,
        data: NaiveDate,
        perfil: &PerfilFiscal,
        classe: &ClasseTributaria,
        ncm: &str,
    ) -> Result<AliquotasItem, AppError> {
        let tenant_id = current_tenant_id()?;
        let mut resultado = AliquotasItem::default();

        for sql in [Self::SQL_TENANT, Self::SQL_REF] {
            let mut query = sqlx::query(sql)
                .bind(data)
                .bind(perfil.uf.as_str())
                .bind(perfil.codigo_municipio.as_str())
                .bind(perfil.regime.as_str())
                .bind(classe.as_str())
                .bind(ncm);
            if sql == Self::SQL_TENANT {
                query = query.bind(tenant_id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(AppError::infra)?;
            for row in rows {
                let tributo: String = row.get("tributo");
                let bps: i32 = row.get("aliquota_bps");
                Self::preencher(&mut resultado, &tributo, bps)?;
            }
        }
        Ok(resultado)
    }

    async fn classe_info(
        &self,
        classe: Option<&ClasseTributaria>,
    ) -> Result<ClasseTributariaInfo, AppError> {
        let Some(classe) = classe else {
            return Ok(ClasseTributariaInfo::integral());
        };
        let row = sqlx::query(
            "SELECT cst_ibs_cbs, reducao_bps FROM ref_classes_tributarias WHERE c_class_trib = $1",
        )
        .bind(classe.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::infra)?;

        Ok(match row {
            Some(row) => ClasseTributariaInfo {
                classe: classe.clone(),
                cst_ibs_cbs: row.get("cst_ibs_cbs"),
                reducao_bps: row.get("reducao_bps"),
            },
            // Classe desconhecida na referência: trata como integral em vez de
            // falhar a emissão — o produto foi classificado com um código que a
            // tabela ainda não tem.
            None => ClasseTributariaInfo::integral(),
        })
    }
}
