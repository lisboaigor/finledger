use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::EstoqueEvent;
use crate::shared::{Dinheiro, Quantidade};

// id do ItemEstoque = produto_id (um registro por produto)
id_type!(ItemEstoqueId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct ItemEstoque {
    #[id]
    id: ItemEstoqueId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<EstoqueEvent>,
    produto_id: Uuid,
    // Privados: mutação só via `registrar_entrada`/`baixar`/`ajustar`/
    // `definir_estoque_minimo`, que emitem o evento correspondente — um
    // `&mut ItemEstoque` de fora não pode zerar o saldo sem passar por essas
    // checagens (ex: saldo insuficiente em `baixar`).
    saldo: u32,
    estoque_minimo: u32,
}

impl ItemEstoque {
    /// Cria o registro de estoque para um produto com saldo zero.
    pub fn criar(produto_id: Uuid, estoque_minimo: u32) -> Self {
        let id = ItemEstoqueId::from_uuid(produto_id);
        Self {
            id,
            version: 0,
            events: AggregateEvents::default(),
            produto_id,
            saldo: 0,
            estoque_minimo,
        }
    }

    pub fn registrar_entrada(
        &mut self,
        quantidade: u32,
        custo_unitario: Dinheiro,
        motivo: String,
        nota_fiscal: Option<String>,
    ) -> DomainResult<()> {
        let quantidade = Quantidade::try_from(quantidade)?;
        if custo_unitario.centavos() < 0 {
            return Err(DomainError::Validation(
                "Custo unitário não pode ser negativo".into(),
            ));
        }

        self.saldo = self.saldo.saturating_add(quantidade.get());
        self.events.raise(EstoqueEvent::EstoqueEntrada {
            item_id: self.id.to_string(),
            produto_id: self.produto_id.to_string(),
            quantidade: quantidade.get(),
            custo_unitario_centavos: custo_unitario.centavos(),
            motivo,
            nota_fiscal,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    /// Baixa o estoque diretamente (chamado na confirmação de venda).
    pub fn baixar(
        &mut self,
        quantidade: u32,
        motivo: String,
        referencia_id: Option<String>,
    ) -> DomainResult<()> {
        let quantidade = Quantidade::try_from(quantidade)?;
        if self.saldo < quantidade.get() {
            return Err(DomainError::BusinessRule(format!(
                "Estoque insuficiente: saldo {}, solicitado {}",
                self.saldo,
                quantidade.get()
            )));
        }

        self.saldo -= quantidade.get();
        self.events.raise(EstoqueEvent::EstoqueSaida {
            item_id: self.id.to_string(),
            produto_id: self.produto_id.to_string(),
            quantidade: quantidade.get(),
            motivo,
            referencia_id,
            occurred_at: Utc::now(),
        });

        if self.saldo <= self.estoque_minimo && self.estoque_minimo > 0 {
            self.events
                .raise(EstoqueEvent::EstoqueMinimoPadraoAtingido {
                    item_id: self.id.to_string(),
                    produto_id: self.produto_id.to_string(),
                    saldo_atual: self.saldo,
                    estoque_minimo: self.estoque_minimo,
                    occurred_at: Utc::now(),
                });
        }
        Ok(())
    }

    /// Ajusta o saldo para `quantidade_nova`. Um ajuste que **aumenta** o
    /// saldo exige `custo_unitario` (as unidades acrescentadas entram no custo
    /// médio); ajuste para baixo/igual ignora o custo.
    pub fn ajustar(
        &mut self,
        quantidade_nova: u32,
        custo_unitario: Option<Dinheiro>,
        justificativa: String,
    ) -> DomainResult<()> {
        if justificativa.trim().is_empty() {
            return Err(DomainError::Validation(
                "Justificativa é obrigatória para ajuste de estoque".into(),
            ));
        }
        let anterior = self.saldo;
        if quantidade_nova > anterior {
            match custo_unitario {
                None => {
                    return Err(DomainError::Validation(
                        "Custo unitário é obrigatório em ajuste que aumenta o saldo".into(),
                    ));
                }
                Some(c) if c.centavos() < 0 => {
                    return Err(DomainError::Validation(
                        "Custo unitário não pode ser negativo".into(),
                    ));
                }
                _ => {}
            }
        }
        self.saldo = quantidade_nova;
        self.events.raise(EstoqueEvent::AjusteEstoque {
            item_id: self.id.to_string(),
            quantidade_anterior: anterior,
            quantidade_nova,
            custo_unitario_centavos: custo_unitario.map(|c| c.centavos()),
            justificativa,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn definir_estoque_minimo(&mut self, estoque_minimo: u32) -> DomainResult<()> {
        if estoque_minimo == self.estoque_minimo {
            return Err(DomainError::BusinessRule(
                "Estoque mínimo informado é igual ao atual".into(),
            ));
        }
        self.estoque_minimo = estoque_minimo;
        self.events.raise(EstoqueEvent::EstoqueMinimoDefinido {
            item_id: self.id.to_string(),
            produto_id: self.produto_id.to_string(),
            estoque_minimo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn saldo_disponivel(&self) -> u32 {
        self.saldo
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Dinheiro;

    #[test]
    fn entrada_incrementa_saldo() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 5);
        item.registrar_entrada(10, Dinheiro::from_centavos(2000), "compra".into(), Some("NF-123".into()))
            .expect("entrada");
        assert_eq!(item.saldo, 10);
    }

    #[test]
    fn entrada_quantidade_zero_retorna_erro() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        assert!(matches!(
            item.registrar_entrada(0, Dinheiro::from_centavos(1000), "x".into(), None),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn baixar_decrementa_saldo() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        item.registrar_entrada(10, Dinheiro::from_centavos(100), "e".into(), None)
            .expect("entrada");
        item.baixar(3, "venda".into(), None).expect("baixar");
        assert_eq!(item.saldo, 7);
    }

    #[test]
    fn baixar_saldo_insuficiente_retorna_erro() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        item.registrar_entrada(5, Dinheiro::from_centavos(100), "e".into(), None)
            .expect("entrada");
        assert!(matches!(
            item.baixar(10, "venda".into(), None),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn ajuste_sem_justificativa_retorna_erro() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        assert!(matches!(
            item.ajustar(20, Some(Dinheiro::from_centavos(100)), "   ".into()),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn ajuste_para_cima_sem_custo_retorna_erro() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        assert!(matches!(
            item.ajustar(20, None, "contagem".into()),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn ajuste_para_cima_com_custo_ok() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        item.registrar_entrada(5, Dinheiro::from_centavos(100), "e".into(), None)
            .expect("entrada");
        item.ajustar(8, Some(Dinheiro::from_centavos(200)), "contagem".into())
            .expect("ajuste");
        assert_eq!(item.saldo, 8);
    }

    #[test]
    fn ajuste_para_baixo_dispensa_custo() {
        let mut item = ItemEstoque::criar(Uuid::new_v4(), 0);
        item.registrar_entrada(10, Dinheiro::from_centavos(100), "e".into(), None)
            .expect("entrada");
        item.ajustar(4, None, "perda".into()).expect("ajuste");
        assert_eq!(item.saldo, 4);
    }
}
