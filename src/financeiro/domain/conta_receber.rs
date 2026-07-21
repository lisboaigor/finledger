use chrono::{DateTime, Utc};
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::FinanceiroEvent;
use crate::shared::Dinheiro;

id_type!(ContaReceberId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusContaReceber {
    Pendente,
    Parcial,
    Liquidada,
    Estornada,
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct ContaReceber {
    #[id]
    id: ContaReceberId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<FinanceiroEvent>,
    pub venda_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub valor_original: Dinheiro,
    pub valor_recebido: Dinheiro,
    pub vencimento: DateTime<Utc>,
    pub status: StatusContaReceber,
}

impl ContaReceber {
    /// Como `criar`, mas calcula o vencimento a partir de um prazo em dias —
    /// a matemática de data pertence ao domínio, não ao event handler que
    /// traduz `VendaConfirmada` para este agregado.
    pub fn criar_a_prazo(
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        valor: Dinheiro,
        dias_prazo: i64,
    ) -> Self {
        Self::criar(
            venda_id,
            cliente_id,
            valor,
            Utc::now() + chrono::Duration::days(dias_prazo),
        )
    }

    pub fn criar(
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        valor: Dinheiro,
        vencimento: DateTime<Utc>,
    ) -> Self {
        let id = ContaReceberId::new();
        let mut events = AggregateEvents::default();
        events.raise(FinanceiroEvent::ContaReceberRegistrada {
            conta_id: id.to_string(),
            venda_id: venda_id.to_string(),
            cliente_id: cliente_id.map(|c| c.to_string()),
            valor_centavos: valor.centavos(),
            vencimento,
            occurred_at: Utc::now(),
        });
        Self {
            id,
            version: 0,
            events,
            venda_id,
            cliente_id,
            valor_original: valor,
            valor_recebido: Dinheiro::zero(),
            vencimento,
            status: StatusContaReceber::Pendente,
        }
    }

    pub fn registrar_pagamento(&mut self, valor: Dinheiro) -> DomainResult<()> {
        if matches!(
            self.status,
            StatusContaReceber::Liquidada | StatusContaReceber::Estornada
        ) {
            return Err(DomainError::BusinessRule("Conta já está encerrada".into()));
        }
        if valor.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Valor do pagamento deve ser positivo".into(),
            ));
        }
        let saldo = self.valor_original.centavos() - self.valor_recebido.centavos();
        if valor.centavos() > saldo {
            return Err(DomainError::BusinessRule(format!(
                "Pagamento ({}) excede o saldo devedor ({})",
                valor,
                Dinheiro::from_centavos(saldo)
            )));
        }
        self.valor_recebido =
            Dinheiro::from_centavos(self.valor_recebido.centavos() + valor.centavos());
        self.events.raise(FinanceiroEvent::PagamentoRecebido {
            conta_id: self.id.to_string(),
            valor_centavos: valor.centavos(),
            occurred_at: Utc::now(),
        });
        if self.valor_recebido == self.valor_original {
            self.status = StatusContaReceber::Liquidada;
            self.events.raise(FinanceiroEvent::ContaReceberLiquidada {
                conta_id: self.id.to_string(),
                occurred_at: Utc::now(),
            });
        } else {
            self.status = StatusContaReceber::Parcial;
        }
        Ok(())
    }

    pub fn estornar(&mut self, motivo: String) -> DomainResult<()> {
        if self.status == StatusContaReceber::Estornada {
            return Err(DomainError::BusinessRule("Conta já estornada".into()));
        }
        self.status = StatusContaReceber::Estornada;
        self.events.raise(FinanceiroEvent::ContaReceberEstornada {
            conta_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }

    pub fn saldo_devedor(&self) -> Dinheiro {
        Dinheiro::from_centavos(self.valor_original.centavos() - self.valor_recebido.centavos())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn conta(valor: i64) -> ContaReceber {
        ContaReceber::criar(
            Uuid::new_v4(),
            None,
            Dinheiro::from_centavos(valor),
            Utc::now(),
        )
    }

    #[test]
    fn pagamento_parcial_muda_status_para_parcial() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(4_000))
            .expect("pagamento");
        assert_eq!(c.status, StatusContaReceber::Parcial);
        assert_eq!(c.saldo_devedor().centavos(), 6_000);
    }

    #[test]
    fn pagamento_total_liquida_conta() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(10_000))
            .expect("pagamento");
        assert_eq!(c.status, StatusContaReceber::Liquidada);
    }

    #[test]
    fn pagamento_apos_liquidada_retorna_erro() {
        let mut c = conta(5_000);
        c.registrar_pagamento(Dinheiro::from_centavos(5_000))
            .expect("liquidar");
        assert!(matches!(
            c.registrar_pagamento(Dinheiro::from_centavos(100)),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn pagamento_excede_saldo_retorna_erro() {
        let mut c = conta(5_000);
        assert!(matches!(
            c.registrar_pagamento(Dinheiro::from_centavos(6_000)),
            Err(DomainError::BusinessRule(_))
        ));
    }
}
