use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use pharos_core::DomainError;

use crate::error::AppError;

pub struct ApiError(AppError);

impl<E: Into<AppError>> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(e.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.0.to_string()),
            AppError::Conflict => (StatusCode::CONFLICT, self.0.to_string()),
            AppError::Domain(DomainError::Validation(msg)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
            }
            AppError::Domain(DomainError::BusinessRule(msg)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
            }
            AppError::Domain(DomainError::NotFound(msg)) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Domain(DomainError::Conflict(msg)) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Domain(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.0.to_string()),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.0.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.0.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.0.to_string()),
            AppError::Infrastructure(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
