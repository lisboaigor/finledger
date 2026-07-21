use std::sync::Arc;

use pharos_app::EventBus;

use crate::{
    catalogo::{
        domain::{Produto, ProdutoId},
        infrastructure::{
            precificacao_repository::PostgresPrecificacaoRepository,
            repository::PostgresProdutoRepository,
        },
    },
    error::AppError,
    shared::{load_aggregate, salvar_aggregate},
};

pub type ProdutoRepository = Arc<PostgresProdutoRepository>;
pub type PrecificacaoRepository = Arc<PostgresPrecificacaoRepository>;

pub struct CatalogoHandlers {
    pub(crate) repo: ProdutoRepository,
    pub(crate) precificacao: PrecificacaoRepository,
    pub(crate) bus: EventBus,
}

impl CatalogoHandlers {
    pub fn new(repo: ProdutoRepository, precificacao: PrecificacaoRepository, bus: EventBus) -> Self {
        Self {
            repo,
            precificacao,
            bus,
        }
    }

    pub(crate) async fn load(&self, id: ProdutoId) -> Result<Produto, AppError> {
        load_aggregate(&*self.repo, &id).await
    }

    pub(crate) async fn salvar(&self, produto: &mut Produto) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo, &self.bus, produto).await
    }
}
