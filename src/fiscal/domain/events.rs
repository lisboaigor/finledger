use chrono::{DateTime, Utc};
use pharos_macros::DomainEvent;

use super::value_objects::{ItemNF, ModeloNF, TotaisNF};

#[derive(Debug, Clone, DomainEvent)]
pub enum NotaFiscalEvent {
    NotaFiscalGerada {
        #[aggregate_id]
        nf_id: String,
        venda_id: String,
        cliente_id: Option<String>,
        modelo: ModeloNF,
        serie: String,
        numero: u32,
        itens: Vec<ItemNF>,
        totais: TotaisNF,
        /// IBS/CBS destacados são meramente informativos para o emitente
        /// (Simples Nacional sem regime regular). Congelado aqui a partir do
        /// perfil vigente na emissão — o BI usa para decidir se IBS/CBS entram
        /// no custo do vendedor. Eventos são publicados in-process (não
        /// serializados no event store), então o campo não carrega serde.
        ibs_cbs_informativo: bool,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalTransmitida {
        #[aggregate_id]
        nf_id: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    /// Nova tentativa de transmissão de NF presa (transmitida sem resposta da
    /// SEFAZ ou rejeitada após correção). `status_anterior` documenta de onde
    /// a nota saiu.
    NotaFiscalRetransmitida {
        #[aggregate_id]
        nf_id: String,
        status_anterior: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalAutorizada {
        #[aggregate_id]
        nf_id: String,
        chave: String,
        protocolo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalRejeitada {
        #[aggregate_id]
        nf_id: String,
        codigo: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    NotaFiscalCancelada {
        #[aggregate_id]
        nf_id: String,
        protocolo_cancelamento: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
    /// Cancelamento registrado como PENDENTE: a integração com a SEFAZ ainda
    /// não está ativa (trâmites burocráticos), então a nota fica marcada para
    /// cancelamento assim que a integração entrar em operação.
    CancelamentoNfSolicitado {
        #[aggregate_id]
        nf_id: String,
        motivo: String,
        #[occurred_at]
        occurred_at: DateTime<Utc>,
    },
}

#[cfg(test)]
mod tests {
    use super::super::value_objects::{ImpostoItem, ItemNF, TotaisNF};

    /// Trava de evolução de schema: payloads de `ItemNF`/`TotaisNF` gravados
    /// ANTES do motor tributário (sem os campos CBS/IBS/IS) precisam continuar
    /// deserializando — todo campo novo carrega `#[serde(default)]`.
    #[test]
    fn item_nf_pre_reforma_ainda_deserializa() {
        let json_antigo = r#"{
            "produto_id": "c0000000-0000-0000-0000-000000000001",
            "sku": "SKU-001",
            "descricao": "Mouse sem fio",
            "ncm": "84716053",
            "cfop": "5102",
            "quantidade": 2,
            "valor_unitario_centavos": 5000,
            "imposto": {
                "icms_centavos": 1800,
                "pis_centavos": 65,
                "cofins_centavos": 300
            }
        }"#;
        let item: ItemNF = serde_json::from_str(json_antigo).expect("payload antigo deserializa");
        assert_eq!(item.imposto.icms_centavos, 1800);
        assert_eq!(item.imposto.cbs_centavos, 0);
        assert_eq!(item.imposto.c_class_trib, None);
    }

    #[test]
    fn totais_nf_pre_reforma_ainda_deserializa() {
        let json_antigo = r#"{
            "produtos_centavos": 10000,
            "icms_centavos": 1800,
            "pis_centavos": 65,
            "cofins_centavos": 300,
            "total_centavos": 10000
        }"#;
        let totais: TotaisNF = serde_json::from_str(json_antigo).expect("payload antigo deserializa");
        assert_eq!(totais.total_centavos, 10_000);
        assert_eq!(totais.cbs_centavos, 0);
        assert_eq!(totais.ibs_uf_centavos, 0);
    }

    /// O caminho inverso também importa: `ImpostoItem` novo serializado e
    /// relido mantém os campos da reforma.
    #[test]
    fn imposto_novo_round_trip() {
        let imposto = ImpostoItem {
            cbs_centavos: 900,
            c_class_trib: Some("000001".into()),
            ..ImpostoItem::calcular_legado_simples(100_000)
        };
        let json = serde_json::to_string(&imposto).expect("serializa");
        let relido: ImpostoItem = serde_json::from_str(&json).expect("deserializa");
        assert_eq!(relido.cbs_centavos, 900);
        assert_eq!(relido.c_class_trib.as_deref(), Some("000001"));
        assert_eq!(relido.icms_centavos, 18_000);
    }
}
