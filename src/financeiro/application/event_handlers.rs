//! Handlers cross-BC do financeiro (VendaConfirmada/ItensDevolvidos → CR,
//! MercadoriaRecebida → CP).
//!
//! Limitação estrutural (issue #3): o EventBus é em memória, sem outbox
//! durável — um crash entre o commit do agregado de origem e a execução destes
//! handlers perde o efeito colateral, e a re-entrega é at-least-once. A
//! mitigação aqui é (a) ids determinísticos (UUID v5) + verificação de
//! existência, tornando o reprocessamento idempotente, e (b) nunca engolir
//! erros: toda falha sai em `tracing::error!` com tenant/ids/valores para
//! reconciliação manual. O outbox durável segue pendente na issue #3.

use std::convert::Infallible;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use pharos_app::EventHandler;
use uuid::Uuid;

use super::handler::{FinanceiroHandlers, ParcelaReceber};
use crate::compras::domain::events::ComprasEvent;
use crate::shared::Dinheiro;
use crate::shared::tenant::current_tenant_id;
use crate::vendas::domain::events::VendaEvent;
use crate::vendas::domain::value_objects::FormaPagamento;

/// Uuid do tenant corrente só para log (os repositórios já escopam sozinhos).
fn tenant_para_log() -> Uuid {
    current_tenant_id().unwrap_or_else(|_| Uuid::nil())
}

/// Parse de id vindo de evento: id inválido é bug/corrupção — loga e devolve
/// `None` para o handler pular o evento (nunca prosseguir com UUID nulo).
fn parse_id_evento(campo: &'static str, valor: &str) -> Option<Uuid> {
    match Uuid::parse_str(valor) {
        Ok(id) => Some(id),
        Err(err) => {
            tracing::error!(
                tenant_id = %tenant_para_log(),
                campo,
                valor,
                error = %err,
                "id inválido em evento cross-BC — evento ignorado pelo financeiro"
            );
            None
        }
    }
}

/// Traduz a forma de pagamento da venda no plano de parcelas do financeiro:
/// à vista → 1 conta já recebida; prazo → 1 conta com vencimento futuro;
/// cartão de crédito parcelado → N contas mensais (sobra de arredondamento na
/// última parcela).
fn planejar_parcelas(
    venda_id: Uuid,
    total_centavos: i64,
    forma: &FormaPagamento,
    agora: DateTime<Utc>,
) -> Vec<ParcelaReceber> {
    match forma {
        FormaPagamento::Prazo { dias } => vec![ParcelaReceber {
            indice: 0,
            valor: Dinheiro::from_centavos(total_centavos),
            vencimento: agora + Duration::days(*dias as i64),
            descricao: None,
            liquidar_imediatamente: false,
        }],
        FormaPagamento::CartaoCredito { parcelas } if *parcelas > 1 => {
            let n = *parcelas as i64;
            let base = total_centavos / n;
            (0..*parcelas as u32)
                .map(|i| {
                    let ultima = i == *parcelas as u32 - 1;
                    ParcelaReceber {
                        indice: i,
                        valor: Dinheiro::from_centavos(if ultima {
                            total_centavos - base * (n - 1)
                        } else {
                            base
                        }),
                        vencimento: agora + Duration::days(30 * (i as i64 + 1)),
                        descricao: Some(format!(
                            "Parcela {}/{} — venda {venda_id}",
                            i + 1,
                            parcelas
                        )),
                        liquidar_imediatamente: false,
                    }
                })
                .collect()
        }
        // Dinheiro, Pix, débito e crédito à vista: o dinheiro entrou no ato.
        FormaPagamento::Dinheiro
        | FormaPagamento::Pix
        | FormaPagamento::CartaoDebito
        | FormaPagamento::CartaoCredito { .. } => vec![ParcelaReceber {
            indice: 0,
            valor: Dinheiro::from_centavos(total_centavos),
            vencimento: agora,
            descricao: None,
            liquidar_imediatamente: true,
        }],
    }
}

