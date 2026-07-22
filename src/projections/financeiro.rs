use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;
use uuid::Uuid;

use crate::financeiro::domain::events::FinanceiroEvent;
use crate::projections::parse_uuid;
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
                descricao,
                occurred_at,
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                let Some(vid) = parse_uuid("venda_id", venda_id) else {
                    return Ok(());
                };
                let cli: Option<Uuid> = cliente_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                sqlx::query(
                    "INSERT INTO proj_contas_receber
                        (conta_id, venda_id, cliente_id, valor_original, valor_recebido,
                         status, vencimento, descricao, criada_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, 0, 'pendente', $5, $6, $7, $7, $8)
                     ON CONFLICT (tenant_id, conta_id) DO NOTHING",
                )
                .bind(cid)
                .bind(vid)
                .bind(cli)
                .bind(*valor_centavos)
                .bind(*vencimento)
                .bind(descricao)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            // Grava o ACUMULADO pós-evento (valor absoluto) em vez de
            // incrementar — reprocessar o mesmo evento é idempotente.
            FinanceiroEvent::PagamentoRecebido {
                conta_id,
                valor_recebido_total_centavos,
                occurred_at,
                ..
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_receber
                     SET valor_recebido = $2,
                         status = CASE
                             WHEN $2 + valor_abatido >= valor_original THEN 'liquidada'
                             ELSE 'parcial'
                         END,
                         atualizado_em = $3
                     WHERE conta_id = $1 AND tenant_id = $4 AND status <> 'estornada'",
                )
                .bind(cid)
                .bind(*valor_recebido_total_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            // Também absoluto (acumulado pós-evento) — idempotente.
            FinanceiroEvent::AbatimentoContaReceberRegistrado {
                conta_id,
                valor_abatido_total_centavos,
                occurred_at,
                ..
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_receber
                     SET valor_abatido = $2,
                         status = CASE
                             WHEN valor_recebido + $2 >= valor_original THEN 'liquidada'
                             ELSE status
                         END,
                         atualizado_em = $3
                     WHERE conta_id = $1 AND tenant_id = $4 AND status <> 'estornada'",
                )
                .bind(cid)
                .bind(*valor_abatido_total_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaReceberLiquidada {
                conta_id,
                occurred_at,
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
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
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
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
                descricao,
                occurred_at,
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                let Some(ped) = parse_uuid("pedido_id", pedido_id) else {
                    return Ok(());
                };
                let Some(forn) = parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_contas_pagar
                        (conta_id, pedido_id, fornecedor_id, valor_original, valor_pago,
                         status, vencimento, descricao, criada_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, 0, 'pendente', $5, $6, $7, $7, $8)
                     ON CONFLICT (tenant_id, conta_id) DO NOTHING",
                )
                .bind(cid)
                .bind(ped)
                .bind(forn)
                .bind(*valor_centavos)
                .bind(*vencimento)
                .bind(descricao)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            // Acumulado pós-evento (valor absoluto) — idempotente.
            FinanceiroEvent::PagamentoEfetuado {
                conta_id,
                valor_pago_total_centavos,
                occurred_at,
                ..
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_contas_pagar
                     SET valor_pago = $2,
                         status = CASE
                             WHEN $2 >= valor_original THEN 'liquidada'
                             ELSE 'parcial'
                         END,
                         atualizado_em = $3
                     WHERE conta_id = $1 AND tenant_id = $4",
                )
                .bind(cid)
                .bind(*valor_pago_total_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FinanceiroEvent::ContaPagarLiquidada {
                conta_id,
                occurred_at,
            } => {
                let Some(cid) = parse_uuid("conta_id", conta_id) else {
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
