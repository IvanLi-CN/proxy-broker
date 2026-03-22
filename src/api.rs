use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    middleware,
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, post},
};

use crate::{
    auth::{AuthConfig, AuthContext, resolve_request_auth},
    error::BrokerError,
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProfileRequest, CreateProfileResponse,
        HealthResponse, LoadSubscriptionRequest, OpenBatchRequest, OpenSessionRequest,
        RefreshRequest, TaskListQuery, TaskRunDetail, TaskStreamEnvelope,
    },
    service::BrokerService,
    tasks::TaskBusEvent,
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
        .route("/api/v1/tasks", get(list_tasks))
        .route("/api/v1/tasks/events", get(stream_tasks))
        .route("/api/v1/tasks/{run_id}", get(get_task_run_detail))
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

async fn list_tasks(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<crate::models::TaskListResponse>, BrokerError> {
    auth.require_admin()?;
    let resp = state.service.list_tasks(&query).await?;
    Ok(Json(resp))
}

async fn get_task_run_detail(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<TaskRunDetail>, BrokerError> {
    auth.require_admin()?;
    let resp = state.service.get_task_run_detail(&run_id).await?;
    Ok(Json(resp))
}

async fn stream_tasks(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, BrokerError> {
    auth.require_admin()?;

    let snapshot = state.service.list_tasks(&query).await?;
    let mut receiver = state.service.subscribe_task_events();
    let service = state.service.clone();
    let stream_query = query.clone();

    let stream = async_stream::stream! {
        yield Ok(sse_event("snapshot", serde_json::to_value(snapshot.clone())));

        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    yield Ok(sse_event("heartbeat", Ok(serde_json::json!({
                        "at": crate::models::now_epoch_sec(),
                    }))));
                }
                message = receiver.recv() => {
                    match message {
                        Ok(TaskBusEvent::RunUpsert(run)) => {
                            if let Some(profile_id) = &stream_query.profile_id
                                && profile_id != &run.profile_id
                            {
                                continue;
                            }

                            yield Ok(sse_event("run-upsert", serde_json::to_value(run.clone())));
                            match service.list_tasks(&stream_query).await {
                                Ok(response) => {
                                    yield Ok(sse_event("summary", serde_json::to_value(response.summary)));
                                }
                                Err(err) => {
                                    tracing::warn!(error = %err, "task sse failed to refresh summary");
                                    break;
                                }
                            }
                        }
                        Ok(TaskBusEvent::RunEvent(event)) => {
                            if let Some(profile_id) = &stream_query.profile_id
                                && profile_id != &event.profile_id
                            {
                                continue;
                            }
                            yield Ok(sse_event("run-event", serde_json::to_value(event.as_public())));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            match service.list_tasks(&stream_query).await {
                                Ok(response) => {
                                    yield Ok(sse_event("snapshot", serde_json::to_value(response.clone())));
                                    yield Ok(sse_event("summary", serde_json::to_value(response.summary)));
                                }
                                Err(err) => {
                                    tracing::warn!(error = %err, "task sse failed to rebuild snapshot after lag");
                                    break;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
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

fn sse_event(event_type: &str, data: Result<serde_json::Value, serde_json::Error>) -> Event {
    let envelope = match data {
        Ok(data) => TaskStreamEnvelope {
            event_type: event_type.to_string(),
            data,
        },
        Err(err) => TaskStreamEnvelope {
            event_type: event_type.to_string(),
            data: serde_json::json!({
                "code": "serialization_error",
                "message": err.to_string(),
            }),
        },
    };
    Event::default()
        .event(event_type)
        .data(serde_json::to_string(&envelope).unwrap_or_else(|_| {
            "{\"type\":\"internal_error\",\"data\":{\"message\":\"failed to encode sse\"}}"
                .to_string()
        }))
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
    use std::{
        net::{Ipv4Addr, SocketAddr},
        sync::Arc,
    };

    use async_trait::async_trait;
    use axum::{body::Body, extract::ConnectInfo, http::Request};
    use tower::ServiceExt;

    use super::{AppState, build_router, decode_refresh_request};
    use crate::{
        auth::{AuthConfig, AuthConfigOptions},
        models::{
            ProfileSyncConfig, SubscriptionSource, TaskRunKind, TaskRunScope, TaskRunStage,
            TaskRunStatus, TaskRunTrigger, now_epoch_sec,
        },
        runtime::MihomoRuntime,
        service::{BrokerService, BrokerServiceOptions},
        store::{BrokerStore, MemoryStore},
    };

    struct ApiTestRuntime;

    #[async_trait]
    impl MihomoRuntime for ApiTestRuntime {
        async fn ensure_started(&self, _profile_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_profile(&self, _profile_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _profile_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, _profile_id: &str) -> anyhow::Result<String> {
            Ok("127.0.0.1:9090".to_string())
        }

        async fn apply_config(&self, _profile_id: &str, _payload: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _profile_id: &str,
            _proxy_name: &str,
            _url: &str,
            _timeout_ms: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(Some(1))
        }
    }

    fn dev_auth() -> AuthConfig {
        AuthConfig::from_options(AuthConfigOptions {
            mode: "development".to_string(),
            subject_headers: "".to_string(),
            email_headers: "".to_string(),
            groups_headers: "".to_string(),
            trusted_proxies: "".to_string(),
            admin_users: "".to_string(),
            admin_groups: "".to_string(),
            dev_user: "dev-admin".to_string(),
            dev_email: "dev@example.com".to_string(),
            dev_groups: "proxy-broker-admins".to_string(),
        })
        .expect("development auth config should build")
    }

    fn trusted_request(mut request: Request<Body>) -> Request<Body> {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((Ipv4Addr::LOCALHOST, 40123))));
        request
    }

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

    #[tokio::test]
    async fn list_tasks_endpoint_returns_seeded_runs() {
        let store = Arc::new(MemoryStore::new());
        let now = now_epoch_sec();
        store
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: "default".to_string(),
                source: SubscriptionSource::Url("https://example.com/sub".to_string()),
                enabled: true,
                sync_every_sec: 600,
                full_refresh_every_sec: 86_400,
                last_sync_due_at: Some(now + 600),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(now + 86_400),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now,
            })
            .await
            .expect("sync config seed should succeed");
        store
            .insert_task_run(&crate::models::TaskRunRecord {
                run_id: "run_1".to_string(),
                profile_id: "default".to_string(),
                kind: TaskRunKind::SubscriptionSync,
                trigger: TaskRunTrigger::Schedule,
                status: TaskRunStatus::Queued,
                stage: TaskRunStage::Queued,
                progress_current: Some(0),
                progress_total: Some(1),
                created_at: now,
                started_at: None,
                finished_at: None,
                summary_json: None,
                error_code: None,
                error_message: None,
                scope: TaskRunScope::All,
            })
            .await
            .expect("task run seed should succeed");

        let service = Arc::new(BrokerService::new(
            store,
            Arc::new(ApiTestRuntime),
            BrokerServiceOptions::default(),
        ));
        let app = build_router(AppState {
            service,
            auth: Arc::new(dev_auth()),
        });

        let response = app
            .oneshot(
                trusted_request(
                    Request::builder()
                        .uri("/api/v1/tasks?profile_id=default")
                        .body(Body::empty())
                        .unwrap(),
                ),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn task_events_endpoint_streams_sse() {
        let service = Arc::new(BrokerService::new(
            Arc::new(MemoryStore::new()),
            Arc::new(ApiTestRuntime),
            BrokerServiceOptions::default(),
        ));
        let app = build_router(AppState {
            service,
            auth: Arc::new(dev_auth()),
        });

        let response = app
            .oneshot(
                trusted_request(
                    Request::builder()
                        .uri("/api/v1/tasks/events")
                        .body(Body::empty())
                        .unwrap(),
                ),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
    }
}
