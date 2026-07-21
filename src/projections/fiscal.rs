use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::fiscal::domain::events::NotaFiscalEvent;
use crate::shared::tenant::current_tenant_id;

pub struct FiscalProjection {
    pool: Pool,
}

impl FiscalProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(
        &self,
        event: &NotaFiscalEvent,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        match event {
            NotaFiscalEvent::NotaFiscalGerada {
                nf_id,
                venda_id,
                cliente_id,
                modelo,
                serie,
                numero,
                totais,
                occurred_at,
                ..
            } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                let venda_uuid =
                    uuid::Uuid::parse_str(venda_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                let cliente_uuid = cliente_id
                    .as_deref()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());

                sqlx::query(
                    "INSERT INTO proj_notas_fiscais
                     (nf_id, venda_id, cliente_id, modelo, serie, numero, status,
                      total_centavos, gerada_em, atualizado_em, tenant_id)
                     VALUES ($1,$2,$3,$4,$5,$6,'gerada',$7,$8,$8,$9)
                     ON CONFLICT (tenant_id, nf_id) DO NOTHING",
                )
                .bind(nf_uuid)
                .bind(venda_uuid)
                .bind(cliente_uuid)
                .bind(modelo.codigo())
                .bind(serie)
                .bind(*numero as i32)
                .bind(totais.total_centavos)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }

            NotaFiscalEvent::NotaFiscalTransmitida { nf_id, occurred_at } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                sqlx::query(
                    "UPDATE proj_notas_fiscais
                     SET status='transmitida', atualizado_em=$2
                     WHERE nf_id=$1 AND tenant_id=$3",
                )
                .bind(nf_uuid)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }

            NotaFiscalEvent::NotaFiscalAutorizada {
                nf_id,
                chave,
                protocolo,
                occurred_at,
            } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                sqlx::query(
                    "UPDATE proj_notas_fiscais
                     SET status='autorizada', chave=$2, protocolo=$3,
                         autorizada_em=$4, atualizado_em=$4
                     WHERE nf_id=$1 AND tenant_id=$5",
                )
                .bind(nf_uuid)
                .bind(chave)
                .bind(protocolo)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }

            NotaFiscalEvent::NotaFiscalRejeitada {
                nf_id,
                codigo,
                motivo,
                occurred_at,
            } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                sqlx::query(
                    "UPDATE proj_notas_fiscais
                     SET status='rejeitada',
                         rejeicao_codigo=$2, rejeicao_motivo=$3,
                         atualizado_em=$4
                     WHERE nf_id=$1 AND tenant_id=$5",
                )
                .bind(nf_uuid)
                .bind(codigo)
                .bind(motivo)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }

            NotaFiscalEvent::CancelamentoNfSolicitado {
                nf_id, occurred_at, ..
            } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                sqlx::query(
                    "UPDATE proj_notas_fiscais
                     SET cancelamento_pendente=TRUE, atualizado_em=$2
                     WHERE nf_id=$1 AND tenant_id=$3",
                )
                .bind(nf_uuid)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }

            NotaFiscalEvent::NotaFiscalCancelada {
                nf_id, occurred_at, ..
            } => {
                let nf_uuid =
                    uuid::Uuid::parse_str(nf_id).map_err(|e| sqlx::Error::Decode(e.into()))?;
                sqlx::query(
                    "UPDATE proj_notas_fiscais
                     SET status='cancelada', cancelamento_pendente=FALSE, cancelada_em=$2, atualizado_em=$2
                     WHERE nf_id=$1 AND tenant_id=$3",
                )
                .bind(nf_uuid)
                .bind(occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<NotaFiscalEvent> for FiscalProjection {
    type Error = Infallible;

    async fn handle(&self, event: &NotaFiscalEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("fiscal projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!("FiscalProjection erro: {e}");
        }
        Ok(())
    }
}
