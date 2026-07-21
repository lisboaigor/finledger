use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;
use crate::vendas::application::queries::listar_vendas::VendaResult;

#[derive(Serialize, sqlx::FromRow)]
pub struct VendaItemResult {
    pub item_id: Uuid,
    pub produto_id: Uuid,
    pub sku: String,
    pub descricao: String,
    pub quantidade: i32,
    pub preco_unitario_centavos: i64,
}

#[derive(Serialize)]
pub struct VendaDetalhes {
    pub venda: VendaResult,
    pub itens: Vec<VendaItemResult>,
}

#[derive(Query)]
#[query(result = Option<VendaDetalhes>)]
pub struct BuscarVenda {
    #[trace(display)]
    pub venda_id: Uuid,
}

impl QueryHandler<BuscarVenda> for VendasHandlers {
    type Error = AppError;

    async fn handle(&self, q: BuscarVenda) -> Result<Option<VendaDetalhes>, AppError> {
        self.repo.buscar(q.venda_id).await
    }
}
