use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::{ComprasEvent, ItemPedidoSnapshot};
use crate::shared::{Dinheiro, Quantidade};

id_type!(PedidoCompraId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusPedido {
    Gerado,
    Aprovado,
    Enviado,
    RecebidoParcial,
    RecebidoTotal,
    Cancelado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPedido {
    produto_id: Uuid,
    quantidade_pedida: u32,
    quantidade_recebida: u32,
    custo_unitario_centavos: i64,
}

impl ItemPedido {
    pub fn pendente(&self) -> u32 {
        self.quantidade_pedida
            .saturating_sub(self.quantidade_recebida)
    }
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct PedidoCompra {
    #[id]
    id: PedidoCompraId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<ComprasEvent>,
    comprador_id: Uuid,
    fornecedor_id: Uuid,
    itens: Vec<ItemPedido>,
    prazo_pagamento_dias: u16,
    status: StatusPedido,
}

impl PedidoCompra {
    pub fn gerar(
        comprador_id: Uuid,
        fornecedor_id: Uuid,
        itens: Vec<(Uuid, u32, Dinheiro)>,
        prazo_pagamento_dias: u16,
    ) -> DomainResult<Self> {
        if itens.is_empty() {
            return Err(DomainError::BusinessRule(
                "Pedido deve ter ao menos um item".into(),
            ));
        }
        for (_, qty, custo) in &itens {
            Quantidade::try_from(*qty)?;
            if custo.centavos() <= 0 {
                return Err(DomainError::Validation(
                    "Custo unitário deve ser positivo".into(),
                ));
            }
        }

        let id = PedidoCompraId::new();
        let snapshots: Vec<ItemPedidoSnapshot> = itens
            .iter()
            .map(|(pid, qty, custo)| ItemPedidoSnapshot {
                produto_id: pid.to_string(),
                quantidade: *qty,
                custo_unitario_centavos: custo.centavos(),
            })
            .collect();

        let mut events = AggregateEvents::default();
        events.raise(ComprasEvent::PedidoCompraGerado {
            pedido_id: id.to_string(),
            comprador_id: comprador_id.to_string(),
            fornecedor_id: fornecedor_id.to_string(),
            itens: snapshots,
            prazo_pagamento_dias,
            occurred_at: Utc::now(),
        });

        let itens_struct: Vec<ItemPedido> = itens
            .into_iter()
            .map(|(produto_id, quantidade_pedida, custo)| ItemPedido {
                produto_id,
                quantidade_pedida,
                quantidade_recebida: 0,
                custo_unitario_centavos: custo.centavos(),
            })
            .collect();

        Ok(Self {
            id,
            version: 0,
            events,
            comprador_id,
            fornecedor_id,
            itens: itens_struct,
            prazo_pagamento_dias,
            status: StatusPedido::Gerado,
        })
    }

    pub fn aprovar(&mut self, aprovador_id: Uuid) -> DomainResult<()> {
        if self.status != StatusPedido::Gerado {
            return Err(DomainError::BusinessRule(
                "Apenas pedidos gerados podem ser aprovados".into(),
            ));
        }
        self.status = StatusPedido::Aprovado;
        self.events.raise(ComprasEvent::PedidoCompraAprovado {
            pedido_id: self.id.to_string(),
            aprovador_id: aprovador_id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn enviar(&mut self) -> DomainResult<()> {
        if self.status != StatusPedido::Aprovado {
            return Err(DomainError::BusinessRule(
                "Apenas pedidos aprovados podem ser enviados".into(),
            ));
        }
        self.status = StatusPedido::Enviado;
        self.events.raise(ComprasEvent::PedidoCompraEnviado {
            pedido_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    /// Registra o recebimento (total ou parcial) das mercadorias.
    pub fn receber_mercadoria(&mut self, itens_recebidos: Vec<(Uuid, u32)>) -> DomainResult<()> {
        if !matches!(
            self.status,
            StatusPedido::Enviado | StatusPedido::RecebidoParcial
        ) {
            return Err(DomainError::BusinessRule(
                "Mercadoria só pode ser recebida para pedidos enviados ou parcialmente recebidos"
                    .into(),
            ));
        }

        let recebimento_id = Uuid::now_v7().to_string();
        let mut snapshots = Vec::new();
        let mut total_centavos: i64 = 0;

        for (produto_id, qty_recebida) in &itens_recebidos {
            let item = self
                .itens
                .iter_mut()
                .find(|i| i.produto_id == *produto_id)
                .ok_or_else(|| {
                    DomainError::NotFound(format!(
                        "Produto {} não encontrado no pedido",
                        produto_id
                    ))
                })?;
            Quantidade::try_from(*qty_recebida)?;
            if item.quantidade_recebida + qty_recebida > item.quantidade_pedida {
                return Err(DomainError::BusinessRule(format!(
                    "Quantidade recebida ({}) excede o saldo pendente ({})",
                    qty_recebida,
                    item.pendente()
                )));
            }
            item.quantidade_recebida += qty_recebida;
            total_centavos += item.custo_unitario_centavos * *qty_recebida as i64;
            snapshots.push(ItemPedidoSnapshot {
                produto_id: produto_id.to_string(),
                quantidade: *qty_recebida,
                custo_unitario_centavos: item.custo_unitario_centavos,
            });
        }

        let tudo_recebido = self.itens.iter().all(|i| i.pendente() == 0);
        self.status = if tudo_recebido {
            StatusPedido::RecebidoTotal
        } else {
            StatusPedido::RecebidoParcial
        };

        self.events.raise(ComprasEvent::MercadoriaRecebida {
            pedido_id: self.id.to_string(),
            recebimento_id,
            fornecedor_id: self.fornecedor_id.to_string(),
            itens: snapshots,
            total_centavos,
            prazo_pagamento_dias: self.prazo_pagamento_dias,
            tudo_recebido,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn cancelar(&mut self, motivo: String) -> DomainResult<()> {
        if matches!(
            self.status,
            StatusPedido::RecebidoParcial | StatusPedido::RecebidoTotal | StatusPedido::Cancelado
        ) {
            return Err(DomainError::BusinessRule(
                "Pedido não pode ser cancelado no estado atual".into(),
            ));
        }
        self.status = StatusPedido::Cancelado;
        self.events.raise(ComprasEvent::PedidoCancelado {
            pedido_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Dinheiro;

    fn pedido() -> PedidoCompra {
        PedidoCompra::gerar(
            Uuid::new_v4(),
            Uuid::new_v4(),
            vec![(Uuid::new_v4(), 10, Dinheiro::from_centavos(500))],
            30,
        )
        .expect("pedido válido")
    }

    #[test]
    fn gerar_sem_itens_retorna_erro() {
        let err = PedidoCompra::gerar(Uuid::new_v4(), Uuid::new_v4(), vec![], 30);
        assert!(matches!(err, Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn aprovar_pedido_gerado_muda_status() {
        let mut p = pedido();
        p.aprovar(Uuid::new_v4()).expect("aprovar");
        assert_eq!(p.status, StatusPedido::Aprovado);
    }

    #[test]
    fn cancelar_pedido_recebido_retorna_erro() {
        let produto_id = Uuid::new_v4();
        let mut p = PedidoCompra::gerar(
            Uuid::new_v4(),
            Uuid::new_v4(),
            vec![(produto_id, 2, Dinheiro::from_centavos(1000))],
            30,
        )
        .expect("pedido");
        p.aprovar(Uuid::new_v4()).expect("aprovar");
        p.enviar().expect("enviar");
        p.receber_mercadoria(vec![(produto_id, 2)])
            .expect("receber");
        assert!(matches!(
            p.cancelar("motivo".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn enviar_sem_aprovacao_retorna_erro() {
        let mut p = pedido();
        assert!(matches!(p.enviar(), Err(DomainError::BusinessRule(_))));
    }
}
