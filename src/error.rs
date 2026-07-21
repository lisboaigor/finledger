use pharos_app::{DispatchError, ValidationError};
use pharos_core::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("entidade não encontrada")]
    NotFound,
    #[error("{0}")]
    Validation(ValidationError),
    #[error("{0}")]
    Domain(#[from] DomainError),
    #[error("erro de infraestrutura: {0}")]
    Infrastructure(String),
    #[error("conflito de concorrência — tente novamente")]
    Conflict,
    #[error("credenciais inválidas")]
    Unauthorized,
    #[error("acesso negado: permissão insuficiente")]
    Forbidden,
}

impl AppError {
    pub fn infra(e: impl std::fmt::Display) -> Self {
        Self::Infrastructure(e.to_string())
    }
}

impl From<DispatchError<AppError>> for AppError {
    fn from(e: DispatchError<AppError>) -> Self {
        match e {
            DispatchError::Validation(v) => AppError::Validation(v),
            DispatchError::Handler(e) => e,
        }
    }
}
