use chrono::{DateTime, Utc};
use pharos_core::{AggregateEvents, DomainError, DomainResult};
use pharos_macros::{AggregateRoot, Entity, id_type};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::events::NotaFiscalEvent;
use super::value_objects::{ItemNF, ModeloNF, StatusNFe, TotaisNF};

id_type!(NotaFiscalId);

#[derive(Debug, Clone, Entity, AggregateRoot, Serialize, Deserialize)]
pub struct NotaFiscal {
    #[id]
    id: NotaFiscalId,
    #[version]
    version: u64,
    #[events]
    #[serde(skip)]
    events: AggregateEvents<NotaFiscalEvent>,
    pub venda_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub modelo: ModeloNF,
    pub serie: String,
    pub numero: u32,
    pub chave: Option<String>,
    pub protocolo: Option<String>,
    pub status: StatusNFe,
    pub itens: Vec<ItemNF>,
    pub totais: TotaisNF,
    pub gerada_em: DateTime<Utc>,
    /// Cancelamento aguardando a integração SEFAZ entrar em operação
    /// (`serde(default)` mantém snapshots antigos deserializáveis).
    #[serde(default)]
    pub cancelamento_pendente: bool,
}

impl NotaFiscal {
    pub fn gerar(
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        modelo: ModeloNF,
        serie: String,
        numero: u32,
        itens: Vec<ItemNF>,
        ibs_cbs_informativo: bool,
    ) -> DomainResult<Self> {
        if itens.is_empty() {
            return Err(DomainError::Validation(
                "NF deve ter ao menos um item".into(),
            ));
        }
        let totais = TotaisNF::calcular(&itens);
        let id = NotaFiscalId::new();
        let now = Utc::now();
        let mut events = AggregateEvents::default();
        events.raise(NotaFiscalEvent::NotaFiscalGerada {
            nf_id: id.to_string(),
            venda_id: venda_id.to_string(),
            cliente_id: cliente_id.map(|c| c.to_string()),
            modelo: modelo.clone(),
            serie: serie.clone(),
            numero,
            itens: itens.clone(),
            totais: totais.clone(),
            ibs_cbs_informativo,
            occurred_at: now,
        });
        Ok(Self {
            id,
            version: 0,
            events,
            venda_id,
            cliente_id,
            modelo,
            serie,
            numero,
            chave: None,
            protocolo: None,
            status: StatusNFe::Gerada,
            itens,
            totais,
            gerada_em: now,
            cancelamento_pendente: false,
        })
    }

    pub fn transmitir(&mut self) -> DomainResult<()> {
        if self.status != StatusNFe::Gerada {
            return Err(DomainError::BusinessRule(
                "Apenas NF com status Gerada pode ser transmitida".into(),
            ));
        }
        self.events.raise(NotaFiscalEvent::NotaFiscalTransmitida {
            nf_id: self.id.to_string(),
            occurred_at: Utc::now(),
        });
        self.status = StatusNFe::Transmitida;
        Ok(())
    }

    pub fn autorizar(&mut self, chave: String, protocolo: String) -> DomainResult<()> {
        if self.status != StatusNFe::Transmitida {
            return Err(DomainError::BusinessRule(
                "Apenas NF transmitida pode ser autorizada".into(),
            ));
        }
        self.events.raise(NotaFiscalEvent::NotaFiscalAutorizada {
            nf_id: self.id.to_string(),
            chave: chave.clone(),
            protocolo: protocolo.clone(),
            occurred_at: Utc::now(),
        });
        self.chave = Some(chave);
        self.protocolo = Some(protocolo);
        self.status = StatusNFe::Autorizada;
        Ok(())
    }

    pub fn rejeitar(&mut self, codigo: String, motivo: String) -> DomainResult<()> {
        if self.status != StatusNFe::Transmitida {
            return Err(DomainError::BusinessRule(
                "Apenas NF transmitida pode ser rejeitada".into(),
            ));
        }
        self.events.raise(NotaFiscalEvent::NotaFiscalRejeitada {
            nf_id: self.id.to_string(),
            codigo,
            motivo,
            occurred_at: Utc::now(),
        });
        self.status = StatusNFe::Rejeitada;
        Ok(())
    }

    pub fn cancelar(&mut self, protocolo_cancelamento: String) -> DomainResult<()> {
        if self.status != StatusNFe::Autorizada {
            return Err(DomainError::BusinessRule(
                "Apenas NF autorizada pode ser cancelada".into(),
            ));
        }
        self.events.raise(NotaFiscalEvent::NotaFiscalCancelada {
            nf_id: self.id.to_string(),
            protocolo_cancelamento,
            occurred_at: Utc::now(),
        });
        self.status = StatusNFe::Cancelada;
        self.cancelamento_pendente = false;
        Ok(())
    }

