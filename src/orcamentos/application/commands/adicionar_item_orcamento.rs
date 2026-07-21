use pharos_app::CommandHandler;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::estoque::application::disponibilidade::resolver_disponibilidade;
use crate::orcamentos::application::handler::OrcamentosHandlers;
use crate::orcamentos::domain::orcamento::OrcamentoId;
use crate::shared::Dinheiro;

#[external_fields]
#[derive(Command, Deserialize)]
pub struct AdicionarItemOrcamento {
    #[external]
    pub orcamento_id: Uuid,
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub quantidade: u32,
    pub preco_unitario_centavos: i64,
}

impl CommandHandler<AdicionarItemOrcamento> for OrcamentosHandlers {
    type Output = uuid::Uuid;
    type Error = AppError;

    async fn handle(&self, cmd: AdicionarItemOrcamento) -> Result<uuid::Uuid, AppError> {
        let mut orcamento = self.load(OrcamentoId::from_uuid(cmd.orcamento_id)).await?;

        // Feature flag self-service (por tenant): quando ligada, orçamentos
        // podem ter itens acima do saldo em estoque (ex.: venda sob encomenda).
        // Só a leitura mora aqui — a regra em si é aplicada dentro do
        // agregado, ver Orcamento::adicionar_item.
        let ignorar = self.tenants.permite_orcamento_sem_estoque().await?;
        let disponibilidade =
            resolver_disponibilidade(&self.pool, cmd.produto_id, ignorar).await?;

        let item_id = orcamento.adicionar_item(
            cmd.produto_id,
            cmd.sku,
            cmd.descricao,
            cmd.quantidade,
            Dinheiro::from_centavos(cmd.preco_unitario_centavos),
            disponibilidade,
        )?;
        self.salvar(&mut orcamento).await?;
        Ok(item_id)
    }
}
