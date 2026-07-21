use std::convert::Infallible;

use pharos_app::EventHandler;
use pharos_postgres::Pool;

use crate::catalogo::domain::events::CatalogoEvent;
use crate::shared::tenant::current_tenant_id;

pub struct CatalogoProjection {
    pool: Pool,
}

impl CatalogoProjection {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn apply(&self, event: &CatalogoEvent, tenant_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        match event {
            CatalogoEvent::ProdutoCadastrado {
                produto_id,
                sku,
                descricao,
                ncm,
                unidade,
                preco_custo_centavos,
                preco_venda_centavos,
                categoria,
                marca,
                controla_estoque,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "INSERT INTO proj_produtos
                        (produto_id, sku, descricao, ncm, unidade, preco_custo, preco_venda, categoria, marca, ativo, controla_estoque, criado_em, atualizado_em, tenant_id)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE, $10, $11, $11, $12)
                     ON CONFLICT (tenant_id, produto_id) DO NOTHING",
                )
                .bind(id)
                .bind(sku.as_str())
                .bind(descricao.as_str())
                .bind(ncm.as_str())
                .bind(unidade.as_str())
                .bind(*preco_custo_centavos)
                .bind(*preco_venda_centavos)
                .bind(categoria.as_str())
                .bind(marca.as_deref())
                .bind(*controla_estoque)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                self.registrar_historico_preco(tenant_id, id, *preco_venda_centavos, *occurred_at)
                    .await?;
            }
            CatalogoEvent::PrecosAtualizados {
                produto_id,
                preco_custo_centavos,
                preco_venda_centavos,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_produtos
                     SET preco_custo = $2, preco_venda = $3, atualizado_em = $4
                     WHERE produto_id = $1 AND tenant_id = $5",
                )
                .bind(id)
                .bind(*preco_custo_centavos)
                .bind(*preco_venda_centavos)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

                self.registrar_historico_preco(tenant_id, id, *preco_venda_centavos, *occurred_at)
                    .await?;
            }
            CatalogoEvent::ProdutoAtualizado {
                produto_id,
                sku,
                descricao,
                ncm,
                unidade,
                categoria,
                marca,
                controla_estoque,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_produtos
                     SET sku = $2, descricao = $3, ncm = $4, unidade = $5, categoria = $6, marca = $7,
                         controla_estoque = $8, atualizado_em = $9
                     WHERE produto_id = $1 AND tenant_id = $10",
                )
                .bind(id)
                .bind(sku.as_str())
                .bind(descricao.as_str())
                .bind(ncm.as_str())
                .bind(unidade.as_str())
                .bind(categoria.as_str())
                .bind(marca.as_deref())
                .bind(*controla_estoque)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CatalogoEvent::ProdutoDesativado {
                produto_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_produtos SET ativo = FALSE, atualizado_em = $2 WHERE produto_id = $1 AND tenant_id = $3",
                )
                .bind(id)
                .bind(*occurred_at)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;
            }
            CatalogoEvent::ProdutoReativado {
                produto_id,
                occurred_at,
            } => {
                let Some(id) = crate::projections::parse_uuid("produto_id", produto_id) else {
                    return Ok(());
                };
                sqlx::query(
                    "UPDATE proj_produtos SET ativo = TRUE, atualizado_em = $2 WHERE produto_id = $1 AND tenant_id = $3",
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

    /// Alimenta o histórico de preço de venda (base do cálculo de
    /// elasticidade). Uma linha por mudança; replays do mesmo evento caem no
    /// ON CONFLICT da chave (tenant, produto, vigente_desde).
    async fn registrar_historico_preco(
        &self,
        tenant_id: uuid::Uuid,
        produto_id: uuid::Uuid,
        preco_venda_centavos: i64,
        vigente_desde: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO proj_historico_precos (tenant_id, produto_id, preco_venda_centavos, vigente_desde)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (tenant_id, produto_id, vigente_desde) DO NOTHING",
        )
        .bind(tenant_id)
        .bind(produto_id)
        .bind(preco_venda_centavos)
        .bind(vigente_desde)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl EventHandler<CatalogoEvent> for CatalogoProjection {
    type Error = Infallible;

    async fn handle(&self, event: &CatalogoEvent) -> Result<(), Infallible> {
        let Ok(tenant_id) = current_tenant_id() else {
            tracing::error!("catalogo projection sem tenant em escopo; evento ignorado");
            return Ok(());
        };
        if let Err(e) = self.apply(event, tenant_id).await {
            tracing::error!(error = %e, "catalogo projection failed");
        }
        Ok(())
    }
}