    /// Marca a nota para cancelamento futuro — usado quando a devolução ocorre
    /// antes de a integração SEFAZ estar em operação. O cancelamento efetivo
    /// acontece via `cancelar()` quando a integração for ativada.
    pub fn solicitar_cancelamento(&mut self, motivo: String) -> DomainResult<()> {
        if self.status != StatusNFe::Autorizada {
            return Err(DomainError::BusinessRule(
                "Apenas NF autorizada pode ter cancelamento solicitado".into(),
            ));
        }
        if self.cancelamento_pendente {
            return Err(DomainError::BusinessRule(
                "Cancelamento já está pendente para esta nota".into(),
            ));
        }
        self.cancelamento_pendente = true;
        self.events.raise(NotaFiscalEvent::CancelamentoNfSolicitado {
            nf_id: self.id.to_string(),
            motivo,
            occurred_at: Utc::now(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiscal::domain::value_objects::{ImpostoItem, ItemNF};

    fn item_nf() -> ItemNF {
        ItemNF::novo(
            Uuid::new_v4(),
            "SKU-001".into(),
            "Mouse sem fio".into(),
            "84716053".into(),
            ModeloNF::NFCe.cfop_padrao().into(),
            2,
            5000,
            ImpostoItem::calcular_legado_simples(10_000),
        )
        .expect("item válido")
    }

    fn nf_gerada() -> NotaFiscal {
        NotaFiscal::gerar(
            Uuid::new_v4(),
            None,
            ModeloNF::NFCe,
            "001".into(),
            1,
            vec![item_nf()],
            false,
        )
        .expect("deve gerar NF válida")
    }

    #[test]
    fn gerar_sem_itens_retorna_erro() {
        let err = NotaFiscal::gerar(
            Uuid::new_v4(),
            None,
            ModeloNF::NFCe,
            "001".into(),
            1,
            vec![],
            false,
        );
        assert!(matches!(err, Err(DomainError::Validation(_))));
    }

    #[test]
    fn gerar_com_item_inicia_status_gerada() {
        let nf = nf_gerada();
        assert_eq!(nf.status, StatusNFe::Gerada);
        assert!(nf.chave.is_none());
        assert_eq!(nf.totais.total_centavos, 10_000);
    }

    #[test]
    fn transmitir_muda_status_para_transmitida() {
        let mut nf = nf_gerada();
        nf.transmitir().expect("deve transmitir");
        assert_eq!(nf.status, StatusNFe::Transmitida);
    }

    #[test]
    fn transmitir_ja_transmitida_retorna_erro() {
        let mut nf = nf_gerada();
        nf.transmitir().expect("transmissão 1");
        assert!(matches!(nf.transmitir(), Err(DomainError::BusinessRule(_))));
    }

    #[test]
    fn autorizar_apos_transmitida_registra_chave() {
        let mut nf = nf_gerada();
        nf.transmitir().expect("transmitir");
        nf.autorizar("44digits_key".into(), "protocolo123".into())
            .expect("autorizar");
        assert_eq!(nf.status, StatusNFe::Autorizada);
        assert_eq!(nf.chave.as_deref(), Some("44digits_key"));
    }

    #[test]
    fn autorizar_sem_transmitir_retorna_erro() {
        let mut nf = nf_gerada();
        assert!(matches!(
            nf.autorizar("chave".into(), "proto".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn cancelar_nf_autorizada_muda_status() {
        let mut nf = nf_gerada();
        nf.transmitir().expect("transmitir");
        nf.autorizar("chave".into(), "proto".into())
            .expect("autorizar");
        nf.cancelar("CANC001".into()).expect("cancelar");
        assert_eq!(nf.status, StatusNFe::Cancelada);
    }

    #[test]
    fn cancelar_nf_nao_autorizada_retorna_erro() {
        let mut nf = nf_gerada();
        assert!(matches!(
            nf.cancelar("CANC001".into()),
            Err(DomainError::BusinessRule(_))
        ));
    }

    #[test]
    fn rejeitar_nf_transmitida_muda_status() {
        let mut nf = nf_gerada();
        nf.transmitir().expect("transmitir");
        nf.rejeitar("999".into(), "dados inválidos".into())
            .expect("rejeitar");
        assert_eq!(nf.status, StatusNFe::Rejeitada);
    }
}
