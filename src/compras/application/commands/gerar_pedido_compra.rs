use pharos_app::CommandHandler;
use pharos_core::Entity;
use pharos_macros::{Command, external_fields};
use serde::Deserialize;
use uuid::Uuid;

use crate::compras::application::handler::ComprasHandlers;
use crate::compras::domain::pedido_compra::{PedidoCompra, PedidoCompraId};
use crate::error::AppError;
use crate::shared::Dinheiro;

#[derive(Debug, Deserialize)]
pub struct ItemPedidoInput {
    pub produto_id: Uuid,
    pub quantidade: u32,
    pub custo_unitario_centavos: i64,
}

#[external_fields]
#[derive(Command, Deserialize)]
pub struct GerarPedidoCompra {
    #[external]
    pub comprador_id: Uuid,
    pub fornecedor_id: Uuid,
    pub itens: Vec<ItemPedidoInput>,
    pub prazo_pagamento_dias: u16,
}

impl CommandHandler<GerarPedidoCompra> for ComprasHandlers {
    type Output = PedidoCompraId;
    type Error = AppError;

    async fn handle(&self, cmd: GerarPedidoCompra) -> Result<PedidoCompraId, AppError> {
        let itens = cmd
            .itens
            .into_iter()
            .map(|i| {
                (
                    i.produto_id,
                    i.quantidade,
                    Dinheiro::from_centavos(i.custo_unitario_centavos),
                )
            })
            .collect();
        let mut pedido = PedidoCompra::gerar(
            cmd.comprador_id,
            cmd.fornecedor_id,
            itens,
            cmd.prazo_pagamento_dias,
        )?;
        let id = *pedido.id();
        self.salvar(&mut pedido).await?;
        Ok(id)
    }
}
