use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::vendas::application::handler::VendasHandlers;

/// Linha da lixeira: mesma leitura da listagem + quando foi criada/arquivada.
#[derive(Serialize, sqlx::FromRow)]
pub struct VendaArquivadaResult {
    pub venda_id: Uuid,
    pub vendedor_id: Uuid,
    pub cliente_id: Option<Uuid>,
    pub total_centavos: i64,
    pub status: String,
    pub forma_pagamento: Option<String>,
    pub criada_em: chrono::DateTime<chrono::Utc>,
    pub arquivada_em: chrono::DateTime<chrono::Utc>,
}

/// Lixeira: vendas arquivadas pela rotina de limpeza (visão do gestor).
#[derive(Query)]
#[query(result = Vec<VendaArquivadaResult>)]
pub struct ListarVendasArquivadas;

impl QueryHandler<ListarVendasArquivadas> for VendasHandlers {
    type Error = AppError;

    async fn handle(&self, _q: ListarVendasArquivadas) -> Result<Vec<VendaArquivadaResult>, AppError> {
        self.repo.listar_lixeira().await
    }
}
