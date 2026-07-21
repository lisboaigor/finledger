use pharos_app::{ApplicationError, EventBus, save_and_publish};
use pharos_core::{AggregateRoot, DomainError, DomainResult, Repository, ValueObject};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub mod tenant;
pub mod tenant_repository;

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
        let reais = self.0 / 100;
        let cents = self.0.abs() % 100;
        write!(f, "R$ {reais},{cents:02}")
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
