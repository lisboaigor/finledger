use std::sync::Arc;

use pharos_app::EventBus;
use pharos_core::Entity;
use uuid::Uuid;

use super::commands::{CancelarNotaFiscal, RetransmitirNotaFiscal};
use crate::error::AppError;
use crate::fiscal::domain::nota_fiscal::{NotaFiscal, NotaFiscalId};
use crate::fiscal::domain::value_objects::{ImpostoItem, ItemNF, ModeloNF};
use crate::fiscal::infrastructure::repository::PostgresNotaFiscalRepository;
use crate::fiscal::infrastructure::sefaz::{SefazClient, SefazError};
use crate::shared::{load_aggregate, salvar_aggregate};
use crate::vendas::domain::events::ItemVendaSnapshot;

pub struct FiscalHandlers<S: SefazClient> {
    pub(crate) repo: Arc<PostgresNotaFiscalRepository>,
    pub(crate) sefaz: Arc<S>,
    pub(crate) bus: EventBus,
}

impl<S: SefazClient> FiscalHandlers<S> {
    pub fn new(repo: Arc<PostgresNotaFiscalRepository>, sefaz: Arc<S>, bus: EventBus) -> Self {
        Self { repo, sefaz, bus }
    }

    pub async fn gerar_e_transmitir(
        &self,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        itens_venda: &[ItemVendaSnapshot],
    ) -> Result<(), AppError> {
        let modelo = if cliente_id.is_some() {
            ModeloNF::NFe
        } else {
            ModeloNF::NFCe
        };
        let itens_nf = self.enriquecer_itens(itens_venda, &modelo).await?;

        let numero = (Uuid::new_v4().as_u128() % 999_999_999u128 + 1) as u32;

        let mut nf =
            NotaFiscal::gerar(venda_id, cliente_id, modelo, "001".into(), numero, itens_nf)?;

        self.salvar(&mut nf).await?;

        nf.transmitir()?;
        let xml = format!("<NF>{}</NF>", nf.id());
        match self.sefaz.transmitir(xml).await {
            Ok(resp) => {
                nf.autorizar(resp.chave, resp.protocolo)?;
            }
            Err(SefazError::Rejeicao { codigo, motivo }) => {
                nf.rejeitar(codigo, motivo)?;
            }
            Err(SefazError::Indisponivel(msg)) => {
                tracing::warn!("SEFAZ indisponível para NF {}: {msg}", nf.id());
            }
        }

        self.salvar(&mut nf).await
    }

    /// Integração real com a SEFAZ liberada? Enquanto os trâmites burocráticos
    /// não saem, cancelamentos de devolução ficam PENDENTES na nota.
    fn integracao_sefaz_ativa() -> bool {
        std::env::var("SEFAZ_INTEGRACAO_ATIVA")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false)
    }

    /// Reação fiscal a uma devolução de itens:
    /// - integração ATIVA: cancela a NF autorizada da venda e, em devolução
    ///   parcial, reemite uma nova NF com os itens restantes;
    /// - integração INATIVA (cenário atual): marca `cancelamento_pendente` na
    ///   nota — o cancelamento (e a reemissão) acontecem quando a integração
    ///   entrar em operação, via tela Fiscal.
    pub async fn processar_devolucao(
        &self,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        itens_restantes: &[ItemVendaSnapshot],
        devolucao_total: bool,
        motivo: &str,
    ) -> Result<(), AppError> {
        let nf_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT nf_id FROM proj_notas_fiscais
             WHERE venda_id = $1 AND status = 'autorizada'
             ORDER BY gerada_em DESC LIMIT 1",
        )
        .bind(venda_id)
        .fetch_optional(self.repo.pool())
        .await
        .map_err(AppError::infra)?;

        let Some(nf_id) = nf_id else {
            tracing::info!(%venda_id, "devolução sem NF autorizada — nada a cancelar no fiscal");
            return Ok(());
        };

        let mut nf = self.carregar(NotaFiscalId::from_uuid(nf_id)).await?;
        if Self::integracao_sefaz_ativa() {
            let proto = format!("CANC{}", Uuid::new_v4().simple());
            nf.cancelar(proto)?;
            self.salvar(&mut nf).await?;
            if !devolucao_total && !itens_restantes.is_empty() {
                // Reemissão com os itens que permaneceram na venda.
                self.gerar_e_transmitir(venda_id, cliente_id, itens_restantes)
                    .await?;
            }
        } else if nf.cancelamento_pendente {
            // Nova devolução sobre nota já marcada — nada a solicitar de novo.
            tracing::info!(%venda_id, "NF já com cancelamento pendente; devolução adicional registrada");
        } else {
            nf.solicitar_cancelamento(format!("Devolução de itens: {motivo}"))?;
            self.salvar(&mut nf).await?;
        }
        Ok(())
    }

    pub(crate) async fn cancelar(&self, cmd: CancelarNotaFiscal) -> Result<(), AppError> {
        let nf_id = NotaFiscalId::from(cmd.nf_id);
        let mut nf = self.carregar(nf_id).await?;
        let proto_cancelamento = format!("CANC{}", Uuid::new_v4().simple());
        nf.cancelar(proto_cancelamento)?;
        self.salvar(&mut nf).await
    }

    pub(crate) async fn retransmitir(&self, cmd: RetransmitirNotaFiscal) -> Result<(), AppError> {
        let nf_id = NotaFiscalId::from(cmd.nf_id);
        let mut nf = self.carregar(nf_id).await?;
        let xml = format!("<NF>{}</NF>", nf.id());
        match self.sefaz.transmitir(xml).await {
            Ok(resp) => {
                nf.transmitir()?;
                nf.autorizar(resp.chave, resp.protocolo)?;
            }
            Err(SefazError::Rejeicao { codigo, motivo }) => {
                nf.transmitir()?;
                nf.rejeitar(codigo, motivo)?;
            }
            Err(SefazError::Indisponivel(msg)) => {
                return Err(AppError::infra(msg));
            }
        }
        self.salvar(&mut nf).await
    }

    async fn carregar(&self, id: NotaFiscalId) -> Result<NotaFiscal, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, nf: &mut NotaFiscal) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, nf).await
    }

    async fn enriquecer_itens(
        &self,
        itens: &[ItemVendaSnapshot],
        modelo: &ModeloNF,
    ) -> Result<Vec<ItemNF>, AppError> {
        let mut result = Vec::with_capacity(itens.len());
        for item in itens {
            let produto_id = Uuid::parse_str(&item.produto_id).map_err(AppError::infra)?;
            let ncm: String =
                sqlx::query_scalar("SELECT ncm FROM proj_produtos WHERE produto_id = $1")
                    .bind(produto_id)
                    .fetch_optional(self.repo.pool())
                    .await
                    .map_err(AppError::infra)?
                    .unwrap_or_else(|| "00000000".into());

            let total = item.preco_unitario_centavos * item.quantidade as i64;
            result.push(ItemNF::novo(
                produto_id,
                item.sku.clone(),
                item.descricao.clone(),
                ncm,
                modelo.cfop_padrao().into(),
                item.quantidade,
                item.preco_unitario_centavos,
                ImpostoItem::calcular(total),
            )?);
        }
        Ok(result)
    }
}
