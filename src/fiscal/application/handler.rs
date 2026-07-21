use std::sync::Arc;

use pharos_app::EventBus;
use pharos_core::Entity;
use uuid::Uuid;

use super::commands::{CancelarNotaFiscal, RetransmitirNotaFiscal};
use crate::error::AppError;
use crate::fiscal::domain::nota_fiscal::{NotaFiscal, NotaFiscalId};
use crate::fiscal::domain::tributacao::{
    ClasseTributaria, ContextoFiscal, FaseTransicao, MotorTributario, PerfilFiscal,
};
use crate::fiscal::domain::value_objects::{ItemNF, ModeloNF};
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::repository::PostgresNotaFiscalRepository;
use crate::fiscal::infrastructure::sefaz::{SefazClient, SefazError};
use crate::shared::{load_aggregate, salvar_aggregate};
use crate::tenants::repository::TenantRepository;
use crate::vendas::domain::events::ItemVendaSnapshot;

pub struct FiscalHandlers<S: SefazClient, A: AliquotaProvider> {
    pub(crate) repo: Arc<PostgresNotaFiscalRepository>,
    pub(crate) sefaz: Arc<S>,
    pub(crate) aliquotas: Arc<A>,
    pub(crate) tenants: Arc<TenantRepository>,
    pub(crate) bus: EventBus,
}

impl<S: SefazClient, A: AliquotaProvider> FiscalHandlers<S, A> {
    pub fn new(
        repo: Arc<PostgresNotaFiscalRepository>,
        sefaz: Arc<S>,
        aliquotas: Arc<A>,
        tenants: Arc<TenantRepository>,
        bus: EventBus,
    ) -> Self {
        Self {
            repo,
            sefaz,
            aliquotas,
            tenants,
            bus,
        }
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
        // Perfil e fase resolvidos uma vez por NF; os valores calculados ficam
        // congelados no evento — replay/retransmissão nunca recalcula.
        let perfil = self
            .tenants
            .obter_perfil_fiscal()
            .await?
            .para_dominio()?
            .unwrap_or_else(PerfilFiscal::padrao_legado);
        let data_emissao = chrono::Utc::now().date_naive();
        let ctx = ContextoFiscal {
            fase: FaseTransicao::de_data(data_emissao),
            perfil,
        };

        let mut result = Vec::with_capacity(itens.len());
        for item in itens {
            let produto_id = Uuid::parse_str(&item.produto_id).map_err(AppError::infra)?;
            let linha: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT ncm, c_class_trib FROM proj_produtos WHERE produto_id = $1",
            )
            .bind(produto_id)
            .fetch_optional(self.repo.pool())
            .await
            .map_err(AppError::infra)?;
            let (ncm, classe_produto) = linha.unwrap_or(("00000000".into(), None));

            let classe_vo = classe_produto
                .map(ClasseTributaria::try_from)
                .transpose()
                .map_err(AppError::Domain)?;
            let classe = self.aliquotas.classe_info(classe_vo.as_ref()).await?;
            let aliquotas = self
                .aliquotas
                .resolver(data_emissao, &ctx.perfil, &classe.classe, &ncm)
                .await?;

            let total = item.preco_unitario_centavos * item.quantidade as i64;
            let imposto = MotorTributario::calcular_item(&ctx, &aliquotas, &classe, total);
            result.push(ItemNF::novo(
                produto_id,
                item.sku.clone(),
                item.descricao.clone(),
                ncm,
                modelo.cfop_padrao().into(),
                item.quantidade,
                item.preco_unitario_centavos,
                imposto,
            )?);
        }
        Ok(result)
    }
}
