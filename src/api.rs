use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
};

use crate::{
    auth::{AuthConfig, AuthContext, resolve_request_auth},
    error::BrokerError,
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProfileRequest, CreateProfileResponse,
        HealthResponse, LoadSubscriptionRequest, OpenBatchRequest, OpenSessionRequest,
        RefreshRequest,
    },
    service::BrokerService,
    web_ui::spa_fallback,
};

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<BrokerService>,
    pub auth: Arc<AuthConfig>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/auth/me", get(auth_me))
        .route("/api/v1/profiles", get(list_profiles).post(create_profile))
        .route(
            "/api/v1/profiles/{profile_id}/subscriptions/load",
            post(load_subscription),
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
            "/api/v1/profiles/{profile_id}/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route(
            "/api/v1/profiles/{profile_id}/api-keys/{key_id}",
            delete(revoke_api_key),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/{session_id}",
            delete(close_session),
        )
        .fallback(spa_fallback)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            resolve_request_auth,
        ))
        .with_state(state)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn auth_me(auth: AuthContext) -> Result<Json<crate::models::AuthMeResponse>, BrokerError> {
    let principal = auth.require_authenticated()?;
    Ok(Json(principal.as_auth_me()))
}

async fn list_profiles(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<crate::models::ListProfilesResponse>, BrokerError> {
    auth.require_admin()?;
    let resp = state.service.list_profiles().await?;
    Ok(Json(resp))
}

async fn create_profile(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<CreateProfileRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateProfileResponse>), BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "create_profile")?;
    let resp = state.service.create_profile(&request.profile_id).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

async fn load_subscription(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<LoadSubscriptionRequest>, JsonRejection>,
) -> Result<Json<crate::models::LoadSubscriptionResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "load_subscription")?;
    let resp = state
        .service
        .load_subscription(&profile_id, &request.source)
        .await?;
    Ok(Json(resp))
}

async fn refresh_profile(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    body: Bytes,
) -> Result<Json<crate::models::RefreshResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
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
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<crate::models::ExtractIpRequest>, JsonRejection>,
) -> Result<Json<crate::models::ExtractIpResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "extract_ips")?;
    let resp = state.service.extract_ips(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn open_session(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<OpenSessionRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "open_session")?;
    let resp = state.service.open_session(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn open_batch(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<OpenBatchRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenBatchResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "open_batch")?;
    let resp = state.service.open_batch(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn list_sessions(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<crate::models::ListSessionsResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let resp = state.service.list_sessions(&profile_id).await?;
    Ok(Json(resp))
}

async fn close_session(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((profile_id, session_id)): Path<(String, String)>,
) -> Result<StatusCode, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    state
        .service
        .close_session(&profile_id, &session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_api_keys(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<crate::models::ListApiKeysResponse>, BrokerError> {
    auth.require_admin()?;
    let response = state.service.list_api_keys(&profile_id).await?;
    Ok(Json(response))
}

async fn create_api_key(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<CreateApiKeyRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), BrokerError> {
    let principal = auth.require_admin()?;
    let request = parse_json_payload(payload, "create_api_key")?;
    let response = state
        .service
        .create_api_key(&profile_id, &request, &principal.subject)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn revoke_api_key(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((profile_id, key_id)): Path<(String, String)>,
) -> Result<StatusCode, BrokerError> {
    auth.require_admin()?;
    state.service.revoke_api_key(&profile_id, &key_id).await?;
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
