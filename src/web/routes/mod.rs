pub mod auth;
pub mod backoffice;
pub mod bi;
pub mod catalogo;
pub mod compras;
pub mod configuracoes;
pub mod crm;
pub mod estoque;
pub mod financeiro;
pub mod fiscal;
pub mod fornecedores;
pub mod orcamentos;
pub mod tenants_publico;
pub mod tls;
pub mod vendas;

use serde::Deserialize;

/// Paginação opcional das listagens (aditivo — sem os params o comportamento é
/// o histórico). O clamp dos limites fica na camada de repositório
/// (`shared::normalizar_paginacao`). Compartilhado por todas as rotas de
/// listagem para não redefinir o mesmo struct por contexto.
#[derive(Deserialize, Default)]
pub struct PaginacaoParams {
    pub limite: Option<i64>,
    pub offset: Option<i64>,
}
