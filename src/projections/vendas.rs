use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;
use uuid::Uuid;

use crate::shared::tenant::current_tenant_id;
use crate::vendas::domain::events::VendaEvent;

pub struct VendasProjection {
    pool: Pool,
}

impl VendasProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &VendaEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            VendaEvent::VendaIniciada {
                venda_id,
                vendedor_id,
                cliente_id,
                occurred_at,
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let Some(vend) = crate::projections::parse_uuid("vendedor_id", vendedor_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "INSERT INTO proj_vendas
                        (venda_id, vendedor_id, cliente_id, total_centavos, status,
                         criada_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, 0, 'iniciada', $4, $4, $5)
                     ON CONFLICT (tenant_id, venda_id) DO NOTHING",
                )
                .bind(vid)
                .bind(vend)
                .bind(cli)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::ItemAdicionado {
                venda_id,
                item_id,
                produto_id,
                sku,
                descricao,
                quantidade,
                preco_unitario_centavos,
                occurred_at,
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let Some(iid) = crate::projections::parse_uuid("item_id", item_id) else {
                    return Ok(());
                };
                let Some(pid) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                let subtotal = (*quantidade as i64) * preco_unitario_centavos;

                sqlx::query(
                    "INSERT INTO proj_vendas_itens
                        (item_id, venda_id, produto_id, sku, descricao,
                         quantidade, preco_unitario_centavos, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                     ON CONFLICT (tenant_id, item_id) DO NOTHING",
                )
                .bind(iid)
                .bind(vid)
                .bind(pid)
                .bind(sku.as_str())
                .bind(descricao.as_str())
                .bind(*quantidade as i32)
                .bind(*preco_unitario_centavos)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    "UPDATE proj_vendas
                     SET total_centavos = total_centavos + $2, atualizado_em = $3
                     WHERE venda_id = $1 AND tenant_id = $4",
                )
                .bind(vid)
                .bind(subtotal)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::ItemRemovido {
                venda_id,
                item_id,
                occurred_at,
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let Some(iid) = crate::projections::parse_uuid("item_id", item_id) else {
                    return Ok(());
                };

                sqlx::query("DELETE FROM proj_vendas_itens WHERE item_id = $1 AND tenant_id = $2")
                    .bind(iid)
                    .bind(tenant_id)
                    .execute(&self.pool)
                    .await?;

                sqlx::query(
                    "UPDATE proj_vendas
                     SET total_centavos = (
                         SELECT COALESCE(SUM(CAST(quantidade AS BIGINT) * preco_unitario_centavos), 0)
                         FROM proj_vendas_itens WHERE venda_id = $1 AND tenant_id = $3
                     ), atualizado_em = $2
                     WHERE venda_id = $1 AND tenant_id = $3",
                )
                .bind(vid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::FormaPagamentoDefinida {
                venda_id,
                forma,
                occurred_at,
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                // Rótulo humano (Display) — a UI exibe esta coluna diretamente.
                let forma_str = forma.to_string();
                sqlx::query(
                    "UPDATE proj_vendas SET forma_pagamento = $2, atualizado_em = $3 WHERE venda_id = $1 AND tenant_id = $4",
                )
                .bind(vid)
                .bind(forma_str)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::VendaConfirmada {
                venda_id,
                total_centavos,
                forma_pagamento,
                occurred_at,
                ..
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let forma_str = forma_pagamento.to_string();
                sqlx::query(
                    "UPDATE proj_vendas
                     SET status = 'confirmada', total_centavos = $2,
                         forma_pagamento = $3, confirmada_em = $4, atualizado_em = $4
                     WHERE venda_id = $1 AND tenant_id = $5",
                )
                .bind(vid)
                .bind(*total_centavos)
                .bind(forma_str)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::VendaCancelada {
                venda_id,
                occurred_at,
                ..
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_vendas SET status = 'cancelada', atualizado_em = $2 WHERE venda_id = $1 AND tenant_id = $3",
                )
                .bind(vid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::ItensDevolvidos {
                venda_id,
                itens_devolvidos,
                devolucao_total,
                occurred_at,
                ..
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                // Devolução TOTAL preserva os itens para auditoria — a venda vira
                // 'cancelada' pelo VendaCancelada emitido em seguida. Só a parcial
                // reduz quantidades (e remove itens zerados), refletindo a NF
                // que será reemitida com os itens restantes.
                if !devolucao_total {
                    for item in itens_devolvidos {
                        let Some(iid) = crate::projections::parse_uuid("item_id", &item.item_id)
                        else {
                            continue;
                        };
                        sqlx::query(
                            "UPDATE proj_vendas_itens SET quantidade = quantidade - $2
                             WHERE item_id = $1 AND tenant_id = $3",
                        )
                        .bind(iid)
                        .bind(item.quantidade as i32)
                        .bind(tenant_id)
                        .execute(&self.pool)
                        .await?;
                    }
                    sqlx::query(
                        "DELETE FROM proj_vendas_itens
                         WHERE venda_id = $1 AND tenant_id = $2 AND quantidade <= 0",
                    )
                    .bind(vid)
                    .bind(tenant_id)
                    .execute(&self.pool)
                    .await?;
                }
                sqlx::query(
                    "UPDATE proj_vendas
                     SET total_centavos = (
                         SELECT COALESCE(SUM(CAST(quantidade AS BIGINT) * preco_unitario_centavos), 0)
                         FROM proj_vendas_itens WHERE venda_id = $1 AND tenant_id = $3
                     ), atualizado_em = $2
                     WHERE venda_id = $1 AND tenant_id = $3",
                )
                .bind(vid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            VendaEvent::VendaAtualizada {
                venda_id,
                cliente_id,
                occurred_at,
            } => {
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "UPDATE proj_vendas SET cliente_id = $2, atualizado_em = $3 WHERE venda_id = $1 AND tenant_id = $4",
                )
                .bind(vid)
                .bind(cli)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<VendaEvent> for VendasProjection {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("vendas projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "vendas projection failed");
        }
        Ok(())
    }
}
