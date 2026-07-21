use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;
use uuid::Uuid;

use crate::orcamentos::domain::events::OrcamentoEvent;
use crate::shared::tenant::current_tenant_id;

pub struct OrcamentosProjection {
    pool: Pool,
}

impl OrcamentosProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(
        &self,
        event: &OrcamentoEvent,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        match event {
            OrcamentoEvent::OrcamentoCriado {
                orcamento_id,
                vendedor_id,
                cliente_id,
                cliente_avulso,
                validade_dias,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                let Some(vend) = crate::projections::parse_uuid("vendedor_id", vendedor_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "INSERT INTO proj_orcamentos
                        (orcamento_id, vendedor_id, cliente_id, cliente_avulso, total_centavos, desconto_centavos,
                         status, validade_dias, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, 0, 0, 'rascunho', $5, $6, $6, $7)
                     ON CONFLICT (tenant_id, orcamento_id) DO NOTHING",
                )
                .bind(oid)
                .bind(vend)
                .bind(cli)
                .bind(cliente_avulso.as_deref())
                .bind(*validade_dias as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::ItemAdicionadoOrcamento {
                orcamento_id,
                item_id,
                produto_id,
                sku,
                descricao,
                quantidade,
                preco_unitario_centavos,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                let Some(iid) = crate::projections::parse_uuid("item_id", item_id) else {
                    return Ok(());
                };
                let Some(pid) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };

                sqlx::query(
                    "INSERT INTO proj_orcamentos_itens
                        (item_id, orcamento_id, produto_id, sku, descricao,
                         quantidade, preco_unitario_centavos, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                     ON CONFLICT (tenant_id, item_id) DO NOTHING",
                )
                .bind(iid)
                .bind(oid)
                .bind(pid)
                .bind(sku.as_str())
                .bind(descricao.as_str())
                .bind(*quantidade as i32)
                .bind(*preco_unitario_centavos)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                // total_centavos é sempre LÍQUIDO (itens − desconto) — a UI e o
                // BI exibem esta coluna diretamente, em qualquer status.
                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET total_centavos = (
                         SELECT COALESCE(SUM(CAST(quantidade AS BIGINT) * preco_unitario_centavos), 0)
                         FROM proj_orcamentos_itens WHERE orcamento_id = $1 AND tenant_id = $3
                     ) - desconto_centavos, atualizado_em = $2
                     WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::ItemRemovidoOrcamento {
                orcamento_id,
                item_id,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                let Some(iid) = crate::projections::parse_uuid("item_id", item_id) else {
                    return Ok(());
                };

                sqlx::query(
                    "DELETE FROM proj_orcamentos_itens WHERE item_id = $1 AND tenant_id = $2",
                )
                .bind(iid)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET total_centavos = (
                         SELECT COALESCE(SUM(CAST(quantidade AS BIGINT) * preco_unitario_centavos), 0)
                         FROM proj_orcamentos_itens WHERE orcamento_id = $1 AND tenant_id = $3
                     ) - desconto_centavos, atualizado_em = $2
                     WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::DescontoOrcamentoAplicado {
                orcamento_id,
                desconto_centavos,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                // Aplica o desconto E reflete no total líquido imediatamente —
                // era aqui que "o desconto não era aplicado" aos olhos do gestor.
                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET desconto_centavos = $2,
                         total_centavos = (
                             SELECT COALESCE(SUM(CAST(quantidade AS BIGINT) * preco_unitario_centavos), 0)
                             FROM proj_orcamentos_itens WHERE orcamento_id = $1 AND tenant_id = $4
                         ) - $2,
                         atualizado_em = $3
                     WHERE orcamento_id = $1 AND tenant_id = $4",
                )
                .bind(oid)
                .bind(*desconto_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoEmitido {
                orcamento_id,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_orcamentos SET status = 'emitido', atualizado_em = $2 WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoAceito {
                orcamento_id,
                total_centavos,
                occurred_at,
                ..
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                // A guarda de status evita regressão sob reentrância síncrona
                // do event bus: ao aceitar, o VendaAPartirDeOrcamentoHandler
                // roda aninhado e já converte o orçamento (status 'convertido')
                // ANTES de esta projeção do OrcamentoAceito rodar. Sem a guarda,
                // o 'aceito' sobrescreveria o 'convertido'. Um orçamento
                // convertido/cancelado nunca volta a 'aceito'.
                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET status = 'aceito', total_centavos = $2, atualizado_em = $3
                     WHERE orcamento_id = $1 AND tenant_id = $4
                       AND status NOT IN ('convertido', 'cancelado')",
                )
                .bind(oid)
                .bind(*total_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoRecusado {
                orcamento_id,
                occurred_at,
                ..
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_orcamentos SET status = 'recusado', atualizado_em = $2 WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoExpirado {
                orcamento_id,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_orcamentos SET status = 'expirado', atualizado_em = $2 WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoConvertidoEmVenda {
                orcamento_id,
                venda_id,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET status = 'convertido', venda_id = $2, atualizado_em = $3
                     WHERE orcamento_id = $1 AND tenant_id = $4",
                )
                .bind(oid)
                .bind(vid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoAtualizado {
                orcamento_id,
                cliente_id,
                cliente_avulso,
                validade_dias,
                occurred_at,
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "UPDATE proj_orcamentos
                     SET cliente_id = $2, cliente_avulso = $3, validade_dias = $4, atualizado_em = $5
                     WHERE orcamento_id = $1 AND tenant_id = $6",
                )
                .bind(oid)
                .bind(cli)
                .bind(cliente_avulso.as_deref())
                .bind(*validade_dias as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            OrcamentoEvent::OrcamentoCancelado {
                orcamento_id,
                occurred_at,
                ..
            } => {
                let Some(oid) = crate::projections::parse_uuid("orcamento_id", orcamento_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_orcamentos SET status = 'cancelado', atualizado_em = $2 WHERE orcamento_id = $1 AND tenant_id = $3",
                )
                .bind(oid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<OrcamentoEvent> for OrcamentosProjection {
    type Error = Infallible;

    async fn handle(&self, event: &OrcamentoEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("orcamentos projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "orcamentos projection failed");
        }
        Ok(())
    }
}
