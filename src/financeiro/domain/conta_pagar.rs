use chrono::{DateTime, Utc};
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::FinanceiroEvent;
use crate::shared::Dinheiro;

id_type!(ContaPagarId);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatusContaPagar {
    Pendente,
    Parcial,
    Liquidada,
}

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct ContaPagar {
    #[id]
    id: ContaPagarId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<FinanceiroEvent>,
    pub pedido_id: Uuid,
    pub fornecedor_id: Uuid,
    pub valor_original: Dinheiro,
    pub valor_pago: Dinheiro,
    pub vencimento: DateTime<Utc>,
    pub status: StatusContaPagar,
}

impl ContaPagar {
    pub fn criar(
        pedido_id: Uuid,
        fornecedor_id: Uuid,
        valor: Dinheiro,
        vencimento: DateTime<Utc>,
    ) -> Self {
        let id = ContaPagarId::new();
        let mut events = AggregateEvents::default();
        events.raise(FinanceiroEvent::ContaPagarRegistrada {
            conta_id: id.to_string(),
            pedido_id: pedido_id.to_string(),
            fornecedor_id: fornecedor_id.to_string(),
            valor_centavos: valor.centavos(),
            vencimento,
            occurred_at: Utc::now(),
        });
        Self {
            id,
            version: 0,
            events,
            pedido_id,
            fornecedor_id,
            valor_original: valor,
            valor_pago: Dinheiro::zero(),
            vencimento,
            status: StatusContaPagar::Pendente,
        }
    }

    pub fn registrar_pagamento(&mut self, valor: Dinheiro) -> DomainResult<()> {
        if self.status == StatusContaPagar::Liquidada {
            return Err(DomainError::BusinessRule("Conta já está liquidada".into()));
        }
        if valor.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Valor do pagamento deve ser positivo".into(),
            ));
        }
        let saldo = self.valor_original.centavos() - self.valor_pago.centavos();
        if valor.centavos() > saldo {
            return Err(DomainError::BusinessRule(format!(
                "Pagamento ({}) excede o saldo ({}) ",
                valor,
                Dinheiro::from_centavos(saldo)
            )));
        }
        self.valor_pago = Dinheiro::from_centavos(self.valor_pago.centavos() + valor.centavos());
        self.events.raise(FinanceiroEvent::PagamentoEfetuado {
            conta_id: self.id.to_string(),
            valor_centavos: valor.centavos(),
            occurred_at: Utc::now(),
        });
        if self.valor_pago == self.valor_original {
            self.status = StatusContaPagar::Liquidada;
            self.events.raise(FinanceiroEvent::ContaPagarLiquidada {
                conta_id: self.id.to_string(),
                occurred_at: Utc::now(),
            });
        } else {
            self.status = StatusContaPagar::Parcial;
        }
        Ok(())
    }

    pub fn saldo_devedor(&self) -> Dinheiro {
        Dinheiro::from_centavos(self.valor_original.centavos() - self.valor_pago.centavos())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn conta(valor: i64) -> ContaPagar {
        ContaPagar::criar(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Dinheiro::from_centavos(valor),
            Utc::now(),
        )
    }

    #[test]
    fn pagamento_parcial_muda_status() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(3_000))
            .expect("pagamento");
        assert_eq!(c.status, StatusContaPagar::Parcial);
    }

    #[test]
    fn pagamento_total_liquida_conta() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(10_000))
            .expect("pagamento");
        assert_eq!(c.status, StatusContaPagar::Liquidada);
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
