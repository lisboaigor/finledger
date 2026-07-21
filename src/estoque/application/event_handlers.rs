use std::convert::Infallible;
use std::sync::Arc;
use uuid::Uuid;

use pharos_app::{CommandHandler, EventHandler};
use pharos_postgres::Pool;

use super::commands::{BaixarEstoque, RegistrarEntradaEstoque};
use super::handler::EstoqueHandlers;
use crate::compras::domain::events::ComprasEvent;
use crate::vendas::domain::events::VendaEvent;

/// Registra entrada de estoque para cada item quando MercadoriaRecebida é publicado.
pub struct EstoqueComprasEventHandler {
    pub estoque: Arc<EstoqueHandlers>,
}

impl EventHandler<ComprasEvent> for EstoqueComprasEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &ComprasEvent) -> Result<(), Infallible> {
        if let ComprasEvent::MercadoriaRecebida { itens, .. } = event {
            for item in itens {
                let produto_id = match Uuid::parse_str(&item.produto_id) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                let _ = self
                    .estoque
                    .handle(RegistrarEntradaEstoque {
                        produto_id,
                        quantidade: item.quantidade,
                        custo_unitario_centavos: item.custo_unitario_centavos,
                        motivo: "Recebimento de mercadoria".into(),
                        nota_fiscal: None,
                    })
                    .await;
            }
        }
        Ok(())
    }
}

/// Reentra itens devolvidos no estoque quando ItensDevolvidos é publicado.
///
/// A reentrada usa o custo médio ATUAL do produto (lido da projeção) para não
/// distorcer a média ponderada — devolução não é uma nova compra.
pub struct EstoqueDevolucaoEventHandler {
    pub estoque: Arc<EstoqueHandlers>,
    pub pool: Pool,
}

impl EventHandler<VendaEvent> for EstoqueDevolucaoEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        if let VendaEvent::ItensDevolvidos {
            venda_id,
            itens_devolvidos,
            ..
        } = event
        {
            for item in itens_devolvidos {
                let produto_id = match Uuid::parse_str(&item.produto_id) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                let custo_medio: i64 = sqlx::query_scalar(
                    "SELECT custo_medio FROM proj_saldo_estoque WHERE produto_id = $1",
                )
                .bind(produto_id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten()
                .unwrap_or(0);

                if let Err(err) = self
                    .estoque
                    .handle(RegistrarEntradaEstoque {
                        produto_id,
                        quantidade: item.quantidade,
                        custo_unitario_centavos: custo_medio,
                        motivo: format!("Devolução da venda {venda_id}"),
                        nota_fiscal: None,
                    })
                    .await
                {
                    tracing::warn!(
                        venda_id,
                        produto_id = %produto_id,
                        error = %err,
                        "falha ao reentrar estoque de item devolvido"
                    );
                }
            }
        }
        Ok(())
    }
}

/// Baixa estoque para cada item quando VendaConfirmada é publicado no EventBus.
///
/// Roda depois que a venda já foi persistida (save_and_publish salva e só então
/// publica), então estoque insuficiente aqui não desfaz a venda — é uma reação
/// assíncrona best-effort, não um guard transacional. Ver bootstrap/events.rs.
pub struct EstoqueVendaEventHandler {
    pub estoque: Arc<EstoqueHandlers>,
}

impl EventHandler<VendaEvent> for EstoqueVendaEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        if let VendaEvent::VendaConfirmada {
            venda_id, itens, ..
        } = event
        {
            for item in itens {
                let produto_id = match Uuid::parse_str(&item.produto_id) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if let Err(err) = self
                    .estoque
                    .handle(BaixarEstoque {
                        produto_id,
                        quantidade: item.quantidade,
                        referencia_id: Some(venda_id.clone()),
                    })
                    .await
                {
                    tracing::warn!(
                        venda_id,
                        produto_id = %produto_id,
                        error = %err,
                        "falha ao baixar estoque para venda confirmada"
                    );
                }
            }
        }
        Ok(())
    }
}
