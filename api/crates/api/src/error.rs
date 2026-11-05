//! Typed API errors rendered as RFC 9457 `application/problem+json`, always with the request id.

use crate::request_id::current_request_id;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use reelearn_db::DbError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("validation failed")]
    Validation(Vec<FieldError>),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    PaymentRequired(String),
    #[error("{0}")]
    Locked(String),
    #[error("{0}")]
    UnprocessableEntity(String),
    #[error("too many requests")]
    TooManyRequests { retry_after_seconds: u64 },
    #[error("{0}")]
    NotImplemented(String),
    #[error("{0}")]
    ServiceUnavailable(String),
    #[error("internal error")]
    Internal(#[source] anyhow::Error),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            ApiError::Locked(_) => StatusCode::LOCKED,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            ApiError::BadRequest(_) => "Bad request",
            ApiError::Validation(_) => "Validation failed",
            ApiError::Unauthorized(_) => "Unauthorized",
            ApiError::Forbidden(_) => "Forbidden",
            ApiError::NotFound(_) => "Not found",
            ApiError::Conflict(_) => "Conflict",
            ApiError::PaymentRequired(_) => "Not enough credits",
            ApiError::Locked(_) => "Locked",
            ApiError::UnprocessableEntity(_) => "Unprocessable entity",
            ApiError::TooManyRequests { .. } => "Too many requests",
            ApiError::NotImplemented(_) => "Not implemented",
            ApiError::ServiceUnavailable(_) => "Service unavailable",
            ApiError::Internal(_) => "Internal server error",
        }
    }

    fn type_uri(&self) -> String {
        let slug = match self {
            ApiError::Validation(_) => "validation",
            ApiError::PaymentRequired(_) => "insufficient-credits",
            ApiError::TooManyRequests { .. } => "rate-limited",
            ApiError::NotImplemented(_) => "not-implemented",
            ApiError::Locked(_) => "locked",
            ApiError::Conflict(_) => "conflict",
            ApiError::Unauthorized(_) => "unauthorized",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::NotFound(_) => "not-found",
            _ => "about:blank",
        };
        if slug == "about:blank" {
            slug.to_string()
        } else {
            format!("https://reelearn.app/problems/{slug}")
        }
    }

    pub fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
        ApiError::Internal(e.into())
    }

    pub fn not_implemented(what: &str) -> ApiError {
        ApiError::NotImplemented(format!("{what} is not implemented yet"))
    }

    pub fn field(field: &str, message: &str) -> ApiError {
        ApiError::Validation(vec![FieldError {
            field: field.into(),
            message: message.into(),
        }])
    }
}

#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    type_: String,
    title: String,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<FieldError>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        } else {
            tracing::debug!(error = %self, "request rejected");
        }
        let detail = match &self {
            ApiError::Internal(_) => None,
            ApiError::Validation(_) => Some("One or more fields are invalid".to_string()),
            ApiError::TooManyRequests {
                retry_after_seconds,
            } => Some(format!("Try again in {retry_after_seconds} seconds")),
            other => Some(other.to_string()),
        };
        let errors = match &self {
            ApiError::Validation(v) => Some(v.clone()),
            _ => None,
        };
        let body = Problem {
            type_: self.type_uri(),
            title: self.title().to_string(),
            status: status.as_u16(),
            detail,
            request_id: current_request_id(),
            errors,
        };
        let json = serde_json::to_vec(&body)
            .unwrap_or_else(|_| b"{\"title\":\"Internal server error\",\"status\":500}".to_vec());
        let mut response = (status, json).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/problem+json"),
        );
        if let ApiError::TooManyRequests {
            retry_after_seconds,
        } = &self
        {
            if let Ok(v) = header::HeaderValue::from_str(&retry_after_seconds.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, v);
            }
        }
        response
    }
}

impl From<DbError> for ApiError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound(what) => ApiError::NotFound(format!("{what} not found")),
            DbError::Conflict(msg) => ApiError::Conflict(msg),
            DbError::DualApprovalRequired => {
                ApiError::Conflict(DbError::DualApprovalRequired.to_string())
            }
            DbError::InsufficientCredits { needed, available } => ApiError::PaymentRequired(
                format!("this needs {needed} credits; {available} available"),
            ),
            DbError::Invalid(msg) => ApiError::UnprocessableEntity(msg),
            other => ApiError::Internal(other.into()),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        DbError::from(e).into()
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(e: redis::RedisError) -> Self {
        ApiError::ServiceUnavailable(format!("cache unavailable: {e}"))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("invalid JSON: {e}"))
    }
}

impl From<axum::extract::rejection::JsonRejection> for ApiError {
    fn from(e: axum::extract::rejection::JsonRejection) -> Self {
        ApiError::BadRequest(e.body_text())
    }
}

impl From<axum::extract::rejection::QueryRejection> for ApiError {
    fn from(e: axum::extract::rejection::QueryRejection) -> Self {
        ApiError::BadRequest(e.body_text())
    }
}

impl From<axum::extract::rejection::PathRejection> for ApiError {
    fn from(e: axum::extract::rejection::PathRejection) -> Self {
        ApiError::BadRequest(e.body_text())
    }
}
