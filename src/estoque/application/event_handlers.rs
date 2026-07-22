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
            let tenant_id = match crate::shared::tenant::current_tenant_id() {
                Ok(id) => id,
                Err(_) => {
                    tracing::error!(
                        venda_id,
                        "devolução sem tenant em escopo — reentrada de estoque PULADA; reconcilie manualmente"
                    );
                    return Ok(());
                }
            };
            for item in itens_devolvidos {
                let produto_id = match Uuid::parse_str(&item.produto_id) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                // O evento ItensDevolvidos não carrega custo (só preço de venda),
                // então o custo médio ATUAL da projeção é a única fonte. Sem ele
                // (linha ausente ou erro), NÃO reentramos a custo 0 — isso
                // distorceria a média ponderada; pulamos e registramos para
                // reconciliação manual.
                let custo_medio: i64 = match sqlx::query_scalar::<_, i64>(
                    "SELECT custo_medio FROM proj_saldo_estoque
                      WHERE tenant_id = $1 AND produto_id = $2",
                )
                .bind(tenant_id)
                .bind(produto_id)
                .fetch_optional(&self.pool)
                .await
                {
                    Ok(Some(custo)) => custo,
                    Ok(None) => {
                        tracing::error!(
                            tenant_id = %tenant_id,
                            venda_id,
                            produto_id = %produto_id,
                            quantidade = item.quantidade,
                            "produto sem saldo em proj_saldo_estoque — reentrada da devolução PULADA; \
                             registre a entrada manualmente em Estoque com o custo correto"
                        );
                        continue;
                    }
                    Err(err) => {
                        tracing::error!(
                            tenant_id = %tenant_id,
                            venda_id,
                            produto_id = %produto_id,
                            quantidade = item.quantidade,
                            error = %err,
                            "falha ao consultar custo médio — reentrada da devolução PULADA; \
                             registre a entrada manualmente em Estoque com o custo correto"
                        );
                        continue;
                    }
                };

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
                    tracing::error!(
                        tenant_id = %tenant_id,
                        venda_id,
                        produto_id = %produto_id,
                        quantidade = item.quantidade,
                        error = %err,
                        "falha ao reentrar estoque de item devolvido — registre a entrada manualmente em Estoque"
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
            let tenant_id = crate::shared::tenant::current_tenant_id().ok();
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
                    // A venda já está persistida — a baixa não desfaz. Erro alto
                    // e com todos os dados para permitir o ajuste manual do saldo.
                    tracing::error!(
                        tenant_id = ?tenant_id,
                        venda_id,
                        produto_id = %produto_id,
                        quantidade = item.quantidade,
                        error = %err,
                        "falha ao baixar estoque para venda confirmada — o saldo NÃO foi \
                         decrementado; ajuste manualmente o estoque deste produto"
                    );
                }
            }
        }
        Ok(())
    }
}
