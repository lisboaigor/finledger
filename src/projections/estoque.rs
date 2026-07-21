use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::estoque::domain::events::EstoqueEvent;
use crate::shared::tenant::current_tenant_id;

pub struct EstoqueProjection {
    pool: Pool,
}

impl EstoqueProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &EstoqueEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            EstoqueEvent::EstoqueEntrada {
                produto_id,
                quantidade,
                custo_unitario_centavos,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                let qty = *quantidade as i32;
                let custo = *custo_unitario_centavos;
                // UPSERT: na inserção usa o custo da entrada como custo_medio inicial;
                // no conflito recalcula a média ponderada.
                sqlx::query(
                    "INSERT INTO proj_saldo_estoque (produto_id, quantidade, custo_medio, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (tenant_id, produto_id) DO UPDATE SET
                         custo_medio   = CASE
                             WHEN proj_saldo_estoque.quantidade + EXCLUDED.quantidade = 0 THEN 0
                             ELSE (proj_saldo_estoque.custo_medio * proj_saldo_estoque.quantidade
                                   + EXCLUDED.custo_medio * EXCLUDED.quantidade)
                                  / (proj_saldo_estoque.quantidade + EXCLUDED.quantidade)
                             END,
                         quantidade    = proj_saldo_estoque.quantidade + EXCLUDED.quantidade,
                         atualizado_em = EXCLUDED.atualizado_em",
                )
                .bind(pid)
                .bind(qty)
                .bind(custo)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            EstoqueEvent::EstoqueSaida {
                produto_id,
                quantidade,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_saldo_estoque
                     SET quantidade = GREATEST(0, quantidade - $2), atualizado_em = $3
                     WHERE produto_id = $1 AND tenant_id = $4",
                )
                .bind(pid)
                .bind(*quantidade as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            EstoqueEvent::AjusteEstoque {
                item_id,
                quantidade_nova,
                occurred_at,
                ..
            } => {
                // item_id == produto_id neste sistema (ItemEstoqueId::from_uuid)
                let Some(pid) = crate::projections::parse_uuid("item_id", item_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_saldo_estoque
                     SET quantidade = $2, atualizado_em = $3
                     WHERE produto_id = $1 AND tenant_id = $4",
                )
                .bind(pid)
                .bind(*quantidade_nova as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            EstoqueEvent::EstoqueMinimoDefinido {
                produto_id,
                estoque_minimo,
                occurred_at,
                ..
            } => {
                let Some(pid) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_saldo_estoque (produto_id, quantidade, custo_medio, estoque_minimo, atualizado_em, tenant_id)
                     VALUES ($1, 0, 0, $2, $3, $4)
                     ON CONFLICT (tenant_id, produto_id) DO UPDATE SET
                         estoque_minimo = EXCLUDED.estoque_minimo,
                         atualizado_em  = EXCLUDED.atualizado_em",
                )
                .bind(pid)
                .bind(*estoque_minimo as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            // Evento de alerta: sem dado novo para persistir na projeção.
            EstoqueEvent::EstoqueMinimoPadraoAtingido { .. } => {}
        }
        Ok(())
    }
}

impl EventHandler<EstoqueEvent> for EstoqueProjection {
    type Error = Infallible;

    async fn handle(&self, event: &EstoqueEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("estoque projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "estoque projection failed");
        }
        Ok(())
    }
}