/// Reage aos eventos de venda: cria as contas a receber na confirmação e
/// estorna/abate/reembolsa na devolução.
pub struct FinanceiroVendaEventHandler {
    pub financeiro: Arc<FinanceiroHandlers>,
}

impl EventHandler<VendaEvent> for FinanceiroVendaEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        match event {
            VendaEvent::VendaConfirmada {
                venda_id,
                cliente_id,
                total_centavos,
                forma_pagamento,
                ..
            } => {
                let Some(venda_uuid) = parse_id_evento("venda_id", venda_id) else {
                    return Ok(());
                };
                let cliente_uuid = cliente_id
                    .as_deref()
                    .and_then(|s| parse_id_evento("cliente_id", s));

                let parcelas =
                    planejar_parcelas(venda_uuid, *total_centavos, forma_pagamento, Utc::now());
                if let Err(err) = self
                    .financeiro
                    .criar_contas_receber_da_venda(venda_uuid, cliente_uuid, parcelas)
                    .await
                {
                    tracing::error!(
                        tenant_id = %tenant_para_log(),
                        venda_id,
                        total_centavos,
                        forma_pagamento = %forma_pagamento,
                        error = %err,
                        "falha ao criar contas a receber da venda confirmada"
                    );
                }
            }
            VendaEvent::ItensDevolvidos {
                venda_id,
                cliente_id,
                total_devolvido_centavos,
                devolucao_total,
                motivo,
                occurred_at,
                ..
            } => {
                let Some(venda_uuid) = parse_id_evento("venda_id", venda_id) else {
                    return Ok(());
                };
                let cliente_uuid = cliente_id
                    .as_deref()
                    .and_then(|s| parse_id_evento("cliente_id", s));

                let resultado = if *devolucao_total {
                    self.financeiro
                        .processar_devolucao_total(venda_uuid, cliente_uuid, motivo)
                        .await
                } else {
                    self.financeiro
                        .processar_devolucao_parcial(
                            venda_uuid,
                            cliente_uuid,
                            Dinheiro::from_centavos(*total_devolvido_centavos),
                            motivo,
                            *occurred_at,
                        )
                        .await
                };
                if let Err(err) = resultado {
                    tracing::error!(
                        tenant_id = %tenant_para_log(),
                        venda_id,
                        total_devolvido_centavos,
                        devolucao_total,
                        error = %err,
                        "falha ao ajustar o financeiro após devolução de itens"
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Cria uma ContaPagar quando MercadoriaRecebida é publicada no EventBus (BC Compras).
pub struct FinanceiroComprasEventHandler {
    pub financeiro: Arc<FinanceiroHandlers>,
}

impl EventHandler<ComprasEvent> for FinanceiroComprasEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &ComprasEvent) -> Result<(), Infallible> {
        if let ComprasEvent::MercadoriaRecebida {
            pedido_id,
            recebimento_id,
            fornecedor_id,
            total_centavos,
            prazo_pagamento_dias,
            ..
        } = event
        {
            let Some(pedido_uuid) = parse_id_evento("pedido_id", pedido_id) else {
                return Ok(());
            };
            let Some(fornecedor_uuid) = parse_id_evento("fornecedor_id", fornecedor_id) else {
                return Ok(());
            };
            let vencimento = Utc::now() + Duration::days(*prazo_pagamento_dias as i64);

            if let Err(err) = self
                .financeiro
                .criar_conta_pagar_do_recebimento(
                    pedido_uuid,
                    recebimento_id,
                    fornecedor_uuid,
                    Dinheiro::from_centavos(*total_centavos),
                    vencimento,
                )
                .await
            {
                tracing::error!(
                    tenant_id = %tenant_para_log(),
                    pedido_id,
                    recebimento_id,
                    total_centavos,
                    error = %err,
                    "falha ao criar conta a pagar do recebimento de mercadoria"
                );
            }
        }
        Ok(())
    }
}
