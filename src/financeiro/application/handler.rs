use std::sync::Arc;

use chrono::{DateTime, Utc};
use pharos_app::EventBus;
use pharos_core::Repository;
use uuid::Uuid;

use crate::error::AppError;
use crate::financeiro::domain::conta_pagar::{ContaPagar, ContaPagarId};
use crate::financeiro::domain::conta_receber::{ContaReceber, ContaReceberId};
use crate::financeiro::infrastructure::repository::{
    PostgresContaPagarRepository, PostgresContaReceberRepository,
};
use crate::shared::{Dinheiro, load_aggregate, salvar_aggregate};

/// Namespace fixo do projeto para ids determinísticos (UUID v5) do financeiro.
/// Com ids determinísticos, a re-entrega do mesmo evento cross-BC
/// (at-least-once) converge para a mesma conta em vez de duplicá-la.
fn ns_financeiro() -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, b"https://finledger.com.br/financeiro")
}

/// Id determinístico da CR criada por `VendaConfirmada` (uma por parcela).
pub fn conta_receber_id_da_venda(venda_id: Uuid, parcela: u32) -> ContaReceberId {
    let nome = format!("conta-receber:{venda_id}:{parcela}");
    ContaReceberId::from_uuid(Uuid::new_v5(&ns_financeiro(), nome.as_bytes()))
}

/// Id determinístico da CP criada por `MercadoriaRecebida` (um recebimento).
pub fn conta_pagar_id_do_recebimento(pedido_id: Uuid, recebimento_id: &str) -> ContaPagarId {
    let nome = format!("conta-pagar:{pedido_id}:{recebimento_id}");
    ContaPagarId::from_uuid(Uuid::new_v5(&ns_financeiro(), nome.as_bytes()))
}

/// Id determinístico de uma CP de reembolso ao cliente (`nome` identifica a
/// origem: conta estornada na devolução total, evento na parcial).
fn conta_pagar_id_reembolso(nome: &str) -> ContaPagarId {
    ContaPagarId::from_uuid(Uuid::new_v5(&ns_financeiro(), nome.as_bytes()))
}

