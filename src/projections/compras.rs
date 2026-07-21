use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::compras::domain::events::ComprasEvent;
use crate::shared::tenant::current_tenant_id;

pub struct ComprasProjection {
    pool: Pool,
}

impl ComprasProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &ComprasEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            ComprasEvent::PedidoCompraGerado {
                pedido_id,
                comprador_id,
                fornecedor_id,
                itens,
                prazo_pagamento_dias,
                occurred_at,
            } => {
                let Some(pid) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                let Some(comp) = crate::projections::parse_uuid("comprador_id", comprador_id) else {
                    return Ok(());
                };
                let Some(forn) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                let total: i64 = itens
                    .iter()
                    .map(|i| i.quantidade as i64 * i.custo_unitario_centavos)
                    .sum();

                sqlx::query(
                    "INSERT INTO proj_pedidos_compra
                        (pedido_id, comprador_id, fornecedor_id, total_centavos,
                         prazo_pagamento_dias, status, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, 'gerado', $6, $6, $7)
                     ON CONFLICT (tenant_id, pedido_id) DO NOTHING",
                )
                .bind(pid)
                .bind(comp)
                .bind(forn)
                .bind(total)
                .bind(*prazo_pagamento_dias as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                for item in itens {
                    let Some(item_pid) =
                        crate::projections::parse_uuid("produto_id", &item.produto_id)
                    else {
                        continue;
                    };
                    sqlx::query(
                        "INSERT INTO proj_pedidos_compra_itens
                            (pedido_id, produto_id, quantidade, custo_unitario_centavos, tenant_id)
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(pid)
                    .bind(item_pid)
                    .bind(item.quantidade as i32)
                    .bind(item.custo_unitario_centavos)
                    .bind(tenant_id)
                    .execute(&self.pool)
                    .await?;
                }
            }
            ComprasEvent::PedidoCompraAprovado {
                pedido_id,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_pedidos_compra SET status = 'aprovado', atualizado_em = $2 WHERE pedido_id = $1 AND tenant_id = $3",
                )
                .bind(pid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            ComprasEvent::PedidoCompraEnviado {
                pedido_id,
                occurred_at,
            } => {
                let Some(pid) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_pedidos_compra SET status = 'enviado', atualizado_em = $2 WHERE pedido_id = $1 AND tenant_id = $3",
                )
                .bind(pid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            ComprasEvent::MercadoriaRecebida {
                pedido_id,
                tudo_recebido,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                let status = if *tudo_recebido {
                    "recebido_total"
                } else {
                    "recebido_parcial"
                };
                sqlx::query(
                    "UPDATE proj_pedidos_compra SET status = $4, atualizado_em = $2 WHERE pedido_id = $1 AND tenant_id = $3",
                )
                .bind(pid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .bind(status)
                .execute(&self.pool)
                .await?;
            }
            ComprasEvent::PedidoCancelado {
                pedido_id,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_pedidos_compra SET status = 'cancelado', atualizado_em = $2 WHERE pedido_id = $1 AND tenant_id = $3",
                )
                .bind(pid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<ComprasEvent> for ComprasProjection {
    type Error = Infallible;

    async fn handle(&self, event: &ComprasEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("compras projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "compras projection failed");
        }
        Ok(())
    }
}
