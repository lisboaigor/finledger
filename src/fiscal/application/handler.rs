use std::sync::Arc;

use pharos_app::EventBus;
use pharos_core::Entity;
use uuid::Uuid;

use super::commands::{CancelarNotaFiscal, RetransmitirNotaFiscal};
use super::queries::AliquotaEfetivaProduto;
use crate::error::AppError;
use crate::fiscal::domain::cfop::{resolver_cfop, TipoOperacao};
use crate::fiscal::domain::nota_fiscal::{NotaFiscal, NotaFiscalId};
use crate::fiscal::domain::tributacao::{
    AliquotasItem, ClasseTributaria, ContextoFiscal, FaseTransicao, MotorTributario, PerfilFiscal,
    RegimeTributario, hoje_brasil,
};
use crate::fiscal::domain::value_objects::{ItemNF, ModeloNF};
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::repository::PostgresNotaFiscalRepository;
use crate::fiscal::infrastructure::sefaz::{SefazClient, SefazError};
use crate::shared::tenant::current_tenant_id;
use crate::shared::{load_aggregate, salvar_aggregate};
use crate::tenants::repository::TenantRepository;
use crate::vendas::domain::events::ItemVendaSnapshot;

/// Série única de emissão enquanto não há gestão de séries (issue #16 segue
/// aberta para CFOP/ICMS-ST e múltiplas séries).
const SERIE_PADRAO: i32 = 1;

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
        desconto_centavos: i64,
    ) -> Result<(), AppError> {
        let modelo = if cliente_id.is_some() {
            ModeloNF::NFe
        } else {
            ModeloNF::NFCe
        };
        let (itens_nf, ibs_cbs_informativo) = self
            .enriquecer_itens(itens_venda, desconto_centavos, &modelo, cliente_id)
            .await?;

        let numero = self.proximo_numero(SERIE_PADRAO).await?;

        let mut nf = NotaFiscal::gerar(
            venda_id,
            cliente_id,
            modelo,
            format!("{SERIE_PADRAO:03}"),
            numero,
            itens_nf,
            desconto_centavos,
            ibs_cbs_informativo,
        )?;

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

    /// Próximo número da NF na série, sequencial e atômico por (tenant, série):
    /// o `INSERT ... ON CONFLICT DO UPDATE ... RETURNING` incrementa e devolve
    /// em uma única instrução — duas emissões concorrentes nunca repetem número.
    async fn proximo_numero(&self, serie: i32) -> Result<u32, AppError> {
        let tenant_id = current_tenant_id()?;
        let proximo: i64 = sqlx::query_scalar(
            "INSERT INTO fiscal_numeracao (tenant_id, serie) VALUES ($1, $2)
             ON CONFLICT (tenant_id, serie)
             DO UPDATE SET proximo = fiscal_numeracao.proximo + 1
             RETURNING proximo",
        )
        .bind(tenant_id)
        .bind(serie)
        .fetch_one(self.repo.pool())
        .await
        .map_err(AppError::infra)?;
        u32::try_from(proximo).map_err(|_| {
            AppError::Domain(pharos_core::DomainError::BusinessRule(format!(
                "Numeração da série {serie} esgotada ({proximo})"
            )))
        })
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
        // Além da autorizada, considera NF presa em 'transmitida' (SEFAZ pode
        // tê-la autorizado sem a resposta chegar) e 'rejeitada' (sem efeito
        // fiscal) — antes elas ficavam invisíveis para a devolução.
        let linha: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT nf_id, status FROM proj_notas_fiscais
             WHERE venda_id = $1 AND tenant_id = $2
               AND status IN ('autorizada', 'transmitida', 'rejeitada')
             ORDER BY gerada_em DESC LIMIT 1",
        )
        .bind(venda_id)
        .bind(current_tenant_id()?)
        .fetch_optional(self.repo.pool())
        .await
        .map_err(AppError::infra)?;

        let Some((nf_id, status)) = linha else {
            tracing::info!(%venda_id, "devolução sem NF autorizada/transmitida/rejeitada — nada a cancelar no fiscal");
            return Ok(());
        };

        if status == "rejeitada" {
            // NF rejeitada nunca produziu efeito fiscal: não há o que cancelar
            // nem reemitir — apenas registra a decisão.
            tracing::info!(%venda_id, %nf_id, "devolução sobre NF rejeitada — sem efeito fiscal, nada a cancelar");
            return Ok(());
        }

        let mut nf = self.carregar(NotaFiscalId::from_uuid(nf_id)).await?;
        if status == "transmitida" {
            // Presa no limbo: transmitida sem resposta da SEFAZ. Marca o
            // cancelamento como pendente (cobre o caso de ela ter sido
            // autorizada do outro lado) — resolve-se na tela Fiscal.
            tracing::warn!(%venda_id, %nf_id, "devolução sobre NF presa em 'transmitida' — marcando cancelamento pendente");
            if nf.cancelamento_pendente {
                tracing::info!(%venda_id, "NF já com cancelamento pendente; devolução adicional registrada");
            } else {
                nf.solicitar_cancelamento(format!("Devolução de itens: {motivo}"))?;
                self.salvar(&mut nf).await?;
            }
            return Ok(());
        }

        if Self::integracao_sefaz_ativa() {
            let proto = format!("CANC{}", Uuid::new_v4().simple());
            nf.cancelar(proto)?;
            self.salvar(&mut nf).await?;
            if !devolucao_total && !itens_restantes.is_empty() {
                // Reemissão com os itens que permaneceram na venda. Sem
                // desconto: assim como a CR não é ajustada automaticamente na
                // devolução parcial, o abatimento é negociado no financeiro.
                self.gerar_e_transmitir(venda_id, cliente_id, itens_restantes, 0)
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

    /// Retransmite uma NF presa: `Gerada` (nunca saiu), `Transmitida` (SEFAZ
    /// ficou indisponível e a resposta nunca chegou) ou `Rejeitada` (após
    /// correção). O agregado valida a transição em `NotaFiscal::retransmitir`.
    pub(crate) async fn retransmitir(&self, cmd: RetransmitirNotaFiscal) -> Result<(), AppError> {
        let nf_id = NotaFiscalId::from(cmd.nf_id);
        let mut nf = self.carregar(nf_id).await?;
        let xml = format!("<NF>{}</NF>", nf.id());
        match self.sefaz.transmitir(xml).await {
            Ok(resp) => {
                nf.retransmitir()?;
                nf.autorizar(resp.chave, resp.protocolo)?;
            }
            Err(SefazError::Rejeicao { codigo, motivo }) => {
                nf.retransmitir()?;
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

    /// Rateia o desconto global da venda entre os itens, proporcional ao
    /// subtotal de cada um (aritmética inteira; a sobra de arredondamento vai
    /// para o último item, garantindo Σ ratear == desconto). Com desconto zero
    /// devolve zeros — a base de cálculo fica exatamente o subtotal, como antes.
    fn ratear_desconto(itens: &[ItemVendaSnapshot], desconto_centavos: i64) -> Vec<i64> {
        let total_bruto: i64 = itens
            .iter()
            .map(|i| i.preco_unitario_centavos * i.quantidade as i64)
            .sum();
        if desconto_centavos <= 0 || total_bruto <= 0 {
            return vec![0; itens.len()];
        }
        let mut rateado = Vec::with_capacity(itens.len());
        let mut acumulado: i64 = 0;
        for (idx, item) in itens.iter().enumerate() {
            let quota = if idx + 1 == itens.len() {
                desconto_centavos - acumulado
            } else {
                let subtotal = item.preco_unitario_centavos * item.quantidade as i64;
                ((desconto_centavos as i128 * subtotal as i128) / total_bruto as i128) as i64
            };
            acumulado += quota;
            rateado.push(quota);
        }
        rateado
    }

    /// Enriquece cada item com os impostos calculados pelo motor e devolve, junto,
    /// o flag `ibs_cbs_informativo` do perfil vigente (congelado na NF para o BI).
    /// A base de cálculo de cada item é o subtotal menos a quota rateada do
    /// desconto global da venda.
    async fn enriquecer_itens(
        &self,
        itens: &[ItemVendaSnapshot],
        desconto_centavos: i64,
        modelo: &ModeloNF,
        cliente_id: Option<Uuid>,
    ) -> Result<(Vec<ItemNF>, bool), AppError> {
        // Perfil e fase resolvidos uma vez por NF; os valores calculados ficam
        // congelados no evento — replay/retransmissão nunca recalcula.
        let perfil = self
            .tenants
            .obter_perfil_fiscal()
            .await?
            .para_dominio()?
            .unwrap_or_else(PerfilFiscal::padrao_legado);
        // Dia fiscal no Brasil (America/Sao_Paulo), não em UTC — NF emitida à
        // noite não pode pular de dia (nem de fase, na virada de ano).
        let data_emissao = hoje_brasil();
        // UF do destinatário (para o CFOP intra/interestadual). Ausente =
        // operação interna. UF do emitente vem do perfil fiscal do tenant.
        let uf_emitente = perfil.uf.as_str().to_string();
        let uf_destinatario: Option<String> = if let Some(cid) = cliente_id {
            sqlx::query_scalar(
                "SELECT uf FROM proj_clientes WHERE cliente_id = $1 AND tenant_id = $2",
            )
            .bind(cid)
            .bind(current_tenant_id()?)
            .fetch_optional(self.repo.pool())
            .await
            .map_err(AppError::infra)?
            .flatten()
        } else {
            None
        };
        let ctx = ContextoFiscal {
            fase: FaseTransicao::de_data(data_emissao),
            perfil,
        };
        let ibs_cbs_informativo = ctx.perfil.ibs_cbs_informativo();

        let rateio = Self::ratear_desconto(itens, desconto_centavos);
        let mut result = Vec::with_capacity(itens.len());
        for (item, desconto_item) in itens.iter().zip(rateio) {
            let produto_id = Uuid::parse_str(&item.produto_id).map_err(AppError::infra)?;
            let linha: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT ncm, c_class_trib FROM proj_produtos
                 WHERE produto_id = $1 AND tenant_id = $2",
            )
            .bind(produto_id)
            .bind(current_tenant_id()?)
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
            Self::avisar_tributo_obrigatorio_ausente(&ctx, &aliquotas, &ncm);

            // Base tributável = subtotal do item − quota do desconto rateado.
            // Com desconto zero a base é o subtotal íntegro (números idênticos
            // aos de antes do desconto existir).
            let base = item.preco_unitario_centavos * item.quantidade as i64 - desconto_item;
            let imposto = MotorTributario::calcular_item(&ctx, &aliquotas, &classe, base);
            // CFOP dinâmico por operação/UF. tem_st fica em false enquanto o
            // ICMS-ST (marcação por classe/NCM) não é modelado — ver issue #16.
            let cfop = resolver_cfop(
                TipoOperacao::Venda,
                &uf_emitente,
                uf_destinatario.as_deref(),
                modelo,
                false,
            );
            result.push(ItemNF::novo(
                produto_id,
                item.sku.clone(),
                item.descricao.clone(),
                ncm,
                cfop.into(),
                item.quantidade,
                item.preco_unitario_centavos,
                imposto,
            )?);
        }
        Ok((result, ibs_cbs_informativo))
    }

    /// Zero silencioso (issue #10): um tributo OBRIGATÓRIO na fase que resolve
    /// sem nenhuma linha vigente vira imposto 0 mudo — provavelmente seed/
    /// configuração faltando para a UF/regime do tenant. Avisa em log com o
    /// contexto completo em vez de deixar a NF sair "limpa" sem ninguém notar.
    fn avisar_tributo_obrigatorio_ausente(
        ctx: &ContextoFiscal,
        aliquotas: &AliquotasItem,
        ncm: &str,
    ) {
        let tenant = current_tenant_id().ok();
        let uf = ctx.perfil.uf.as_str();
        let regime = ctx.perfil.regime.as_str();
        let avisar = |tributo: &str| {
            tracing::warn!(
                ?tenant,
                uf,
                regime,
                ncm,
                tributo,
                "nenhuma alíquota vigente resolvida para tributo obrigatório na fase — imposto sairá 0"
            );
        };
        // ICMS: obrigatório nas fases com legado, para os regimes normais
        // (o Simples configurado não destaca; o fallback legado usa a linha de SP).
        let regime_normal = matches!(
            ctx.perfil.regime,
            RegimeTributario::LucroPresumido | RegimeTributario::LucroReal
        );
        if ctx.fase.cobra_legado_estadual() && regime_normal && aliquotas.icms.is_none() {
            avisar("icms");
        }
        // CBS/IBS: obrigatórios (ainda que informativos) nas fases que destacam.
        if ctx.fase.destaca_ibs_cbs() {
            if aliquotas.cbs.is_none() {
                avisar("cbs");
            }
            if aliquotas.ibs_uf.is_none() {
                avisar("ibs_uf");
            }
            if aliquotas.ibs_mun.is_none() {
                avisar("ibs_mun");
            }
        }
    }

    /// Alíquota efetiva (bps) que é CUSTO do vendedor para cada produto ativo,
    /// na fase vigente hoje e no perfil do tenant. Reusa o mesmo motor da
    /// emissão — a precificação assistida consome isto no lugar do imposto
    /// manual único, refletindo a reforma automaticamente conforme as fases
    /// avançam. Resolução por (classe, ncm) é cacheada para não repetir I/O.
    pub async fn listar_aliquota_efetiva_produtos(
        &self,
    ) -> Result<Vec<AliquotaEfetivaProduto>, AppError> {
        // Base nominal alta para a alíquota efetiva ter boa precisão em bps
        // (R$ 10.000,00). bps = custo_vendedor × 10_000 / base.
        const BASE_CENTAVOS: i64 = 1_000_000;

        let perfil = self
            .tenants
            .obter_perfil_fiscal()
            .await?
            .para_dominio()?
            .unwrap_or_else(PerfilFiscal::padrao_legado);
        let data = hoje_brasil();
        let ctx = ContextoFiscal {
            fase: FaseTransicao::de_data(data),
            perfil,
        };
        let informativo = ctx.perfil.ibs_cbs_informativo();

        let produtos: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT produto_id, ncm, c_class_trib FROM proj_produtos
             WHERE ativo AND tenant_id = $1",
        )
        .bind(current_tenant_id()?)
        .fetch_all(self.repo.pool())
        .await
        .map_err(AppError::infra)?;

        // Cache por (classe, ncm): produtos com mesma classe/NCM têm a mesma
        // alíquota efetiva — evita reresolver alíquotas por produto.
        let mut cache: std::collections::HashMap<(String, String), i32> =
            std::collections::HashMap::new();
        let mut result = Vec::with_capacity(produtos.len());
        for (produto_id, ncm, classe_produto) in produtos {
            let classe_key = classe_produto.clone().unwrap_or_default();
            let chave = (classe_key, ncm.clone());
            let bps = if let Some(bps) = cache.get(&chave) {
                *bps
            } else {
                let classe_vo = classe_produto
                    .map(ClasseTributaria::try_from)
                    .transpose()
                    .map_err(AppError::Domain)?;
                let classe = self.aliquotas.classe_info(classe_vo.as_ref()).await?;
                let aliquotas = self
                    .aliquotas
                    .resolver(data, &ctx.perfil, &classe.classe, &ncm)
                    .await?;
                let imposto = MotorTributario::calcular_item(&ctx, &aliquotas, &classe, BASE_CENTAVOS);
                let custo = imposto.custo_vendedor_centavos(informativo);
                let bps = (custo * 10_000 / BASE_CENTAVOS) as i32;
                cache.insert(chave, bps);
                bps
            };
            result.push(AliquotaEfetivaProduto {
                produto_id,
                imposto_efetivo_bps: bps,
            });
        }
        Ok(result)
    }
}
