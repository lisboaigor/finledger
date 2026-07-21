use std::convert::Infallible;
use std::sync::Arc;

use chrono::{Duration, Utc};
use pharos_app::EventHandler;

use super::handler::FinanceiroHandlers;
use crate::compras::domain::events::ComprasEvent;
use crate::financeiro::domain::conta_pagar::ContaPagar;
use crate::financeiro::domain::conta_receber::ContaReceber;
use crate::shared::Dinheiro;
use crate::vendas::domain::events::VendaEvent;

/// Cria uma ContaReceber quando uma VendaConfirmada é publicada no EventBus.
pub struct FinanceiroVendaEventHandler {
    pub financeiro: Arc<FinanceiroHandlers>,
}

impl EventHandler<VendaEvent> for FinanceiroVendaEventHandler {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        if let VendaEvent::VendaConfirmada {
            venda_id,
            cliente_id,
            total_centavos,
            forma_pagamento,
            ..
        } = event
        {
            use crate::vendas::domain::value_objects::FormaPagamento;

            // Traduzir o evento externo (FormaPagamento de vendas) para um
            // prazo em dias é a única parte que pertence aqui; o cálculo do
            // vencimento a partir do prazo é responsabilidade do domínio.
            let dias_prazo = match forma_pagamento {
                FormaPagamento::Prazo { dias } => *dias as i64,
                _ => 0,
            };
            let venda_uuid = uuid::Uuid::parse_str(venda_id).unwrap_or_default();
            let cliente_uuid = cliente_id
                .as_deref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok());

            let conta = ContaReceber::criar_a_prazo(
                venda_uuid,
                cliente_uuid,
                Dinheiro::from_centavos(*total_centavos),
                dias_prazo,
            );
            let _ = self.financeiro.criar_conta_receber(conta).await;
        }

        // Devolução TOTAL desfaz a venda: estorna as contas a receber em aberto.
        // Devolução parcial não ajusta a CR automaticamente (o valor original é
        // imutável no domínio) — o gestor negocia o abatimento na tela Financeiro.
        if let VendaEvent::ItensDevolvidos {
            venda_id,
            devolucao_total: true,
            motivo,
            ..
        } = event
        {
            let venda_uuid = uuid::Uuid::parse_str(venda_id).unwrap_or_default();
            if let Err(err) = self
                .financeiro
                .estornar_contas_da_venda(venda_uuid, format!("Devolução total: {motivo}"))
                .await
            {
                tracing::warn!(venda_id, error = %err, "falha ao estornar contas da venda devolvida");
            }
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
            fornecedor_id,
            total_centavos,
            prazo_pagamento_dias,
            ..
        } = event
        {
            let pedido_uuid = uuid::Uuid::parse_str(pedido_id).unwrap_or_default();
            let fornecedor_uuid = uuid::Uuid::parse_str(fornecedor_id).unwrap_or_default();
            let vencimento = Utc::now() + Duration::days(*prazo_pagamento_dias as i64);

            let conta = ContaPagar::criar(
                pedido_uuid,
                fornecedor_uuid,
                Dinheiro::from_centavos(*total_centavos),
                vencimento,
            );
            let _ = self.financeiro.criar_conta_pagar(conta).await;
        }
        Ok(())
    }
}
