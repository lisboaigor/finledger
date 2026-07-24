use std::collections::HashMap;

use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fiscal::application::handler::FiscalHandlers;
use crate::fiscal::domain::tributacao::{
    ClasseTributaria, ContextoFiscal, FaseTransicao, MotorTributario, hoje_brasil,
};
use crate::fiscal::infrastructure::aliquotas::AliquotaProvider;
use crate::fiscal::infrastructure::repository::ProdutoTributavel;
use crate::fiscal::infrastructure::sefaz::SefazClient;

/// Alíquota efetiva de imposto (em basis points) que é CUSTO do vendedor para
/// um produto, na fase tributária vigente hoje e no perfil fiscal do tenant.
/// Consumida pela precificação assistida no lugar do imposto manual único —
/// reflete a reforma automaticamente conforme as fases avançam (LC 214/2025).
#[derive(Debug, Clone, Serialize)]
pub struct AliquotaEfetivaProduto {
    pub produto_id: Uuid,
    pub imposto_efetivo_bps: i32,
}

/// Alíquota efetiva (bps) que é CUSTO do vendedor para cada produto ativo, na
/// fase vigente hoje e no perfil do tenant. Reusa o mesmo motor da emissão — a
/// precificação assistida consome isto no lugar do imposto manual único,
/// refletindo a reforma automaticamente conforme as fases avançam. Resolução
/// por (classe, ncm) é cacheada para não repetir I/O.
#[derive(Query)]
#[query(result = Vec<AliquotaEfetivaProduto>)]
pub struct ListarAliquotaEfetivaProdutos;

impl<S: SefazClient, A: AliquotaProvider> QueryHandler<ListarAliquotaEfetivaProdutos>
    for FiscalHandlers<S, A>
{
    type Error = AppError;

    async fn handle(
        &self,
        _query: ListarAliquotaEfetivaProdutos,
    ) -> Result<Vec<AliquotaEfetivaProduto>, AppError> {
        // Base nominal alta para a alíquota efetiva ter boa precisão em bps
        // (R$ 10.000,00). bps = custo_vendedor × 10_000 / base.
        const BASE_CENTAVOS: i64 = 1_000_000;

        // Tenant SEM perfil fiscal configurado (ex.: MEI, que paga DAS fixo e
        // não percentual sobre a venda) não recebe imposto efetivo assumido: a
        // precificação assistida cairia num markup irreal ao embutir ~21,65% de
        // regime normal onde não há imposto proporcional. Retorna vazio e o
        // frontend usa o imposto manual do tenant (0 por padrão) — preserva o
        // comportamento anterior ao recurso. Quem CONFIGURA o regime passa a ter
        // a alíquota efetiva real da reforma.
        let Some(perfil) = self.tenants.obter_perfil_fiscal().await?.para_dominio()? else {
            return Ok(Vec::new());
        };
        let data = hoje_brasil();
        let ctx = ContextoFiscal {
            fase: FaseTransicao::de_data(data),
            perfil,
        };
        let informativo = ctx.perfil.ibs_cbs_informativo();

        let produtos = self.repo.listar_produtos_tributaveis().await?;

        // Cache por (classe, ncm): produtos com mesma classe/NCM têm a mesma
        // alíquota efetiva — evita reresolver alíquotas por produto.
        let mut cache: HashMap<(String, String), i32> = HashMap::new();
        let mut result = Vec::with_capacity(produtos.len());
        for produto in produtos {
            let ProdutoTributavel {
                produto_id,
                ncm,
                c_class_trib: classe_produto,
            } = produto;
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