/// Plano de uma conta a receber derivada de `VendaConfirmada` — o event
/// handler traduz a forma de pagamento para esta lista e o handler cria/paga.
pub struct ParcelaReceber {
    /// Índice estável da parcela (entra no id determinístico).
    pub indice: u32,
    pub valor: Dinheiro,
    pub vencimento: DateTime<Utc>,
    pub descricao: Option<String>,
    /// Pagamento à vista: o dinheiro entrou no ato da venda, a conta já nasce
    /// recebida (Liquidada).
    pub liquidar_imediatamente: bool,
}

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

    /// Cria as contas a receber de uma venda confirmada, uma por parcela, com
    /// ids determinísticos. Parcela cujo id já existe no event store é pulada
    /// (re-entrega do mesmo `VendaConfirmada` — at-least-once).
    pub async fn criar_contas_receber_da_venda(
        &self,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        parcelas: Vec<ParcelaReceber>,
    ) -> Result<(), AppError> {
        for parcela in parcelas {
            let id = conta_receber_id_da_venda(venda_id, parcela.indice);
            let ja_existe = self
                .repo_receber
                .find_by_id(&id)
                .await
                .map_err(AppError::infra)?
                .is_some();
            if ja_existe {
                tracing::info!(
                    %venda_id,
                    parcela = parcela.indice,
                    conta_id = %id,
                    "conta a receber já existe para esta parcela — evento reprocessado, pulando"
                );
                continue;
            }
            let mut conta = ContaReceber::criar_com_id(
                id,
                venda_id,
                cliente_id,
                parcela.valor,
                parcela.vencimento,
                parcela.descricao,
            );
            if parcela.liquidar_imediatamente && parcela.valor.centavos() > 0 {
                conta.registrar_pagamento(parcela.valor)?;
            }
            self.salvar_receber(&mut conta).await?;
        }
        Ok(())
    }

    /// Cria a CP de um recebimento de mercadoria com id determinístico,
    /// pulando se já existir (re-entrega do mesmo `MercadoriaRecebida`).
    pub async fn criar_conta_pagar_do_recebimento(
        &self,
        pedido_id: Uuid,
        recebimento_id: &str,
        fornecedor_id: Uuid,
        valor: Dinheiro,
        vencimento: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let id = conta_pagar_id_do_recebimento(pedido_id, recebimento_id);
        let ja_existe = self
            .repo_pagar
            .find_by_id(&id)
            .await
            .map_err(AppError::infra)?
            .is_some();
        if ja_existe {
            tracing::info!(
                %pedido_id,
                recebimento_id,
                conta_id = %id,
                "conta a pagar já existe para este recebimento — evento reprocessado, pulando"
            );
            return Ok(());
        }
        let mut conta = ContaPagar::criar_com_id(id, pedido_id, fornecedor_id, valor, vencimento, None);
        self.salvar_pagar(&mut conta).await
    }

    /// Devolução TOTAL (venda desfeita): estorna TODAS as contas a receber da
    /// venda — inclusive parciais e liquidadas — e, para cada valor já
    /// recebido, cria uma ContaPagar de reembolso ao cliente.
    pub async fn processar_devolucao_total(
        &self,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        motivo: &str,
    ) -> Result<(), AppError> {
        for conta_id in self.repo_receber.contas_por_venda(venda_id).await? {
            let mut conta = self
                .load_receber(ContaReceberId::from_uuid(conta_id))
                .await?;
            let recebido = conta.valor_recebido;
            if let Err(err) = conta.estornar(format!("Devolução total: {motivo}")) {
                // "Já estornada" = projeção/reprocessamento defasado; só pular.
                tracing::warn!(%venda_id, %conta_id, error = %err, "conta não estornável, pulando");
                continue;
            }
            self.salvar_receber(&mut conta).await?;
            if recebido.centavos() > 0 {
                self.criar_reembolso_cliente(
                    conta_pagar_id_reembolso(&format!("reembolso:{conta_id}")),
                    venda_id,
                    cliente_id,
                    recebido,
                    format!("Reembolso ao cliente — devolução total da venda {venda_id}"),
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Devolução PARCIAL (venda segue confirmada): abate o valor devolvido do
    /// saldo em aberto das contas da venda, em ordem de vencimento. O que não
    /// couber no saldo (parte já recebida) vira reembolso ao cliente.
    pub async fn processar_devolucao_parcial(
        &self,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        valor_devolvido: Dinheiro,
        motivo: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut restante = valor_devolvido.centavos();
        for conta_id in self.repo_receber.contas_por_venda(venda_id).await? {
            if restante <= 0 {
                break;
            }
            let mut conta = self
                .load_receber(ContaReceberId::from_uuid(conta_id))
                .await?;
            let saldo = conta.saldo_devedor().centavos();
            if saldo <= 0 {
                continue;
            }
            let abate = saldo.min(restante);
            conta.abater(
                Dinheiro::from_centavos(abate),
                format!("Devolução parcial: {motivo}"),
            )?;
            self.salvar_receber(&mut conta).await?;
            restante -= abate;
        }
        if restante > 0 {
            // O excedente já tinha sido recebido — devolver em dinheiro.
            // O id inclui o instante do evento para distinguir devoluções
            // parciais sucessivas da mesma venda, mantendo o reprocessamento
            // do MESMO evento idempotente.
            let nome = format!(
                "reembolso-parcial:{venda_id}:{}",
                occurred_at.timestamp_micros()
            );
            self.criar_reembolso_cliente(
                conta_pagar_id_reembolso(&nome),
                venda_id,
                cliente_id,
                Dinheiro::from_centavos(restante),
                format!("Reembolso ao cliente — devolução parcial da venda {venda_id}"),
            )
            .await?;
        }
        Ok(())
    }

    /// CP de reembolso ao cliente: sem schema próprio, reaproveita a ContaPagar
    /// com `pedido_id` = venda devolvida e `fornecedor_id` = cliente (nulo em
    /// venda de balcão sem cliente identificado); a descrição deixa o credor
    /// claro. Vencimento imediato: reembolso se paga no ato.
    async fn criar_reembolso_cliente(
        &self,
        id: ContaPagarId,
        venda_id: Uuid,
        cliente_id: Option<Uuid>,
        valor: Dinheiro,
        descricao: String,
    ) -> Result<(), AppError> {
        let ja_existe = self
            .repo_pagar
            .find_by_id(&id)
            .await
            .map_err(AppError::infra)?
            .is_some();
        if ja_existe {
            tracing::info!(%venda_id, conta_id = %id, "reembolso já registrado — evento reprocessado, pulando");
            return Ok(());
        }
        let mut conta = ContaPagar::criar_com_id(
            id,
            venda_id,
            cliente_id.unwrap_or_else(Uuid::nil),
            valor,
            Utc::now(),
            Some(descricao),
        );
        self.salvar_pagar(&mut conta).await
    }
}
