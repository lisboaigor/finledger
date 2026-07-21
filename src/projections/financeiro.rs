use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;
use uuid::Uuid;

use crate::financeiro::domain::events::FinanceiroEvent;
use crate::shared::tenant::current_tenant_id;

pub struct FinanceiroProjection {
    pool: Pool,
}

impl FinanceiroProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(
        &self,
        event: &FinanceiroEvent,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        match event {
            FinanceiroEvent::ContaReceberRegistrada {
                conta_id,
                venda_id,
                cliente_id,
                valor_centavos,
                vencimento,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                let Some(vid) = crate::projections::parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "INSERT INTO proj_contas_receber
                        (conta_id, venda_id, cliente_id, valor_original, valor_recebido,
                         status, vencimento, criada_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, 0, 'pendente', $5, $6, $6, $7)
                     ON CONFLICT (tenant_id, conta_id) DO NOTHING",
                )
                .bind(cid)
                .bind(vid)
                .bind(cli)
                .bind(*valor_centavos)
                .bind(*vencimento)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::PagamentoRecebido {
                conta_id,
                valor_centavos,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_receber
                     SET valor_recebido = valor_recebido + $2,
                         status = CASE
                             WHEN valor_recebido + $2 >= valor_original THEN 'liquidada'
                             ELSE 'parcial'
                         END,
                         atualizado_em = $3
                     WHERE conta_id = $1 AND tenant_id = $4",
                )
                .bind(cid)
                .bind(*valor_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaReceberLiquidada {
                conta_id,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_receber SET status = 'liquidada', atualizado_em = $2 WHERE conta_id = $1 AND tenant_id = $3",
                )
                .bind(cid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaReceberEstornada {
                conta_id,
                occurred_at,
                ..
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_receber SET status = 'estornada', atualizado_em = $2 WHERE conta_id = $1 AND tenant_id = $3",
                )
                .bind(cid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaPagarRegistrada {
                conta_id,
                pedido_id,
                fornecedor_id,
                valor_centavos,
                vencimento,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                let Some(ped) = crate::projections::parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                let Some(forn) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_contas_pagar
                        (conta_id, pedido_id, fornecedor_id, valor_original, valor_pago,
                         status, vencimento, criada_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, 0, 'pendente', $5, $6, $6, $7)
                     ON CONFLICT (tenant_id, conta_id) DO NOTHING",
                )
                .bind(cid)
                .bind(ped)
                .bind(forn)
                .bind(*valor_centavos)
                .bind(*vencimento)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::PagamentoEfetuado {
                conta_id,
                valor_centavos,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_pagar
                     SET valor_pago = valor_pago + $2,
                         status = CASE
                             WHEN valor_pago + $2 >= valor_original THEN 'liquidada'
                             ELSE 'parcial'
                         END,
                         atualizado_em = $3
                     WHERE conta_id = $1 AND tenant_id = $4",
                )
                .bind(cid)
                .bind(*valor_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaPagarLiquidada {
                conta_id,
                occurred_at,
            } => {
                let Some(cid) = crate::projections::parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_pagar SET status = 'liquidada', atualizado_em = $2 WHERE conta_id = $1 AND tenant_id = $3",
                )
                .bind(cid)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<FinanceiroEvent> for FinanceiroProjection {
    type Error = Infallible;

    async fn handle(&self, event: &FinanceiroEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("financeiro projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "financeiro projection failed");
        }
        Ok(())
    }
}
