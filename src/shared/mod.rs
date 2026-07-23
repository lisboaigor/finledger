use std::sync::{Arc, OnceLock};

use pharos_app::{ApplicationError, EventBus, Message, save_and_publish};
use pharos_core::{
    AggregateRoot, DomainError, DomainEvent, DomainResult, Repository, RepositoryError, ValueObject,
};
use pharos_postgres::{Pool, SaveAndEnqueueError, TransactionalRepository, save_and_enqueue_in};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::error::AppError;
use crate::shared::tenant::current_tenant_id;

pub mod tenant;
pub mod tenant_repository;

/// Limite padrão de uma listagem quando o cliente não informa `limite`
/// (preserva o comportamento histórico das telas).
pub const PAGINA_LIMITE_PADRAO: i64 = 200;
/// Teto de segurança por página, independentemente do que o cliente pedir —
/// impede que uma listagem varra a projeção inteira num único request.
pub const PAGINA_LIMITE_MAX: i64 = 500;

/// Normaliza paginação vinda de query params: `limite` em `1..=PAGINA_LIMITE_MAX`
/// (default [`PAGINA_LIMITE_PADRAO`]) e `offset` ≥ 0. Compartilhado por todas as
/// listagens para não repetir clamp por contexto.
pub fn normalizar_paginacao(limite: Option<i64>, offset: Option<i64>) -> (i64, i64) {
    (
        limite
            .unwrap_or(PAGINA_LIMITE_PADRAO)
            .clamp(1, PAGINA_LIMITE_MAX),
        offset.unwrap_or(0).max(0),
    )
}

/// Carrega um agregado pelo id, mapeando ausência/erro de infra para [`AppError`].
///
/// Repetido de forma idêntica em cada `*Handlers` de bounded context antes desta
/// extração; centralizado aqui para evitar redivergência entre contextos.
pub async fn load_aggregate<A, R>(repo: &R, id: &A::Id) -> Result<A, AppError>
where
    A: AggregateRoot,
    R: Repository<A>,
{
    repo.find_by_id(id)
        .await
        .map_err(AppError::infra)?
        .ok_or(AppError::NotFound)
}

/// Persiste um agregado e publica seus eventos pendentes, mapeando conflito de
/// concorrência e erros de infra para [`AppError`].
pub async fn salvar_aggregate<A, R>(
    repo: &R,
    bus: &EventBus,
    aggregate: &mut A,
) -> Result<(), AppError>
where
    A: AggregateRoot,
    R: Repository<A>,
{
    save_and_publish(repo, bus, aggregate)
        .await
        .map_err(|e| match e {
            ApplicationError::ConcurrencyConflict { .. } => AppError::Conflict,
            e => AppError::infra(e),
        })
}

/// Sinalizador que "cutuca" o relay do outbox a drenar imediatamente após um
/// commit durável, em vez de esperar o próximo tick — mantém a leitura
/// pós-escrita (projeções) na casa dos milissegundos. Setado uma vez no
/// bootstrap; ausente em testes, que drenam o outbox manualmente
/// (`tests/helpers::drenar_outbox`).
static RELAY_KICK: OnceLock<Arc<Notify>> = OnceLock::new();

/// Registra o sinalizador do relay (chamado no bootstrap). Idempotente: só o
/// primeiro registro vale.
pub fn registrar_relay_kick(notify: Arc<Notify>) {
    let _ = RELAY_KICK.set(notify);
}

fn cutucar_relay() {
    if let Some(n) = RELAY_KICK.get() {
        n.notify_one();
    }
}

/// Liga o caminho durável (outbox + relay). Desligado via
/// `OUTBOX_RELAY_ATIVO=false`/`0`, recai no `salvar_aggregate` síncrono legado —
/// interruptor de rollout para produção.
fn outbox_ativo() -> bool {
    std::env::var("OUTBOX_RELAY_ATIVO")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true)
}

