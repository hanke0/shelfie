use axum::{
    http::{header::WWW_AUTHENTICATE, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("{0}")]
    Internal(String),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

/// Prompts OPDS / HTTP Basic clients to show a login dialog.
pub const WWW_AUTH_BASIC_REALM: &str = r#"Basic realm="Authentication Required""#;

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Sqlx(_)
            | AppError::Json(_)
            | AppError::Io(_)
            | AppError::Jwt(_)
            | AppError::Bcrypt(_)
            | AppError::Migrate(_)
            | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            _ => "internal_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let message = self.to_string();

        if status.is_server_error() {
            tracing::error!(
                status = %status.as_u16(),
                error_code = code,
                message = %message,
                err = ?self,
                "internal server error"
            );
        } else if status.is_client_error() {
            tracing::warn!(
                status = %status.as_u16(),
                error_code = code,
                message = %message,
                "client error"
            );
        }

        let body = ApiErrorBody {
            code: code.to_string(),
            message,
        };
        if matches!(&self, AppError::Unauthorized(_)) {
            return (
                status,
                [(WWW_AUTHENTICATE, WWW_AUTH_BASIC_REALM)],
                Json(body),
            )
                .into_response();
        }
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_code_mapping() {
        let cases = [
            (AppError::BadRequest("x".into()), 400, "bad_request"),
            (AppError::Unauthorized("x".into()), 401, "unauthorized"),
            (AppError::Forbidden("x".into()), 403, "forbidden"),
            (AppError::NotFound("x".into()), 404, "not_found"),
            (AppError::Conflict("x".into()), 409, "conflict"),
            (AppError::Internal("x".into()), 500, "internal_error"),
            (
                AppError::Io(std::io::Error::other("disk")),
                500,
                "internal_error",
            ),
        ];
        for (err, status, code) in cases {
            assert_eq!(err.status().as_u16(), status);
            assert_eq!(err.code(), code);
        }
    }

    #[test]
    fn only_unauthorized_sends_basic_challenge() {
        let res = AppError::Unauthorized("x".into()).into_response();
        assert_eq!(res.headers()[WWW_AUTHENTICATE], WWW_AUTH_BASIC_REALM);
        let res = AppError::Forbidden("x".into()).into_response();
        assert!(!res.headers().contains_key(WWW_AUTHENTICATE));
    }
}
