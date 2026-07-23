use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::crm::domain::events::CrmEvent;
use crate::shared::tenant::current_tenant_id;

pub struct CrmProjection {
    pool: Pool,
}

impl CrmProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &CrmEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            CrmEvent::ClienteCadastrado {
                cliente_id,
                nome,
                cpf_cnpj,
                uf,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_clientes
                        (cliente_id, nome, cpf_cnpj, uf, bloqueado, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, FALSE, $5, $5, $6)
                     ON CONFLICT (tenant_id, cliente_id) DO NOTHING",
                )
                .bind(id)
                .bind(nome.as_str())
                .bind(cpf_cnpj.as_str())
                .bind(uf.as_deref())
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CrmEvent::ClienteAtualizado {
                cliente_id,
                nome,
                telefone,
                email,
                uf,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_clientes
                     SET nome = $2, telefone = $3, email = $4, uf = $5, atualizado_em = $6
                     WHERE cliente_id = $1 AND tenant_id = $7",
                )
                .bind(id)
                .bind(nome.as_str())
                .bind(telefone.as_deref())
                .bind(email.as_deref())
                .bind(uf.as_deref())
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CrmEvent::ClienteBloqueado {
                cliente_id,
                occurred_at,
                ..
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_clientes SET bloqueado = TRUE, atualizado_em = $2 WHERE cliente_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CrmEvent::ClienteDesbloqueado {
                cliente_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_clientes SET bloqueado = FALSE, atualizado_em = $2 WHERE cliente_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CrmEvent::ClienteDesativado {
                cliente_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_clientes SET ativo = FALSE, atualizado_em = $2 WHERE cliente_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CrmEvent::ClienteReativado {
                cliente_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("cliente_id", cliente_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_clientes SET ativo = TRUE, atualizado_em = $2 WHERE cliente_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

impl EventHandler<CrmEvent> for CrmProjection {
    type Error = Infallible;

    async fn handle(&self, event: &CrmEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("crm projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "crm projection failed");
        }
        Ok(())
    }
}
