use std::convert::Infallible;
use std::sync::Arc;

use pharos_app::{CommandHandler, EventHandler};
use uuid::Uuid;

use super::commands::{AdicionarItemVenda, AplicarDescontoVenda, IniciarVenda};
use super::handler::VendasHandlers;
use crate::orcamentos::application::commands::MarcarConvertidoOrcamento;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::events::OrcamentoEvent;

/// Ao aceitar um orçamento, cria a venda correspondente EM ANDAMENTO
/// (pré-preenchida com os itens/preços/cliente/vendedor do orçamento) e marca o
/// orçamento como convertido, ligando-o à venda. O operador finaliza o
/// pagamento no PDV — a venda aparece na recuperação do terminal como
/// EmAndamento. A baixa de estoque/conta a receber/NF só dispara quando a venda
/// for confirmada; aqui nada disso acontece ainda.
///
/// Cliente avulso do orçamento não é transportado (a venda só modela
/// `cliente_id`); nesse caso a venda nasce como consumidor final e o operador
/// ajusta o cliente no PDV se quiser.
pub struct VendaAPartirDeOrcamentoHandler {
    pub vendas: Arc<VendasHandlers>,
    pub orcamentos: Arc<OrcamentosHandlers>,
}

impl EventHandler<OrcamentoEvent> for VendaAPartirDeOrcamentoHandler {
    type Error = Infallible;

    async fn handle(&self, event: &OrcamentoEvent) -> Result<(), Infallible> {
        let OrcamentoEvent::OrcamentoAceito {
            orcamento_id,
            itens,
            desconto_centavos,
            vendedor_id,
            cliente_id,
            ..
        } = event
        else {
            return Ok(());
        };

        let (Ok(orcamento_uuid), Ok(vendedor_uuid)) =
            (Uuid::parse_str(orcamento_id), Uuid::parse_str(vendedor_id))
        else {
            return Ok(());
        };
        let cliente_uuid = cliente_id.as_deref().and_then(|c| Uuid::parse_str(c).ok());

        // Cria a venda EmAndamento.
        let venda_id = match self
            .vendas
            .handle(IniciarVenda {
                vendedor_id: vendedor_uuid,
                cliente_id: cliente_uuid,
            })
            .await
        {
            Ok(id) => id.as_uuid(),
            Err(_) => return Ok(()),
        };

        // Replica os itens no preço acordado (`preservar_preco_informado`: o
        // handler não relê o catálogo — o preço do orçamento aceito prevalece).
        // `vender_sem_estoque: true` para a conversão nunca falhar por saldo —
        // a venda ainda é EmAndamento e não baixa estoque; a checagem real
        // ocorre na confirmação no PDV.
        for item in itens {
            let Ok(produto_uuid) = Uuid::parse_str(&item.produto_id) else {
                continue;
            };
            let _ = self
                .vendas
                .handle(AdicionarItemVenda {
                    venda_id,
                    produto_id: produto_uuid,
                    sku: item.sku.clone(),
                    descricao: item.descricao.clone(),
                    quantidade: item.quantidade,
                    preco_unitario_centavos: item.preco_unitario_centavos,
                    vender_sem_estoque: true,
                    preservar_preco_informado: true,
                })
                .await;
        }

        // Herda o desconto global do orçamento — sem isto a venda (e a conta a
        // receber/NF geradas na confirmação) cobraria o total BRUTO.
        if *desconto_centavos > 0
            && let Err(err) = self
                .vendas
                .handle(AplicarDescontoVenda {
                    venda_id,
                    desconto_centavos: *desconto_centavos,
                })
                .await
        {
            tracing::error!(
                %venda_id,
                orcamento_id,
                error = %err,
                "falha ao herdar o desconto do orçamento na venda"
            );
        }

        // Liga o orçamento à venda (status → convertido).
        let _ = self
            .orcamentos
            .handle(MarcarConvertidoOrcamento {
                orcamento_id: orcamento_uuid,
                venda_id,
            })
            .await;

        Ok(())
    }
}
