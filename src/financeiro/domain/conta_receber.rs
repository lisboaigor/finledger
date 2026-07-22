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
    /// Total abatido do valor da conta (devolução parcial, desconto negociado).
    /// Campo novo: `default` para snapshots antigos sem ele.
    #[serde(default = "Dinheiro::zero")]
    pub valor_abatido: Dinheiro,
    /// Rótulo humano (ex.: "Parcela 2/3 — venda X"). Campo novo: `default`
    /// para snapshots antigos.
    #[serde(default)]
    pub descricao: Option<String>,
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
        Self::criar_com_id(
            ContaReceberId::new(),
            venda_id,
            cliente_id,
            valor,
            vencimento,
            None,
        )
    }

    /// Criação com id fornecido pelo chamador — os event handlers cross-BC
    /// derivam ids determinísticos (UUID v5 de venda/parcela) para que a
    /// re-entrega do mesmo evento (at-least-once) não duplique contas.
    pub fn criar_com_id(
        id: ContaReceberId,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        valor: Dinheiro,
        vencimento: DateTime<Utc>,
        descricao: Option<String>,
    ) -> Self {
        let mut events = AggregateEvents::default();
        events.raise(FinanceiroEvent::ContaReceberRegistrada {
            conta_id: id.to_string(),
            venda_id: venda_id.to_string(),
            cliente_id: cliente_id.map(|c| c.to_string()),
            valor_centavos: valor.centavos(),
            vencimento,
            descricao: descricao.clone(),
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
            valor_abatido: Dinheiro::zero(),
            descricao,
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
        let saldo = self.saldo_devedor().centavos();
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
            valor_recebido_total_centavos: self.valor_recebido.centavos(),
            occurred_at: Utc::now(),
        });
        if self.saldo_devedor().centavos() == 0 {
            self.liquidar();
        } else {
            self.status = StatusContaReceber::Parcial;
        }
        Ok(())
    }

    /// Abate parte (ou todo) o saldo em aberto — devolução parcial de venda ou
    /// desconto negociado. Mantém `valor_original` intacto para histórico.
    ///
    /// Se o saldo zerar, a conta vira `Liquidada` mesmo sem recebimento:
    /// abatimento total é uma quitação comercial (nada mais é devido, sem
    /// inadimplência); `Estornada` fica reservada a desfazer a conta em si
    /// (devolução total / erro de lançamento).
    pub fn abater(&mut self, valor: Dinheiro, motivo: String) -> DomainResult<()> {
        if matches!(
            self.status,
            StatusContaReceber::Liquidada | StatusContaReceber::Estornada
        ) {
            return Err(DomainError::BusinessRule("Conta já está encerrada".into()));
        }
        if valor.centavos() <= 0 {
            return Err(DomainError::Validation(
                "Valor do abatimento deve ser positivo".into(),
            ));
        }
        let saldo = self.saldo_devedor().centavos();
        if valor.centavos() > saldo {
            return Err(DomainError::BusinessRule(format!(
                "Abatimento ({}) excede o saldo em aberto ({})",
                valor,
                Dinheiro::from_centavos(saldo)
            )));
        }
        self.valor_abatido =
            Dinheiro::from_centavos(self.valor_abatido.centavos() + valor.centavos());
        self.events
            .raise(FinanceiroEvent::AbatimentoContaReceberRegistrado {
                conta_id: self.id.to_string(),
                valor_centavos: valor.centavos(),
                valor_abatido_total_centavos: self.valor_abatido.centavos(),
                motivo,
                occurred_at: Utc::now(),
            });
        if self.saldo_devedor().centavos() == 0 {
            self.liquidar();
        } else if self.valor_recebido.centavos() > 0 {
            self.status = StatusContaReceber::Parcial;
        }
        Ok(())
    }

    fn liquidar(&mut self) {
        self.status = StatusContaReceber::Liquidada;
        self.events.raise(FinanceiroEvent::ContaReceberLiquidada {
            conta_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
    }

    /// Estorno usado pelos fluxos automáticos (devolução total): cancela a
    /// conta independente de já ter recebimento — o reembolso do que foi
    /// recebido é tratado à parte (ContaPagar de reembolso ao cliente).
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

    /// Estorno manual (tela Financeiro): bloqueado quando já houve
    /// recebimento — estornar apagaria o valor recebido do saldo do caixa.
    /// Nesse caso o operador deve usar abatimento (saldo em aberto) e/ou
    /// registrar um reembolso ao cliente.
    pub fn estornar_manual(&mut self, motivo: String) -> DomainResult<()> {
        if self.valor_recebido.centavos() > 0 {
            return Err(DomainError::BusinessRule(format!(
                "Conta já tem {} recebidos — use abatimento do saldo em aberto \
                 e/ou reembolso ao cliente em vez de estorno",
                self.valor_recebido
            )));
        }
        self.estornar(motivo)
    }

    /// Saldo em aberto: original − abatido − recebido.
    pub fn saldo_devedor(&self) -> Dinheiro {
        Dinheiro::from_centavos(
            self.valor_original.centavos()
                - self.valor_abatido.centavos()
                - self.valor_recebido.centavos(),
        )
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

    #[test]
    fn abatimento_reduz_saldo_e_preserva_valor_original() {
        let mut c = conta(10_000);
        c.abater(Dinheiro::from_centavos(3_000), "devolução parcial".into())
            .expect("abater");
        assert_eq!(c.valor_original.centavos(), 10_000);
        assert_eq!(c.valor_abatido.centavos(), 3_000);
        assert_eq!(c.saldo_devedor().centavos(), 7_000);
        assert_eq!(c.status, StatusContaReceber::Pendente);
    }

    #[test]
    fn abatimento_que_zera_saldo_liquida_conta() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(4_000))
            .expect("pagamento");
        c.abater(Dinheiro::from_centavos(6_000), "devolução parcial".into())
            .expect("abater");
        assert_eq!(c.status, StatusContaReceber::Liquidada);
        assert_eq!(c.saldo_devedor().centavos(), 0);
    }

    #[test]
    fn abatimento_maior_que_saldo_retorna_erro() {
        let mut c = conta(5_000);
        c.registrar_pagamento(Dinheiro::from_centavos(2_000))
            .expect("pagamento");
        assert!(matches!(
            c.abater(Dinheiro::from_centavos(3_100), "x".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn pagamento_respeita_saldo_apos_abatimento() {
        let mut c = conta(10_000);
        c.abater(Dinheiro::from_centavos(4_000), "x".into())
            .expect("abater");
        assert!(matches!(
            c.registrar_pagamento(Dinheiro::from_centavos(7_000)),
            Err(DomainError::BusinessRule(_))
        ));
        c.registrar_pagamento(Dinheiro::from_centavos(6_000))
            .expect("quitar saldo restante");
        assert_eq!(c.status, StatusContaReceber::Liquidada);
    }

    #[test]
    fn estorno_manual_com_recebimento_retorna_erro() {
        let mut c = conta(10_000);
        c.registrar_pagamento(Dinheiro::from_centavos(1_000))
            .expect("pagamento");
        assert!(matches!(
            c.estornar_manual("engano".into()),
            Err(DomainError::BusinessRule(_))
        ));
        // O estorno automático (devolução total) continua permitido.
        c.estornar("devolução total".into()).expect("estornar");
        assert_eq!(c.status, StatusContaReceber::Estornada);
    }

    #[test]
    fn estorno_manual_sem_recebimento_estorna() {
        let mut c = conta(10_000);
        c.estornar_manual("lançamento errado".into())
            .expect("estornar");
        assert_eq!(c.status, StatusContaReceber::Estornada);
    }
}
