use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::fiscal::domain::events::NotaFiscalEvent;
use crate::fiscal::domain::value_objects::ImpostoItem;
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
                itens,
                totais,
                ibs_cbs_informativo,
                occurred_at,
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
                      total_centavos, desconto_centavos, gerada_em, atualizado_em, tenant_id,
                      icms_centavos, pis_centavos, cofins_centavos, iss_centavos,
                      cbs_centavos, ibs_uf_centavos, ibs_mun_centavos, is_centavos)
                     VALUES ($1,$2,$3,$4,$5,$6,'gerada',$7,$8,$9,$9,$10,
                             $11,$12,$13,$14,$15,$16,$17,$18)
                     ON CONFLICT (tenant_id, nf_id) DO NOTHING",
                )
                .bind(nf_uuid)
                .bind(venda_uuid)
                .bind(cliente_uuid)
                .bind(modelo.codigo())
                .bind(serie)
                .bind(*numero as i32)
                .bind(totais.total_centavos)
                .bind(totais.desconto_centavos)
                .bind(occurred_at)
                .bind(tenant_id)
                .bind(totais.icms_centavos)
                .bind(totais.pis_centavos)
                .bind(totais.cofins_centavos)
                .bind(totais.iss_centavos)
                .bind(totais.cbs_centavos)
                .bind(totais.ibs_uf_centavos)
                .bind(totais.ibs_mun_centavos)
                .bind(totais.is_centavos)
                .execute(&self.pool)
                .await?;

                // Breakdown por produto para a margem líquida do BI. Impostos
                // congelados no evento; `ibs_cbs_informativo` idem — o ETL
                // decide se soma IBS/CBS ao custo do vendedor. A NF pode ter
                // duas linhas do mesmo produto (a venda não funde linhas), então
                // agregamos por produto: a PK (tenant, nf, produto) exige grão
                // por produto e um DO NOTHING descartaria a 2ª linha.
                let mut por_produto: std::collections::HashMap<uuid::Uuid, (i32, i64, ImpostoItem)> =
                    std::collections::HashMap::new();
                for item in itens {
                    let (qtd, total, imp) = por_produto
                        .entry(item.produto_id)
                        .or_insert_with(|| (0, 0, ImpostoItem::default()));
                    *qtd += item.quantidade() as i32;
                    *total += item.total_centavos();
                    imp.icms_centavos += item.imposto.icms_centavos;
                    imp.iss_centavos += item.imposto.iss_centavos;
                    imp.pis_centavos += item.imposto.pis_centavos;
                    imp.cofins_centavos += item.imposto.cofins_centavos;
                    imp.cbs_centavos += item.imposto.cbs_centavos;
                    imp.ibs_uf_centavos += item.imposto.ibs_uf_centavos;
                    imp.ibs_mun_centavos += item.imposto.ibs_mun_centavos;
                    imp.is_centavos += item.imposto.is_centavos;
                }
                for (produto_id, (quantidade, total, imp)) in por_produto {
                    sqlx::query(
                        "INSERT INTO proj_nf_itens
                         (tenant_id, nf_id, venda_id, produto_id, quantidade, total_centavos,
                          icms_centavos, iss_centavos, pis_centavos, cofins_centavos,
                          cbs_centavos, ibs_uf_centavos, ibs_mun_centavos, is_centavos,
                          ibs_cbs_informativo)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                         ON CONFLICT (tenant_id, nf_id, produto_id) DO NOTHING",
                    )
                    .bind(tenant_id)
                    .bind(nf_uuid)
                    .bind(venda_uuid)
                    .bind(produto_id)
                    .bind(quantidade)
                    .bind(total)
                    .bind(imp.icms_centavos)
                    .bind(imp.iss_centavos)
                    .bind(imp.pis_centavos)
                    .bind(imp.cofins_centavos)
                    .bind(imp.cbs_centavos)
                    .bind(imp.ibs_uf_centavos)
                    .bind(imp.ibs_mun_centavos)
                    .bind(imp.is_centavos)
                    .bind(*ibs_cbs_informativo)
                    .execute(&self.pool)
                    .await?;
                }
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

            NotaFiscalEvent::NotaFiscalRetransmitida {
                nf_id, occurred_at, ..
            } => {
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