/// Persiste um agregado **produtor** de forma durável: o snapshot e os eventos
/// (como mensagens de outbox) commitam na MESMA transação; o relay
/// (`bootstrap::outbox_relay`) despacha depois as projeções e os efeitos
/// cross-context. Fecha a janela de perda de eventos da issue #3 — um crash
/// após o commit não perde mais a conta a receber, a NF nem a baixa de estoque.
///
/// Com `OUTBOX_RELAY_ATIVO=false` recai no [`salvar_aggregate`] síncrono legado.
///
/// `topic` roteia o evento no relay e DEVE casar com o `register_decoder`
/// registrado no bootstrap (ex.: `"VendaEvent"`).
pub async fn salvar_aggregate_duravel<A, R>(
    pool: &Pool,
    repo: &R,
    bus: &EventBus,
    aggregate: &mut A,
    topic: &'static str,
) -> Result<(), AppError>
where
    A: AggregateRoot,
    A::Event: Serialize,
    R: Repository<A> + TransactionalRepository<A>,
{
    if !outbox_ativo() {
        return salvar_aggregate(repo, bus, aggregate).await;
    }

    // Surfa erro de serialização ANTES de tocar o banco (nada persistido) — e
    // garante que o `map_event` abaixo nunca produza payload vazio por falha.
    for evento in aggregate.pending_events() {
        serde_json::to_vec(evento).map_err(AppError::infra)?;
    }

    let tenant_id = current_tenant_id()?.to_string();
    let map_event = |evento: &A::Event| {
        Message::new(
            topic,
            serde_json::to_vec(evento).unwrap_or_default(),
            "application/json",
        )
        .with_key(evento.aggregate_id())
        .with_header("event_type", evento.event_type())
        .with_header("tenant_id", tenant_id.clone())
    };

    save_and_enqueue_in(pool, repo, aggregate, map_event)
        .await
        .map_err(|e| match e {
            SaveAndEnqueueError::Repository(RepositoryError::ConcurrencyConflict { .. }) => {
                AppError::Conflict
            }
            other => AppError::infra(other),
        })?;

    // Efeitos são despachados pelo relay; cutuca para drenar já.
    cutucar_relay();
    Ok(())
}

/// Valor monetário em centavos de real. Evita imprecisão de ponto flutuante.
///
/// O sinal não é restringido aqui: descontos, estornos e saldos devedores são
/// legitimamente negativos. Regras de positividade (ex: preço de venda > 0)
/// pertencem ao agregado que as exige, não a este value object — ver
/// `Produto::cadastrar`. O campo é privado só para forçar toda construção a
/// passar por `from_centavos`/`zero`, em vez de um literal `Dinheiro(x)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Dinheiro(i64);

impl Dinheiro {
    pub fn from_centavos(centavos: i64) -> Self {
        Self(centavos)
    }
    pub fn centavos(self) -> i64 {
        self.0
    }
    pub fn zero() -> Self {
        Self(0)
    }
}

impl ValueObject for Dinheiro {}

impl std::fmt::Display for Dinheiro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Formata sobre o valor absoluto e prefixa o sinal explicitamente:
        // `self.0 / 100` perderia o sinal para valores entre −1 e −99 centavos
        // (−50 → "R$ 0,50" em vez de "R$ -0,50").
        let sinal = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "R$ {sinal}{},{:02}", abs / 100, abs % 100)
    }
}

// ── Ncm ──────────────────────────────────────────────────────────────────────

/// Código NCM fiscal: exatamente 8 dígitos numéricos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ncm(String);

impl Ncm {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Ncm {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 8 {
            return Err(DomainError::Validation(
                "NCM deve ter exatamente 8 dígitos numéricos".into(),
            ));
        }
        Ok(Self(digits))
    }
}

impl From<Ncm> for String {
    fn from(n: Ncm) -> Self {
        n.0
    }
}

impl std::fmt::Display for Ncm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Ncm {}

// ── CpfCnpj ──────────────────────────────────────────────────────────────────

/// CPF (11 dígitos) ou CNPJ (14 dígitos), normalizado (apenas dígitos).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CpfCnpj(String);

impl CpfCnpj {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_cnpj(&self) -> bool {
        self.0.len() == 14
    }
}

impl TryFrom<String> for CpfCnpj {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 11 && digits.len() != 14 {
            return Err(DomainError::Validation(
                "CPF deve ter 11 dígitos ou CNPJ 14 dígitos".into(),
            ));
        }
        Ok(Self(digits))
    }
}

impl From<CpfCnpj> for String {
    fn from(c: CpfCnpj) -> Self {
        c.0
    }
}

impl std::fmt::Display for CpfCnpj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for CpfCnpj {}

// ── Cnpj ─────────────────────────────────────────────────────────────────────

/// CNPJ: exatamente 14 dígitos numéricos, normalizado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cnpj(String);

impl Cnpj {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Cnpj {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 14 {
            return Err(DomainError::Validation("CNPJ deve ter 14 dígitos".into()));
        }
        Ok(Self(digits))
    }
}

impl From<Cnpj> for String {
    fn from(c: Cnpj) -> Self {
        c.0
    }
}

impl std::fmt::Display for Cnpj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Cnpj {}

// ── Quantidade ────────────────────────────────────────────────────────────────

/// Quantidade de itens: inteiro positivo (> 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Quantidade(u32);

