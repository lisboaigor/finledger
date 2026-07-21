use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::fornecedores::domain::events::FornecedorEvent;
use crate::shared::tenant::current_tenant_id;

pub struct FornecedoresProjection {
    pool: Pool,
}

impl FornecedoresProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(
        &self,
        event: &FornecedorEvent,
        tenant_id: uuid::Uuid,
    ) -> Result<(), sqlx::Error> {
        match event {
            FornecedorEvent::FornecedorCadastrado {
                fornecedor_id,
                razao_social,
                cnpj,
                telefone,
                email,
                prazo_pagamento_dias,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_fornecedores
                        (fornecedor_id, razao_social, cnpj, telefone, email,
                         prazo_pagamento_dias, ativo, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7, $7, $8)
                     ON CONFLICT (tenant_id, fornecedor_id) DO NOTHING",
                )
                .bind(id)
                .bind(razao_social.as_str())
                .bind(cnpj.as_str())
                .bind(telefone.as_deref())
                .bind(email.as_deref())
                .bind(*prazo_pagamento_dias as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FornecedorEvent::FornecedorAtualizado {
                fornecedor_id,
                razao_social,
                telefone,
                email,
                prazo_pagamento_dias,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_fornecedores
                     SET razao_social = $2, telefone = $3, email = $4,
                         prazo_pagamento_dias = $5, atualizado_em = $6
                     WHERE fornecedor_id = $1 AND tenant_id = $7",
                )
                .bind(id)
                .bind(razao_social.as_str())
                .bind(telefone.as_deref())
                .bind(email.as_deref())
                .bind(*prazo_pagamento_dias as i32)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FornecedorEvent::FornecedorDesativado {
                fornecedor_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_fornecedores SET ativo = FALSE, atualizado_em = $2 WHERE fornecedor_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            FornecedorEvent::FornecedorReativado {
                fornecedor_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("fornecedor_id", fornecedor_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_fornecedores SET ativo = TRUE, atualizado_em = $2 WHERE fornecedor_id = $1 AND tenant_id = $3",
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

impl EventHandler<FornecedorEvent> for FornecedoresProjection {
    type Error = Infallible;

    async fn handle(&self, event: &FornecedorEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("fornecedores projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "fornecedores projection failed");
        }
        Ok(())
    }
}
