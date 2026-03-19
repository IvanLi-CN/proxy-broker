use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::models::ErrorResponse;

#[derive(Debug, Clone, Error)]
pub enum BrokerError {
    #[error("invalid subscription payload")]
    SubscriptionInvalid,
    #[error("subscription source not reachable: {0}")]
    SubscriptionFetch(String),
    #[error("no candidate ip found")]
    IpNotFound,
    #[error("specified_ips intersects blacklist_ips")]
    IpConflictBlacklist(Vec<String>),
    #[error("session not found")]
    SessionNotFound,
    #[error("port already in use")]
    PortInUse,
    #[error("profile already exists")]
    ProfileExists,
    #[error("profile not found")]
    ProfileNotFound,
    #[error("invalid port")]
    InvalidPort,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("authentication required")]
    AuthenticationRequired,
    #[error("admin access required")]
    AdminRequired,
    #[error("api key invalid")]
    ApiKeyInvalid,
    #[error("api key revoked")]
    ApiKeyRevoked,
    #[error("api key not found")]
    ApiKeyNotFound,
    #[error("profile access denied")]
    ProfileAccessDenied,
    #[error("mihomo runtime unavailable: {0}")]
    MihomoUnavailable(String),
    #[error("batch open failed")]
    BatchOpenFailed,
    #[error("internal error: {0}")]
    Internal(String),
}

impl BrokerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SubscriptionInvalid => "subscription_invalid",
            Self::SubscriptionFetch(_) => "subscription_fetch_failed",
            Self::IpNotFound => "ip_not_found",
            Self::IpConflictBlacklist(_) => "ip_conflict_blacklist",
            Self::SessionNotFound => "session_not_found",
            Self::PortInUse => "port_in_use",
            Self::ProfileExists => "profile_exists",
            Self::ProfileNotFound => "profile_not_found",
            Self::InvalidPort => "invalid_port",
            Self::InvalidRequest(_) => "invalid_request",
            Self::AuthenticationRequired => "authentication_required",
            Self::AdminRequired => "admin_required",
            Self::ApiKeyInvalid => "api_key_invalid",
            Self::ApiKeyRevoked => "api_key_revoked",
            Self::ApiKeyNotFound => "api_key_not_found",
            Self::ProfileAccessDenied => "profile_access_denied",
            Self::MihomoUnavailable(_) => "mihomo_unavailable",
            Self::BatchOpenFailed => "batch_open_failed",
            Self::Internal(_) => "internal_error",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::SubscriptionInvalid => StatusCode::BAD_REQUEST,
            Self::SubscriptionFetch(_) => StatusCode::BAD_GATEWAY,
            Self::IpNotFound => StatusCode::NOT_FOUND,
            Self::IpConflictBlacklist(_) => StatusCode::BAD_REQUEST,
            Self::SessionNotFound => StatusCode::NOT_FOUND,
            Self::PortInUse => StatusCode::CONFLICT,
            Self::ProfileExists => StatusCode::CONFLICT,
            Self::ProfileNotFound => StatusCode::NOT_FOUND,
            Self::InvalidPort => StatusCode::BAD_REQUEST,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            Self::AdminRequired => StatusCode::FORBIDDEN,
            Self::ApiKeyInvalid => StatusCode::UNAUTHORIZED,
            Self::ApiKeyRevoked => StatusCode::UNAUTHORIZED,
            Self::ApiKeyNotFound => StatusCode::NOT_FOUND,
            Self::ProfileAccessDenied => StatusCode::FORBIDDEN,
            Self::MihomoUnavailable(_) => StatusCode::BAD_GATEWAY,
            Self::BatchOpenFailed => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            Self::IpConflictBlacklist(items) => Some(serde_json::json!({ "conflicts": items })),
            _ => None,
        }
    }
}

impl IntoResponse for BrokerError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            code: self.code().to_string(),
            message: self.to_string(),
            details: self.details(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}

impl From<anyhow::Error> for BrokerError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

pub type BrokerResult<T> = Result<T, BrokerError>;
