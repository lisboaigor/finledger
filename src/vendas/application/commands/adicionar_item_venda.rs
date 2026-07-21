use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::disponibilidade::resolver_disponibilidade;
use crate::shared::Dinheiro;
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
    pub preco_unitario_centavos: i64,
    /// Confirmação explícita do vendedor para vender acima do saldo em
    /// estoque (venda sob encomenda). Produtos com `controla_estoque = false`
    /// (serviços) já ficam de fora da checagem independentemente disto.
    #[serde(default)]
    pub vender_sem_estoque: bool,
}

impl CommandHandler<AdicionarItemVenda> for VendasHandlers {
    type Output = uuid::Uuid;
    type Error = AppError;

    async fn handle(&self, cmd: AdicionarItemVenda) -> Result<uuid::Uuid, AppError> {
        let mut venda = self.load(VendaId::from_uuid(cmd.venda_id)).await?;

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
            Dinheiro::from_centavos(cmd.preco_unitario_centavos),
            disponibilidade,
        )?;
        self.salvar(&mut venda).await?;
        Ok(item_id)
    }
}
