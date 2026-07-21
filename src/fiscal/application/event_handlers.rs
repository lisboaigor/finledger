use std::convert::Infallible;
use std::sync::Arc;

use pharos_app::EventHandler;

use super::handler::FiscalHandlers;
use crate::fiscal::infrastructure::sefaz::SefazClient;
use crate::vendas::domain::events::VendaEvent;

/// Dispara a emissão de NF-e / NFC-e quando uma venda é confirmada.
pub struct FiscalVendaEventHandler<S: SefazClient> {
    pub fiscal: Arc<FiscalHandlers<S>>,
}

impl<S: SefazClient> EventHandler<VendaEvent> for FiscalVendaEventHandler<S> {
    type Error = Infallible;

    async fn handle(&self, event: &VendaEvent) -> Result<(), Infallible> {
        if let VendaEvent::VendaConfirmada {
            venda_id,
            cliente_id,
            itens,
            ..
        } = event
        {
            let venda_uuid = match uuid::Uuid::parse_str(venda_id) {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!("FiscalVendaEventHandler: venda_id inválido {venda_id}: {e}");
                    return Ok(());
                }
            };
            let cliente_uuid = cliente_id
                .as_deref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok());

            if let Err(e) = self
                .fiscal
                .gerar_e_transmitir(venda_uuid, cliente_uuid, itens)
                .await
            {
                tracing::error!("Falha ao emitir NF para venda {venda_id}: {e:?}");
            }
        }

        if let VendaEvent::ItensDevolvidos {
            venda_id,
            cliente_id,
            itens_restantes,
            devolucao_total,
            motivo,
            ..
        } = event
        {
            let Ok(venda_uuid) = uuid::Uuid::parse_str(venda_id) else {
                return Ok(());
            };
            let cliente_uuid = cliente_id
                .as_deref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok());
            if let Err(e) = self
                .fiscal
                .processar_devolucao(
                    venda_uuid,
                    cliente_uuid,
                    itens_restantes,
                    *devolucao_total,
                    motivo,
                )
                .await
            {
                tracing::error!("Falha no tratamento fiscal da devolução da venda {venda_id}: {e:?}");
            }
        }
        Ok(())
    }
}
