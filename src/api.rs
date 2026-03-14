use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{delete, get, post},
};

use crate::{
    error::BrokerError,
    models::{
        HealthResponse, LoadSubscriptionRequest, OpenBatchRequest, OpenSessionRequest,
        RefreshRequest,
    },
    service::BrokerService,
    web_ui::spa_fallback,
};

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<BrokerService>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/profiles", get(list_profiles))
        .route(
            "/api/v1/profiles/{profile_id}/subscriptions/load",
            post(load_subscription),
        )
        .route(
            "/api/v1/profiles/{profile_id}/summary",
            get(profile_summary),
        )
        .route(
            "/api/v1/profiles/{profile_id}/refresh",
            post(refresh_profile),
        )
        .route(
            "/api/v1/profiles/{profile_id}/ips/extract",
            post(extract_ips),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/open",
            post(open_session),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/open-batch",
            post(open_batch),
        )
        .route("/api/v1/profiles/{profile_id}/sessions", get(list_sessions))
        .route(
            "/api/v1/profiles/{profile_id}/sessions/{session_id}",
            delete(close_session),
        )
        .fallback(spa_fallback)
        .with_state(state)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn load_subscription(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<LoadSubscriptionRequest>, JsonRejection>,
) -> Result<Json<crate::models::LoadSubscriptionResponse>, BrokerError> {
    let request = parse_json_payload(payload, "load_subscription")?;
    let resp = state
        .service
        .load_subscription(&profile_id, &request.source)
        .await?;
    Ok(Json(resp))
}

async fn list_profiles(
    State(state): State<AppState>,
) -> Result<Json<crate::models::ListProfilesResponse>, BrokerError> {
    let resp = state.service.list_profiles().await?;
    Ok(Json(resp))
}

async fn profile_summary(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<crate::models::ProfileSummaryResponse>, BrokerError> {
    let resp = state.service.profile_summary(&profile_id).await?;
    Ok(Json(resp))
}

async fn refresh_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    body: Bytes,
) -> Result<Json<crate::models::RefreshResponse>, BrokerError> {
    let request = decode_refresh_request(&body)?;
    let resp = state.service.refresh(&profile_id, &request).await?;
    Ok(Json(resp))
}

fn decode_refresh_request(body: &[u8]) -> Result<RefreshRequest, BrokerError> {
    if body.is_empty() {
        return Ok(RefreshRequest { force: false });
    }
    serde_json::from_slice::<RefreshRequest>(body)
        .map_err(|err| BrokerError::InvalidRequest(format!("invalid refresh payload: {err}")))
}

fn parse_json_payload<T>(
    payload: Result<Json<T>, JsonRejection>,
    endpoint: &str,
) -> Result<T, BrokerError> {
    payload.map(|Json(value)| value).map_err(|err| {
        BrokerError::InvalidRequest(format!(
            "{endpoint} invalid json payload: {}",
            err.body_text()
        ))
    })
}

async fn extract_ips(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<crate::models::ExtractIpRequest>, JsonRejection>,
) -> Result<Json<crate::models::ExtractIpResponse>, BrokerError> {
    let request = parse_json_payload(payload, "extract_ips")?;
    let resp = state.service.extract_ips(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn open_session(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<OpenSessionRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    let request = parse_json_payload(payload, "open_session")?;
    let resp = state.service.open_session(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn open_batch(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<OpenBatchRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenBatchResponse>, BrokerError> {
    let request = parse_json_payload(payload, "open_batch")?;
    let resp = state.service.open_batch(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn list_sessions(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<crate::models::ListSessionsResponse>, BrokerError> {
    let resp = state.service.list_sessions(&profile_id).await?;
    Ok(Json(resp))
}

async fn close_session(
    State(state): State<AppState>,
    Path((profile_id, session_id)): Path<(String, String)>,
) -> Result<StatusCode, BrokerError> {
    state
        .service
        .close_session(&profile_id, &session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::decode_refresh_request;

    #[test]
    fn decode_refresh_request_defaults_for_empty_body() {
        let request = decode_refresh_request(&[]).expect("empty body should default");
        assert!(!request.force);
    }

    #[test]
    fn decode_refresh_request_rejects_invalid_json() {
        let err =
            decode_refresh_request(br#"{"force":"oops"}"#).expect_err("invalid json should fail");
        assert_eq!(err.code(), "invalid_request");
    }
}
