use chrono::Utc;
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::{ItemVendaSnapshot, VendaEvent};
use super::value_objects::{FormaPagamento, StatusVenda};
use crate::estoque::domain::Disponibilidade;
use crate::shared::{Dinheiro, Quantidade};

id_type!(VendaId);

// `quantidade`/`preco_unitario_centavos` são privados de propósito: o único
// jeito de obter um `ItemVenda` é via `Venda::adicionar_item` (que valida
// através de `Quantidade`/`Dinheiro`) ou a mutação em `devolver_itens` — não
// há como montar um item inválido via struct literal de fora deste módulo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemVenda {
    item_id: Uuid,
    produto_id: Uuid,
    sku: String,
    descricao: String,
    quantidade: u32,
    preco_unitario_centavos: i64,
}

impl ItemVenda {
    pub fn subtotal(&self) -> i64 {
        self.preco_unitario_centavos * self.quantidade as i64
    }
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Venda {
    #[id]
    id: VendaId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<VendaEvent>,
    vendedor_id: Uuid,
    cliente_id: Option<Uuid>,
    itens: Vec<ItemVenda>,
    /// Desconto global da venda (herdado do orçamento na conversão).
    /// `#[serde(default)]`: snapshots persistidos antes do campo deserializam
    /// com desconto zero — comportamento idêntico ao anterior.
    #[serde(default)]
    desconto_centavos: i64,
    forma_pagamento: Option<FormaPagamento>,
    status: StatusVenda,
}

impl Venda {
    pub fn iniciar(vendedor_id: Uuid, cliente_id: Option<Uuid>) -> Self {
        let id = VendaId::new();
        let mut events = AggregateEvents::default();
        events.raise(VendaEvent::VendaIniciada {
            venda_id: id.to_string(),
            vendedor_id: vendedor_id.to_string(),
            cliente_id: cliente_id.map(|c| c.to_string()),
            occurred_at: Utc::now(),
        });
        Self {
            id,
            version: 0,
            events,
            vendedor_id,
            cliente_id,
            itens: vec![],
            desconto_centavos: 0,
            forma_pagamento: None,
            status: StatusVenda::EmAndamento,
        }
    }

