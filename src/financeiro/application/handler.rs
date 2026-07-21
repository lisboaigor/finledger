use std::sync::Arc;

use pharos_app::EventBus;

use crate::error::AppError;
use crate::financeiro::domain::conta_pagar::{ContaPagar, ContaPagarId};
use crate::financeiro::domain::conta_receber::{ContaReceber, ContaReceberId};
use crate::financeiro::infrastructure::repository::{
    PostgresContaPagarRepository, PostgresContaReceberRepository,
};
use crate::shared::{load_aggregate, salvar_aggregate};

pub struct FinanceiroHandlers {
    pub(crate) repo_receber: Arc<PostgresContaReceberRepository>,
    pub(crate) repo_pagar: Arc<PostgresContaPagarRepository>,
    pub(crate) bus: EventBus,
}

impl FinanceiroHandlers {
    pub fn new(
        repo_receber: Arc<PostgresContaReceberRepository>,
        repo_pagar: Arc<PostgresContaPagarRepository>,
        bus: EventBus,
    ) -> Self {
        Self {
            repo_receber,
            repo_pagar,
            bus,
        }
    }

    pub(crate) async fn load_receber(&self, id: ContaReceberId) -> Result<ContaReceber, AppError> {
        load_aggregate(&*self.repo_receber, &id).await
    }

    pub(crate) async fn load_pagar(&self, id: ContaPagarId) -> Result<ContaPagar, AppError> {
        load_aggregate(&*self.repo_pagar, &id).await
    }

    pub(crate) async fn salvar_receber(&self, conta: &mut ContaReceber) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo_receber, &self.bus, conta).await
    }

    pub(crate) async fn salvar_pagar(&self, conta: &mut ContaPagar) -> Result<(), AppError> {
        salvar_aggregate(&*self.repo_pagar, &self.bus, conta).await
    }

    pub async fn criar_conta_receber(&self, mut conta: ContaReceber) -> Result<(), AppError> {
        self.salvar_receber(&mut conta).await
    }

    pub async fn criar_conta_pagar(&self, mut conta: ContaPagar) -> Result<(), AppError> {
        self.salvar_pagar(&mut conta).await
    }

    /// Estorna todas as contas a receber em aberto de uma venda (devolução total).
    pub async fn estornar_contas_da_venda(
        &self,
        venda_id: uuid::Uuid,
        motivo: String,
    ) -> Result<(), AppError> {
        for conta_id in self.repo_receber.contas_abertas_por_venda(venda_id).await? {
            let mut conta = self
                .load_receber(ContaReceberId::from_uuid(conta_id))
                .await?;
            conta.estornar(motivo.clone())?;
            self.salvar_receber(&mut conta).await?;
        }
        Ok(())
    }
}
