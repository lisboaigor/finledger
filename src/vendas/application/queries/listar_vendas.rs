use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct VendaResult {
    pub venda_id: Uuid,
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
    /// Total LÍQUIDO (bruto dos itens − desconto).
    pub total_centavos: i64,
    /// Desconto global da venda (0 quando não há) — campo aditivo.
    #[sqlx(default)]
    #[serde(default)]
    pub desconto_centavos: i64,
    pub status: String,
    pub forma_pagamento: Option<String>,
}

#[derive(Query)]
#[query(result = Vec<VendaResult>)]
pub struct ListarVendas {
    /// Filtra vendas que contenham um item cujo SKU ou descrição casa (ILIKE)
    /// com o termo — permite achar uma venda pelo produto vendido nela.
    pub produto_busca: Option<String>,
    /// Só vendas EmAndamento (recuperáveis no PDV). Padrão: todas.
    pub apenas_abertas: bool,
    /// Paginação opcional (aditivo): sem os params, as 200 primeiras.
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}

impl QueryHandler<ListarVendas> for VendasHandlers {
    type Error = AppError;

    async fn handle(&self, query: ListarVendas) -> Result<Vec<VendaResult>, AppError> {
        self.repo
            .listar(
                query.produto_busca,
                query.apenas_abertas,
                query.limite,
                query.offset,
            )
            .await
    }
}
