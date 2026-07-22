use chrono::{DateTime, Duration, Utc};
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::{ItemOrcamentoSnapshot, OrcamentoEvent};
use super::identificacao_cliente::IdentificacaoCliente;
use crate::estoque::domain::Disponibilidade;
use crate::shared::{Dinheiro, Quantidade};

id_type!(OrcamentoId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusOrcamento {
    Rascunho,
    Emitido,
    Aceito,
    Recusado,
    Expirado,
    ConvertidoEmVenda,
    Cancelado,
}

// `quantidade`/`preco_unitario_centavos` privados: só se constrói um
// `ItemOrcamento` via `Orcamento::adicionar_item`, que valida através de
// `Quantidade`/`Dinheiro` — sem struct literal de fora deste módulo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOrcamento {
    pub item_id: Uuid,
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    quantidade: u32,
    preco_unitario_centavos: i64,
}

impl ItemOrcamento {
    pub fn subtotal(&self) -> i64 {
        self.preco_unitario_centavos * self.quantidade as i64
    }
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct Orcamento {
    #[id]
    id: OrcamentoId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<OrcamentoEvent>,
    pub vendedor_id: Uuid,
    /// Quem é o cliente deste orçamento — cadastro completo do CRM ou nome
    /// avulso de balcão. Ver [`IdentificacaoCliente`] para a regra de
    /// exclusividade mútua entre os dois casos.
    pub identificacao_cliente: IdentificacaoCliente,
    pub itens: Vec<ItemOrcamento>,
    pub desconto_centavos: i64,
    pub validade_dias: u16,
    pub status: StatusOrcamento,
    /// Data de criação do orçamento — âncora do vencimento (`criado_em +
    /// validade_dias`). Snapshots antigos não têm o campo: o default `Utc::now`
    /// faz o vencimento contar a partir da primeira releitura, o comportamento
    /// que já valia antes (vencimento deslizante) — nunca MENOS prazo que antes.
    #[serde(default = "Utc::now")]
    pub criado_em: DateTime<Utc>,
}

impl Orcamento {
    pub fn criar(
        vendedor_id: Uuid,
        cliente_id: Option<Uuid>,
        cliente_avulso: Option<String>,
        validade_dias: u16,
    ) -> DomainResult<Self> {
        let identificacao_cliente = IdentificacaoCliente::resolver(cliente_id, cliente_avulso)?;
        let id = OrcamentoId::new();
        let criado_em = Utc::now();
        let mut events = AggregateEvents::default();

        events.raise(OrcamentoEvent::OrcamentoCriado {
            orcamento_id: id.to_string(),
            vendedor_id: vendedor_id.to_string(),
            cliente_id: identificacao_cliente.cliente_id().map(|c| c.to_string()),
            cliente_avulso: identificacao_cliente.nome_avulso().map(str::to_string),
            validade_dias,
            occurred_at: criado_em,
        });

        Ok(Self {
            id,
            version: 0,
            events,
            vendedor_id,
            identificacao_cliente,
            itens: vec![],
            desconto_centavos: 0,
            validade_dias,
            status: StatusOrcamento::Rascunho,
            criado_em,
        })
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
        self.garantir_editavel()?;
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
        self.itens.push(ItemOrcamento {
            item_id,
            produto_id,
            sku: sku.clone(),
            descricao: descricao.clone(),
            quantidade: quantidade.get(),
            preco_unitario_centavos: preco_unitario.centavos(),
        });

        self.events.raise(OrcamentoEvent::ItemAdicionadoOrcamento {
            orcamento_id: self.id.to_string(),
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
        self.garantir_editavel()?;

        let pos = self
            .itens
            .iter()
            .position(|i| i.item_id == item_id)
            .ok_or_else(|| DomainError::NotFound(format!("Item {item_id} não encontrado")))?;

        self.itens.remove(pos);

        self.events.raise(OrcamentoEvent::ItemRemovidoOrcamento {
            orcamento_id: self.id.to_string(),
            item_id: item_id.to_string(),
            occurred_at: Utc::now(),
        });

        Ok(())
    }

    pub fn aplicar_desconto(&mut self, desconto_centavos: i64) -> DomainResult<()> {
        self.garantir_editavel()?;

        let total = self.total_bruto();

        if desconto_centavos < 0 || desconto_centavos > total {
            return Err(DomainError::Validation(
                "Desconto inválido: deve ser entre zero e o total do orçamento".into(),
            ));
        }
        self.desconto_centavos = desconto_centavos;
        self.events
            .raise(OrcamentoEvent::DescontoOrcamentoAplicado {
                orcamento_id: self.id.to_string(),
                desconto_centavos,
                occurred_at: Utc::now(),
            });

        Ok(())
    }

    pub fn atualizar(
        &mut self,
        cliente_id: Option<Uuid>,
        cliente_avulso: Option<String>,
        validade_dias: u16,
    ) -> DomainResult<()> {
        self.garantir_editavel()?;

        let identificacao_cliente = IdentificacaoCliente::resolver(cliente_id, cliente_avulso)?;

        self.validade_dias = validade_dias;
        self.events.raise(OrcamentoEvent::OrcamentoAtualizado {
            orcamento_id: self.id.to_string(),
            cliente_id: identificacao_cliente.cliente_id().map(|c| c.to_string()),
            cliente_avulso: identificacao_cliente.nome_avulso().map(str::to_string),
            validade_dias,
            occurred_at: Utc::now(),
        });

        self.identificacao_cliente = identificacao_cliente;

        Ok(())
    }

    pub fn cancelar(&mut self, motivo: String) -> DomainResult<()> {
        if matches!(
            self.status,
            StatusOrcamento::Aceito | StatusOrcamento::ConvertidoEmVenda
        ) {
            return Err(DomainError::BusinessRule(
                "Orçamento aceito ou convertido em venda não pode ser cancelado".into(),
            ));
        }
        if self.status == StatusOrcamento::Cancelado {
            return Err(DomainError::BusinessRule(
                "Orçamento já está cancelado".into(),
            ));
        }
        self.status = StatusOrcamento::Cancelado;
        self.events.raise(OrcamentoEvent::OrcamentoCancelado {
            orcamento_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn emitir(&mut self) -> DomainResult<()> {
        if self.status != StatusOrcamento::Rascunho {
            return Err(DomainError::BusinessRule(
                "Apenas orçamentos em rascunho podem ser emitidos".into(),
            ));
        }
        if self.itens.is_empty() {
            return Err(DomainError::BusinessRule(
                "Orçamento sem itens não pode ser emitido".into(),
            ));
        }
        self.status = StatusOrcamento::Emitido;
        self.events.raise(OrcamentoEvent::OrcamentoEmitido {
            orcamento_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn aceitar(&mut self) -> DomainResult<Vec<ItemOrcamentoSnapshot>> {
        self.garantir_emitido()?;
        let snapshots: Vec<ItemOrcamentoSnapshot> = self
            .itens
            .iter()
            .map(|i| ItemOrcamentoSnapshot {
                item_id: i.item_id.to_string(),
                produto_id: i.produto_id.to_string(),
                sku: i.sku.clone(),
                descricao: i.descricao.clone(),
                quantidade: i.quantidade,
                preco_unitario_centavos: i.preco_unitario_centavos,
            })
            .collect();

        let total = self.total_liquido();
        self.status = StatusOrcamento::Aceito;
        self.events.raise(OrcamentoEvent::OrcamentoAceito {
            orcamento_id: self.id.to_string(),
            itens: snapshots.clone(),
            total_centavos: total,
            desconto_centavos: self.desconto_centavos,
            vendedor_id: self.vendedor_id.to_string(),
            cliente_id: self
                .identificacao_cliente
                .cliente_id()
                .map(|c| c.to_string()),
            occurred_at: Utc::now(),
        });
        Ok(snapshots)
    }

    pub fn recusar(&mut self, motivo: String) -> DomainResult<()> {
        self.garantir_emitido()?;
        self.status = StatusOrcamento::Recusado;
        self.events.raise(OrcamentoEvent::OrcamentoRecusado {
            orcamento_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn expirar(&mut self) -> DomainResult<()> {
        if self.status != StatusOrcamento::Emitido {
            return Err(DomainError::BusinessRule(
                "Apenas orçamentos emitidos podem expirar".into(),
            ));
        }
        self.status = StatusOrcamento::Expirado;
        self.events.raise(OrcamentoEvent::OrcamentoExpirado {
            orcamento_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn marcar_convertido(&mut self, venda_id: Uuid) -> DomainResult<()> {
        if self.status != StatusOrcamento::Aceito {
            return Err(DomainError::BusinessRule(
                "Apenas orçamentos aceitos podem ser convertidos".into(),
            ));
        }
        self.status = StatusOrcamento::ConvertidoEmVenda;
        self.events
            .raise(OrcamentoEvent::OrcamentoConvertidoEmVenda {
                orcamento_id: self.id.to_string(),
                venda_id: venda_id.to_string(),
                occurred_at: Utc::now(),
            });
        Ok(())
    }

    /// Vencimento ancorado na data de criação — não desliza a cada consulta.
    pub fn vencimento(&self) -> DateTime<Utc> {
        self.criado_em + Duration::days(self.validade_dias as i64)
    }

    pub fn total_bruto(&self) -> i64 {
        self.itens.iter().map(|i| i.subtotal()).sum()
    }

    pub fn total_liquido(&self) -> i64 {
        self.total_bruto() - self.desconto_centavos
    }

    fn garantir_editavel(&self) -> DomainResult<()> {
        if self.status != StatusOrcamento::Rascunho {
            return Err(DomainError::BusinessRule(
                "Orçamento só pode ser editado no status Rascunho".into(),
            ));
        }
        Ok(())
    }

    fn garantir_emitido(&self) -> DomainResult<()> {
        if self.status != StatusOrcamento::Emitido {
            return Err(DomainError::BusinessRule(
                "Operação permitida apenas para orçamentos emitidos".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Dinheiro;

    fn orcamento_com_item() -> Orcamento {
        let mut o = Orcamento::criar(Uuid::new_v4(), None, None, 7).expect("criar orçamento");
        o.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Disco de freio".into(),
            1,
            Dinheiro::from_centavos(15_000),
            Disponibilidade::NaoControlada,
        )
        .expect("adicionar item");
        o
    }

    #[test]
    fn emitir_rascunho_com_itens_muda_status() {
        let mut o = orcamento_com_item();
        o.emitir().expect("emitir");
        assert_eq!(o.status, StatusOrcamento::Emitido);
    }

    #[test]
    fn emitir_rascunho_sem_itens_retorna_erro() {
        let mut o = Orcamento::criar(Uuid::new_v4(), None, None, 7).expect("criar orçamento");
        assert!(matches!(o.emitir(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn aceitar_orcamento_emitido_muda_status() {
        let mut o = orcamento_com_item();
        o.emitir().expect("emitir");
        o.aceitar().expect("aceitar");
        assert_eq!(o.status, StatusOrcamento::Aceito);
    }

    #[test]
    fn aceitar_rascunho_retorna_erro() {
        let mut o = orcamento_com_item();
        assert!(matches!(o.aceitar(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn desconto_invalido_retorna_erro() {
        let mut o = orcamento_com_item();
        assert!(matches!(
            o.aplicar_desconto(99_999_999),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn adicionar_item_acima_do_saldo_controlado_retorna_erro() {
        let mut o = Orcamento::criar(Uuid::new_v4(), None, None, 7).expect("criar orçamento");
        let err = o.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Disco de freio".into(),
            5,
            Dinheiro::from_centavos(15_000),
            Disponibilidade::Controlada(3),
        );
        assert!(matches!(err, Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn adicionar_item_sem_checagem_ignora_saldo() {
        let mut o = Orcamento::criar(Uuid::new_v4(), None, None, 7).expect("criar orçamento");
        o.adicionar_item(
            Uuid::new_v4(),
            "SKU-1".into(),
            "Disco de freio".into(),
            999,
            Dinheiro::from_centavos(15_000),
            Disponibilidade::SemChecagem,
        )
        .expect("orçamento com flag do tenant ligada ignora o saldo");
    }
}
