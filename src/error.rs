use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::models::ErrorResponse;

#[derive(Debug, Error)]
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
    #[error("invalid port")]
    InvalidPort,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
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
            Self::InvalidPort => "invalid_port",
            Self::InvalidRequest(_) => "invalid_request",
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
            Self::InvalidPort => StatusCode::BAD_REQUEST,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
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