impl Quantidade {
    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Quantidade {
    type Error = DomainError;

    fn try_from(n: u32) -> DomainResult<Self> {
        if n == 0 {
            return Err(DomainError::Validation(
                "Quantidade deve ser maior que zero".into(),
            ));
        }
        Ok(Self(n))
    }
}

impl From<Quantidade> for u32 {
    fn from(q: Quantidade) -> Self {
        q.0
    }
}

impl std::fmt::Display for Quantidade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Quantidade {}

// ── NomeNaoVazio ──────────────────────────────────────────────────────────────

/// String não vazia após trim. Usada para nomes de cliente, descrições, razão social.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NomeNaoVazio(String);

impl NomeNaoVazio {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NomeNaoVazio {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("Nome não pode ser vazio".into()));
        }
        Ok(Self(trimmed))
    }
}

impl From<NomeNaoVazio> for String {
    fn from(n: NomeNaoVazio) -> Self {
        n.0
    }
}

impl std::fmt::Display for NomeNaoVazio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for NomeNaoVazio {}

// ── Sku ───────────────────────────────────────────────────────────────────────

/// Código SKU do produto: string não vazia, sem espaços nas bordas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sku(String);

impl Sku {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sku {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("SKU não pode ser vazio".into()));
        }
        Ok(Self(trimmed))
    }
}

impl From<Sku> for String {
    fn from(s: Sku) -> Self {
        s.0
    }
}

impl std::fmt::Display for Sku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Sku {}

// ── Unidade ───────────────────────────────────────────────────────────────────

/// Unidade de medida do produto (ex: "UN", "L", "KG", "M"). String não vazia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Unidade(String);

impl Unidade {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Unidade {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("Unidade não pode ser vazia".into()));
        }
        Ok(Self(trimmed.to_uppercase()))
    }
}

impl From<Unidade> for String {
    fn from(u: Unidade) -> Self {
        u.0
    }
}

impl std::fmt::Display for Unidade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Unidade {}

// ── Email ─────────────────────────────────────────────────────────────────────

/// Endereço de e-mail: deve conter exatamente um '@' com texto antes e depois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Email {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let s = s.trim().to_lowercase();
        let parts: Vec<&str> = s.splitn(2, '@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() || !parts[1].contains('.')
        {
            return Err(DomainError::Validation("E-mail inválido".into()));
        }
        Ok(Self(s))
    }
}

impl From<Email> for String {
    fn from(e: Email) -> Self {
        e.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Email {}

// ── Telefone ──────────────────────────────────────────────────────────────────

/// Telefone: entre 8 e 15 dígitos numéricos (remove formatação).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Telefone(String);

impl Telefone {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Telefone {
    type Error = DomainError;

    fn try_from(s: String) -> DomainResult<Self> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 8 || digits.len() > 15 {
            return Err(DomainError::Validation(
                "Telefone deve ter entre 8 e 15 dígitos".into(),
            ));
        }
        Ok(Self(digits))
    }
}

impl From<Telefone> for String {
    fn from(t: Telefone) -> Self {
        t.0
    }
}

impl std::fmt::Display for Telefone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ValueObject for Telefone {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Dinheiro, normalizar_paginacao, PAGINA_LIMITE_MAX, PAGINA_LIMITE_PADRAO};

    #[test]
    fn paginacao_aplica_defaults_e_clampa() {
        // Sem params → default histórico, offset 0.
        assert_eq!(normalizar_paginacao(None, None), (PAGINA_LIMITE_PADRAO, 0));
        // Limite acima do teto é clampado; offset negativo vira 0.
        assert_eq!(normalizar_paginacao(Some(10_000), Some(-5)), (PAGINA_LIMITE_MAX, 0));
        // Limite ≤ 0 sobe para 1 (nunca uma página vazia por erro de cliente).
        assert_eq!(normalizar_paginacao(Some(0), Some(40)), (1, 40));
        // Dentro da faixa passa intacto.
        assert_eq!(normalizar_paginacao(Some(50), Some(100)), (50, 100));
    }

    #[test]
    fn display_formata_positivos_e_zero() {
        assert_eq!(Dinheiro::from_centavos(0).to_string(), "R$ 0,00");
        assert_eq!(Dinheiro::from_centavos(50).to_string(), "R$ 0,50");
        assert_eq!(Dinheiro::from_centavos(123_456).to_string(), "R$ 1234,56");
    }

    #[test]
    fn display_preserva_sinal_entre_menos_um_e_menos_99_centavos() {
        // Regressão issue #17: -50 imprimia "R$ 0,50" (sinal perdido).
        assert_eq!(Dinheiro::from_centavos(-1).to_string(), "R$ -0,01");
        assert_eq!(Dinheiro::from_centavos(-50).to_string(), "R$ -0,50");
        assert_eq!(Dinheiro::from_centavos(-99).to_string(), "R$ -0,99");
    }

    #[test]
    fn display_negativos_maiores_que_um_real() {
        assert_eq!(Dinheiro::from_centavos(-100).to_string(), "R$ -1,00");
        assert_eq!(Dinheiro::from_centavos(-123_456).to_string(), "R$ -1234,56");
    }
}
