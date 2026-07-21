use pharos_app::QueryHandler;
use pharos_macros::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::fornecedores::application::handler::FornecedoresHandlers;

#[derive(Serialize, sqlx::FromRow)]
pub struct FornecedorResult {
    pub fornecedor_id: Uuid,
    pub razao_social: String,
    pub cnpj: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub prazo_pagamento_dias: i32,
    pub ativo: bool,
}

#[derive(Query)]
#[query(result = Vec<FornecedorResult>)]
pub struct ListarFornecedores;

impl QueryHandler<ListarFornecedores> for FornecedoresHandlers {
    type Error = AppError;

    async fn handle(&self, _query: ListarFornecedores) -> Result<Vec<FornecedorResult>, AppError> {
        self.repo.listar().await
    }
}
