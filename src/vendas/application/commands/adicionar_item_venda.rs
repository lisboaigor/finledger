use pharos_app::CommandHandler;
use pharos_core::DomainError;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::disponibilidade::resolver_disponibilidade;
use crate::shared::Dinheiro;
use crate::shared::tenant::current_tenant_id;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::domain::venda::VendaId;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AdicionarItemVenda {
    #[external]
    pub venda_id: Uuid,
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub quantidade: u32,
    /// IGNORADO quando o comando vem da API: o preço praticado é SEMPRE o de
    /// tabela (`proj_produtos.preco_venda`), nunca o do payload — um cliente
    /// adulterado não pode vender abaixo do preço. O campo permanece por
    /// retrocompatibilidade (o PDV continua enviando). Só é honrado no caminho
    /// interno da conversão orçamento→venda (`preservar_preco_informado`).
    pub preco_unitario_centavos: i64,
    /// Confirmação explícita do vendedor para vender acima do saldo em
    /// estoque (venda sob encomenda). Produtos com `controla_estoque = false`
    /// (serviços) já ficam de fora da checagem independentemente disto.
    #[serde(default)]
    pub vender_sem_estoque: bool,
    /// Caminho interno (conversão orçamento→venda): preserva o preço acordado
    /// no orçamento aceito em vez de reler o catálogo. `skip_deserializing`
    /// garante que a API jamais consegue ligar esta flag.
    #[serde(skip_deserializing, default)]
    pub preservar_preco_informado: bool,
}

impl CommandHandler<AdicionarItemVenda> for VendasHandlers {
    type Output = uuid::Uuid;
    type Error = AppError;

    async fn handle(&self, cmd: AdicionarItemVenda) -> Result<uuid::Uuid, AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;

        // Preço de tabela do catálogo — fonte de verdade contra adulteração do
        // payload. Filtro por tenant_id explícito além da RLS.
        let preco_unitario_centavos = if cmd.preservar_preco_informado {
            cmd.preco_unitario_centavos
        } else {
            let tenant_id = current_tenant_id()?;
            let preco_venda: Option<i64> = sqlx::query_scalar(
                "SELECT preco_venda FROM proj_produtos
                 WHERE produto_id = $1 AND tenant_id = $2",
            )
            .bind(cmd.produto_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::infra)?;
            preco_venda.ok_or_else(|| {
                AppError::Domain(DomainError::BusinessRule(format!(
                    "Produto {} não encontrado no catálogo — cadastre-o antes de vender",
                    cmd.produto_id
                )))
            })?
        };

        // Só a leitura (I/O) mora na aplicação; a regra de negócio (comparar
        // quantidade pretendida × saldo, somando o que já está na venda) é
        // aplicada dentro do agregado — ver Venda::adicionar_item.
        let disponibilidade =
            resolver_disponibilidade(&self.pool, cmd.produto_id, cmd.vender_sem_estoque).await?;

        let item_id = venda.adicionar_item(
            cmd.produto_id,
            cmd.sku,
            cmd.descricao,
            cmd.quantidade,
            Dinheiro::from_centavos(preco_unitario_centavos),
            disponibilidade,
        )?;
        self.salvar(&mut venda).await?;
        Ok(item_id)
    }
}