    #[allow(clippy::too_many_arguments)] // flat args mirram o payload do comando
    pub fn adicionar_item(
        &mut self,
        produto_id: Uuid,
        sku: String,
        descricao: String,
        quantidade: u32,
        preco_unitario: Dinheiro,
        disponibilidade: Disponibilidade,
    ) -> DomainResult<Uuid> {
        self.garantir_em_andamento()?;
        let quantidade = Quantidade::try_from(quantidade)?;
        if preco_unitario.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Preço unitário deve ser positivo".into(),
            ));
        }

        let ja_no_documento: u32 = self
            .itens
            .iter()
            .filter(|i| i.produto_id == produto_id)
            .map(|i| i.quantidade)
            .sum();
        disponibilidade.validar(ja_no_documento, quantidade.get())?;

        let item_id = Uuid::now_v7();
        self.itens.push(ItemVenda {
            item_id,
            produto_id,
            sku: sku.clone(),
            descricao: descricao.clone(),
            quantidade: quantidade.get(),
            preco_unitario_centavos: preco_unitario.centavos(),
        });
        self.events.raise(VendaEvent::ItemAdicionado {
            venda_id: self.id.to_string(),
            item_id: item_id.to_string(),
            produto_id: produto_id.to_string(),
            sku,
            descricao,
            quantidade: quantidade.get(),
            preco_unitario_centavos: preco_unitario.centavos(),
            occurred_at: Utc::now(),
        });
        Ok(item_id)
    }

    pub fn remover_item(&mut self, item_id: Uuid) -> DomainResult<()> {
        self.garantir_em_andamento()?;
        let pos = self
            .itens
            .iter()
            .position(|i| i.item_id == item_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Item {item_id} não encontrado na venda"))
            })?;
        self.itens.remove(pos);
        self.events.raise(VendaEvent::ItemRemovido {
            venda_id: self.id.to_string(),
            item_id: item_id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    /// Aplica um desconto global sobre a venda (só EmAndamento). O total
    /// cobrado passa a ser bruto dos itens − desconto.
    pub fn aplicar_desconto(&mut self, desconto_centavos: i64) -> DomainResult<()> {
        self.garantir_em_andamento()?;
        if desconto_centavos < 0 || desconto_centavos > self.total_bruto() {
            return Err(DomainError::Validation(
                "Desconto inválido: deve ser entre zero e o total dos itens da venda".into(),
            ));
        }
        self.desconto_centavos = desconto_centavos;
        self.events.raise(VendaEvent::DescontoVendaAplicado {
            venda_id: self.id.to_string(),
            desconto_centavos,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn definir_forma_pagamento(&mut self, forma: FormaPagamento) -> DomainResult<()> {
        self.garantir_em_andamento()?;
        self.events.raise(VendaEvent::FormaPagamentoDefinida {
            venda_id: self.id.to_string(),
            forma: forma.clone(),
            occurred_at: Utc::now(),
        });
        self.forma_pagamento = Some(forma);
        Ok(())
    }

    pub fn atualizar(&mut self, cliente_id: Option<Uuid>) -> DomainResult<()> {
        self.garantir_em_andamento()?;
        self.cliente_id = cliente_id;
        self.events.raise(VendaEvent::VendaAtualizada {
            venda_id: self.id.to_string(),
            cliente_id: cliente_id.map(|c| c.to_string()),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn confirmar(&mut self) -> DomainResult<()> {
        self.garantir_em_andamento()?;
        if self.itens.is_empty() {
            return Err(DomainError::BusinessRule(
                "Venda não pode ser confirmada sem itens".into(),
            ));
        }
        let forma = self
            .forma_pagamento
            .clone()
            .ok_or_else(|| DomainError::BusinessRule("Forma de pagamento não definida".into()))?;
        // Itens podem ter sido removidos depois do desconto aplicado — o total
        // líquido nunca pode ficar negativo.
        if self.desconto_centavos > self.total_bruto() {
            return Err(DomainError::BusinessRule(
                "Desconto excede o total dos itens — ajuste o desconto antes de confirmar".into(),
            ));
        }

        let total = self.total_centavos();
        let snapshots: Vec<ItemVendaSnapshot> = self
            .itens
            .iter()
            .map(|i| ItemVendaSnapshot {
                item_id: i.item_id.to_string(),
                produto_id: i.produto_id.to_string(),
                sku: i.sku.clone(),
                descricao: i.descricao.clone(),
                quantidade: i.quantidade,
                preco_unitario_centavos: i.preco_unitario_centavos,
            })
            .collect();

        self.status = StatusVenda::Confirmada;
        self.events.raise(VendaEvent::VendaConfirmada {
            venda_id: self.id.to_string(),
            vendedor_id: self.vendedor_id.to_string(),
            cliente_id: self.cliente_id.map(|c| c.to_string()),
            itens: snapshots,
            total_centavos: total,
            desconto_centavos: self.desconto_centavos,
            forma_pagamento: forma,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn cancelar(&mut self, motivo: String) -> DomainResult<()> {
        if self.status == StatusVenda::Cancelada {
            return Err(DomainError::BusinessRule("Venda já está cancelada".into()));
        }
        if self.status == StatusVenda::Confirmada {
            return Err(DomainError::BusinessRule(
                "Venda confirmada não pode ser cancelada diretamente — cancele a nota fiscal primeiro".into(),
            ));
        }
        self.status = StatusVenda::Cancelada;
        self.events.raise(VendaEvent::VendaCancelada {
            venda_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    /// Devolve itens de uma venda confirmada. Quantidade parcial por item é
    /// permitida; devolver tudo desfaz a venda (status Cancelada). O estoque,
    /// o financeiro e o fiscal reagem ao evento no bus — ver bootstrap/events.rs.
    pub fn devolver_itens(
        &mut self,
        devolucoes: &[(Uuid, u32)],
        motivo: String,
    ) -> DomainResult<()> {
        if self.status != StatusVenda::Confirmada {
            return Err(DomainError::BusinessRule(
                "Apenas vendas confirmadas aceitam devolução".into(),
            ));
        }
        if motivo.trim().is_empty() {
            return Err(DomainError::Validation(
                "Motivo é obrigatório para devolução".into(),
            ));
        }
        let efetivas: Vec<&(Uuid, u32)> = devolucoes.iter().filter(|(_, q)| *q > 0).collect();
        if efetivas.is_empty() {
            return Err(DomainError::Validation(
                "Informe ao menos um item com quantidade a devolver".into(),
            ));
        }

        let mut devolvidos: Vec<ItemVendaSnapshot> = Vec::with_capacity(efetivas.len());
        for (item_id, quantidade) in &efetivas {
            let item = self
                .itens
                .iter()
                .find(|i| i.item_id == *item_id)
                .ok_or_else(|| {
                    DomainError::NotFound(format!("Item {item_id} não encontrado na venda"))
                })?;
            if *quantidade > item.quantidade {
                return Err(DomainError::BusinessRule(format!(
                    "Item {}: devolução de {} unidades excede as {} vendidas",
                    item.sku, quantidade, item.quantidade
                )));
            }
            devolvidos.push(ItemVendaSnapshot {
                item_id: item.item_id.to_string(),
                produto_id: item.produto_id.to_string(),
                sku: item.sku.clone(),
                descricao: item.descricao.clone(),
                quantidade: *quantidade,
                preco_unitario_centavos: item.preco_unitario_centavos,
            });
        }

        // Aplica as devoluções: reduz quantidades e remove itens zerados.
        for (item_id, quantidade) in &efetivas {
            if let Some(item) = self.itens.iter_mut().find(|i| i.item_id == *item_id) {
                item.quantidade -= quantidade;
            }
        }
        self.itens.retain(|i| i.quantidade > 0);

        let restantes: Vec<ItemVendaSnapshot> = self
            .itens
            .iter()
            .map(|i| ItemVendaSnapshot {
                item_id: i.item_id.to_string(),
                produto_id: i.produto_id.to_string(),
                sku: i.sku.clone(),
                descricao: i.descricao.clone(),
                quantidade: i.quantidade,
                preco_unitario_centavos: i.preco_unitario_centavos,
            })
            .collect();
        let devolucao_total = restantes.is_empty();
        let total_devolvido: i64 = devolvidos
            .iter()
            .map(|i| i.preco_unitario_centavos * i.quantidade as i64)
            .sum();

        self.events.raise(VendaEvent::ItensDevolvidos {
            venda_id: self.id.to_string(),
            cliente_id: self.cliente_id.map(|c| c.to_string()),
            itens_devolvidos: devolvidos,
            itens_restantes: restantes,
            total_devolvido_centavos: total_devolvido,
            devolucao_total,
            motivo: motivo.clone(),
            occurred_at: Utc::now(),
        });

        if devolucao_total {
            self.status = StatusVenda::Cancelada;
            self.events.raise(VendaEvent::VendaCancelada {
                venda_id: self.id.to_string(),
                motivo: format!("Devolução total: {motivo}"),
                occurred_at: Utc::now(),
            });
        }
        Ok(())
    }

    /// Soma bruta dos subtotais dos itens, sem desconto.
    pub fn total_bruto(&self) -> i64 {
        self.itens.iter().map(|i| i.subtotal()).sum()
    }

    /// Total líquido cobrado: bruto − desconto global.
    pub fn total_centavos(&self) -> i64 {
        self.total_bruto() - self.desconto_centavos
    }

    fn garantir_em_andamento(&self) -> DomainResult<()> {
        if self.status != StatusVenda::EmAndamento {
            return Err(DomainError::BusinessRule(format!(
                "Venda não pode ser modificada no status {:?}",
                self.status
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Dinheiro;

    fn venda_com_item() -> Venda {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        v.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Pastilha".into(),
            1,
            Dinheiro::from_centavos(8000),
            Disponibilidade::NaoControlada,
        )
        .expect("adicionar item");
        v.definir_forma_pagamento(FormaPagamento::Dinheiro)
            .expect("forma");
        v
    }

    #[test]
    fn confirmar_sem_itens_retorna_erro() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        v.definir_forma_pagamento(FormaPagamento::Dinheiro)
            .expect("forma");
        assert!(matches!(v.confirmar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn confirmar_sem_forma_pagamento_retorna_erro() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        v.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Pastilha".into(),
            1,
            Dinheiro::from_centavos(8000),
            Disponibilidade::NaoControlada,
        )
        .expect("adicionar item");
        assert!(matches!(v.confirmar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn confirmar_venda_valida_muda_status() {
        let mut v = venda_com_item();
        v.confirmar().expect("confirmar");
        assert_eq!(v.status, StatusVenda::Confirmada);
    }

    #[test]
    fn adicionar_item_quantidade_zero_retorna_erro() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let err = v.adicionar_item(
            Uuid::new_v4(),
            "SKU".into(),
            "X".into(),
            0,
            Dinheiro::from_centavos(100),
            Disponibilidade::NaoControlada,
        );
        assert!(matches!(err, Err(DomainError::Validation(_))));
    }

    #[test]
    fn cancelar_venda_confirmada_retorna_erro() {
        let mut v = venda_com_item();
        v.confirmar().expect("confirmar");
        assert!(matches!(
            v.cancelar("motivo".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn devolucao_parcial_reduz_quantidade_e_mantem_confirmada() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let item = v
            .adicionar_item(Uuid::new_v4(), "A".into(), "A".into(), 3, Dinheiro::from_centavos(1000), Disponibilidade::NaoControlada)
            .expect("item");
        v.definir_forma_pagamento(FormaPagamento::Dinheiro).expect("forma");
        v.confirmar().expect("confirmar");

        v.devolver_itens(&[(item, 2)], "defeito".into()).expect("devolver");
        assert_eq!(v.status, StatusVenda::Confirmada);
        assert_eq!(v.itens[0].quantidade, 1);
        assert_eq!(v.total_centavos(), 1000);
    }

    #[test]
    fn devolucao_total_desfaz_a_venda() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let item = v
            .adicionar_item(Uuid::new_v4(), "A".into(), "A".into(), 2, Dinheiro::from_centavos(1000), Disponibilidade::NaoControlada)
            .expect("item");
        v.definir_forma_pagamento(FormaPagamento::Dinheiro).expect("forma");
        v.confirmar().expect("confirmar");

        v.devolver_itens(&[(item, 2)], "arrependimento".into()).expect("devolver");
        assert_eq!(v.status, StatusVenda::Cancelada);
        assert!(v.itens.is_empty());
    }

    #[test]
    fn devolucao_acima_do_vendido_retorna_erro() {
        let mut v = venda_com_item();
        let item = v.itens[0].item_id;
        v.confirmar().expect("confirmar");
        assert!(matches!(
            v.devolver_itens(&[(item, 5)], "x".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn devolucao_de_venda_nao_confirmada_retorna_erro() {
        let mut v = venda_com_item();
        let item = v.itens[0].item_id;
        assert!(matches!(
            v.devolver_itens(&[(item, 1)], "x".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn total_centavos_soma_todos_os_itens() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        v.adicionar_item(
            Uuid::new_v4(),
            "A".into(),
            "A".into(),
            2,
            Dinheiro::from_centavos(1000),
            Disponibilidade::NaoControlada,
        )
        .expect("item 1");
        v.adicionar_item(
            Uuid::new_v4(),
            "B".into(),
            "B".into(),
            3,
            Dinheiro::from_centavos(500),
            Disponibilidade::NaoControlada,
        )
        .expect("item 2");
        assert_eq!(v.total_centavos(), 3500);
    }

    #[test]
    fn aplicar_desconto_reduz_o_total_liquido() {
        let mut v = venda_com_item(); // 1 × 8000
        v.aplicar_desconto(500).expect("desconto");
        assert_eq!(v.total_bruto(), 8000);
        assert_eq!(v.total_centavos(), 7500);
    }

    #[test]
    fn aplicar_desconto_acima_do_total_bruto_retorna_erro() {
        let mut v = venda_com_item(); // 1 × 8000
        assert!(matches!(
            v.aplicar_desconto(8001),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn aplicar_desconto_negativo_retorna_erro() {
        let mut v = venda_com_item();
        assert!(matches!(
            v.aplicar_desconto(-1),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn aplicar_desconto_em_venda_confirmada_retorna_erro() {
        let mut v = venda_com_item();
        v.confirmar().expect("confirmar");
        assert!(matches!(
            v.aplicar_desconto(100),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn confirmar_com_desconto_emite_total_liquido_no_evento() {
        let mut v = venda_com_item(); // 1 × 8000
        v.aplicar_desconto(1000).expect("desconto");
        v.confirmar().expect("confirmar");
        let confirmada = v.events.drain().into_iter().find_map(|e| match e {
            VendaEvent::VendaConfirmada {
                total_centavos,
                desconto_centavos,
                ..
            } => Some((total_centavos, desconto_centavos)),
            _ => None,
        });
        assert_eq!(confirmada, Some((7000, 1000)));
    }

    #[test]
    fn confirmar_com_desconto_maior_que_o_bruto_apos_remocao_retorna_erro() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let item_caro = v
            .adicionar_item(
                Uuid::new_v4(),
                "A".into(),
                "A".into(),
                1,
                Dinheiro::from_centavos(10_000),
                Disponibilidade::NaoControlada,
            )
            .expect("item caro");
        v.adicionar_item(
            Uuid::new_v4(),
            "B".into(),
            "B".into(),
            1,
            Dinheiro::from_centavos(1_000),
            Disponibilidade::NaoControlada,
        )
        .expect("item barato");
        v.definir_forma_pagamento(FormaPagamento::Dinheiro)
            .expect("forma");
        v.aplicar_desconto(5_000).expect("desconto válido na hora");
        v.remover_item(item_caro).expect("remover");
        assert!(matches!(v.confirmar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn adicionar_item_acima_do_saldo_controlado_retorna_erro() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let err = v.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Pastilha".into(),
            5,
            Dinheiro::from_centavos(8000),
            Disponibilidade::Controlada(3),
        );
        assert!(matches!(err, Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn adicionar_item_soma_quantidade_ja_no_documento_para_o_mesmo_produto() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        let produto_id = Uuid::new_v4();
        v.adicionar_item(
            produto_id,
            "SKU-1".into(),
            "Pastilha".into(),
            2,
            Dinheiro::from_centavos(8000),
            Disponibilidade::Controlada(3),
        )
        .expect("primeira adição cabe no saldo");
        let err = v.adicionar_item(
            produto_id,
            "SKU-1".into(),
            "Pastilha".into(),
            2,
            Dinheiro::from_centavos(8000),
            Disponibilidade::Controlada(3),
        );
        assert!(
            matches!(err, Err(DomainError::BusinessRule(_))),
            "2 + 2 = 4 unidades excede o saldo de 3"
        );
    }

    #[test]
    fn adicionar_item_sem_checagem_ignora_saldo() {
        let mut v = Venda::iniciar(Uuid::new_v4(), None);
        v.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Pastilha".into(),
            999,
            Dinheiro::from_centavos(8000),
            Disponibilidade::SemChecagem,
        )
        .expect("venda sob encomenda ignora o saldo");
    }
}
