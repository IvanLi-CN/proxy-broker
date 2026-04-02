use std::{collections::HashSet, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post},
};

use crate::{
    auth::{AuthConfig, AuthContext, resolve_request_auth},
    error::BrokerError,
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProfileRequest, CreateProfileResponse,
        HealthResponse, LoadSubscriptionRequest, NodeExportFormat, NodeExportRequest,
        NodeListQuery, NodeOpenSessionsRequest, OpenBatchRequest, OpenSessionRequest,
        RefreshRequest, SearchSessionOptionsRequest, SuggestedPortResponse, TaskListQuery,
        TaskRunDetail, TaskRunSummary, TaskStreamEnvelope,
    },
    service::BrokerService,
    tasks::{TaskBusEvent, build_task_list_response, matches_task_query},
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
            "/api/v1/profiles/{profile_id}/ips/options/search",
            post(search_session_options),
        )
        .route(
            "/api/v1/profiles/{profile_id}/nodes/query",
            post(query_nodes),
        )
        .route(
            "/api/v1/profiles/{profile_id}/nodes/export",
            post(export_nodes),
        )
        .route(
            "/api/v1/profiles/{profile_id}/nodes/open-sessions",
            post(open_node_sessions),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/open",
            post(open_session),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/open-batch",
            post(open_batch),
        )
        .route(
            "/api/v1/profiles/{profile_id}/sessions/suggested-port",
            get(suggested_port),
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

    let mut receiver = state.service.subscribe_task_events();
    let stream_query = query.clone();
    let service = state.service.clone();
    let mut matching_runs = service.list_task_run_summaries(&query).await?;
    let snapshot = build_task_list_response(&query, matching_runs.clone());
    let initial_visible_run_ids = snapshot_visible_run_ids(&snapshot.runs);

    let stream = async_stream::stream! {
        yield Ok(sse_event("snapshot", serde_json::to_value(snapshot.clone())));
        let mut visible_run_ids = initial_visible_run_ids;
        let mut summary = snapshot.summary.clone();
        let mut next_cursor = snapshot.next_cursor.clone();

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
                            upsert_stream_matching_runs(&mut matching_runs, &stream_query, &run);
                            let response = build_task_list_response(&stream_query, matching_runs.clone());
                            let next_visible_run_ids = snapshot_visible_run_ids(&response.runs);
                            let snapshot_changed =
                                next_visible_run_ids != visible_run_ids || response.next_cursor != next_cursor;
                            let emit_run_upsert = should_stream_run_upsert(&next_visible_run_ids, &run);
                            let emit_summary = response.summary != summary;

                            visible_run_ids = next_visible_run_ids;
                            next_cursor = response.next_cursor.clone();

                            if snapshot_changed {
                                summary = response.summary.clone();
                                yield Ok(sse_event("snapshot", serde_json::to_value(response)));
                            } else {
                                if emit_summary {
                                    summary = response.summary.clone();
                                    yield Ok(sse_event("summary", serde_json::to_value(summary.clone())));
                                }
                                if emit_run_upsert {
                                    yield Ok(sse_event("run-upsert", serde_json::to_value(run.clone())));
                                }
                            }
                        }
                        Ok(TaskBusEvent::RunEvent(event)) => {
                            if !should_stream_run_event(&visible_run_ids, &event.run_id) {
                                continue;
                            }
                            yield Ok(sse_event("run-event", serde_json::to_value(event.as_public())));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            match service.list_task_run_summaries(&stream_query).await {
                                Ok(response) => {
                                    matching_runs = response;
                                    let snapshot = build_task_list_response(&stream_query, matching_runs.clone());
                                    visible_run_ids = snapshot_visible_run_ids(&snapshot.runs);
                                    summary = snapshot.summary.clone();
                                    next_cursor = snapshot.next_cursor.clone();
                                    yield Ok(sse_event("snapshot", serde_json::to_value(snapshot)));
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

fn should_stream_run_upsert(visible_run_ids: &HashSet<String>, run: &TaskRunSummary) -> bool {
    visible_run_ids.contains(&run.run_id)
}

fn should_stream_run_event(visible_run_ids: &HashSet<String>, run_id: &str) -> bool {
    visible_run_ids.contains(run_id)
}

fn snapshot_visible_run_ids(runs: &[TaskRunSummary]) -> HashSet<String> {
    runs.iter().map(|run| run.run_id.clone()).collect()
}

fn upsert_stream_matching_runs(
    matching_runs: &mut Vec<TaskRunSummary>,
    query: &TaskListQuery,
    run: &TaskRunSummary,
) {
    matching_runs.retain(|item| item.run_id != run.run_id);
    if matches_task_query(run, query) {
        matching_runs.push(run.clone());
    }
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

async fn suggested_port(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Json<SuggestedPortResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let resp = state.service.suggested_port(&profile_id).await?;
    Ok(Json(resp))
}

async fn search_session_options(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<SearchSessionOptionsRequest>, JsonRejection>,
) -> Result<Json<crate::models::SearchSessionOptionsResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "search_session_options")?;
    let resp = state
        .service
        .search_session_options(&profile_id, &request)
        .await?;
    Ok(Json(resp))
}

async fn query_nodes(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<NodeListQuery>, JsonRejection>,
) -> Result<Json<crate::models::NodeListResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "query_nodes")?;
    let resp = state.service.query_nodes(&profile_id, &request).await?;
    Ok(Json(resp))
}

async fn export_nodes(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<NodeExportRequest>, JsonRejection>,
) -> Result<impl IntoResponse, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "export_nodes")?;
    let body = state.service.export_nodes(&profile_id, &request).await?;
    let (content_type, filename) = match request.format {
        NodeExportFormat::Csv => ("text/csv; charset=utf-8", "proxy-broker-nodes.csv"),
        NodeExportFormat::LinkLines => ("text/plain; charset=utf-8", "proxy-broker-node-links.txt"),
    };
    let content_disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|err| BrokerError::Internal(err.to_string()))?;
    Ok((
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        body,
    ))
}

async fn open_node_sessions(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    payload: Result<Json<NodeOpenSessionsRequest>, JsonRejection>,
) -> Result<Json<crate::models::NodeOpenSessionsResponse>, BrokerError> {
    auth.require_profile_access(&profile_id)?;
    let request = parse_json_payload(payload, "open_node_sessions")?;
    let resp = state
        .service
        .open_node_sessions(&profile_id, &request)
        .await?;
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
        collections::HashSet,
        net::{Ipv4Addr, SocketAddr},
        sync::Arc,
    };

    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Method, Request},
    };
    use tower::ServiceExt;

    use super::{
        AppState, build_router, decode_refresh_request, should_stream_run_event,
        should_stream_run_upsert, snapshot_visible_run_ids, upsert_stream_matching_runs,
    };
    use crate::{
        auth::{AuthConfig, AuthConfigOptions},
        models::{
            ProfileSyncConfig, SubscriptionSource, TaskListQuery, TaskRunKind, TaskRunScope,
            TaskRunStage, TaskRunStatus, TaskRunSummary, TaskRunTrigger, now_epoch_sec,
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

    fn enforce_auth() -> AuthConfig {
        AuthConfig::from_options(AuthConfigOptions {
            mode: "enforce".to_string(),
            subject_headers: "x-auth-user".to_string(),
            email_headers: "x-auth-email".to_string(),
            groups_headers: "x-auth-groups".to_string(),
            trusted_proxies: "127.0.0.1/32".to_string(),
            admin_users: "admin".to_string(),
            admin_groups: "proxy-broker-admins".to_string(),
            dev_user: "dev-admin".to_string(),
            dev_email: "dev@example.com".to_string(),
            dev_groups: "proxy-broker-admins".to_string(),
        })
        .expect("enforce auth config should build")
    }

    fn trusted_request(mut request: Request<Body>) -> Request<Body> {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((Ipv4Addr::LOCALHOST, 40123))));
        request
    }

    async fn seed_nodes_profile(store: &MemoryStore, profile_id: &str) {
        store
            .replace_subscription(
                profile_id,
                &[crate::models::ProxyNode {
                    proxy_name: "edge-a".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "edge.example".to_string(),
                    resolved_ips: vec!["2001:db8::5".to_string(), "5.5.5.5".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "edge-a",
                        "type": "socks5",
                        "server": "edge.example",
                        "port": 1080
                    }),
                }],
            )
            .await
            .expect("seed subscription should succeed");
        store
            .replace_ip_records(
                profile_id,
                &[
                    crate::models::IpRecord {
                        ip: "5.5.5.5".to_string(),
                        country_code: Some("US".to_string()),
                        country_name: Some("United States".to_string()),
                        region_name: Some("California".to_string()),
                        city: Some("San Jose".to_string()),
                        geo_source: Some("test".to_string()),
                        probe_updated_at: None,
                        geo_updated_at: None,
                        last_used_at: Some(10),
                    },
                    crate::models::IpRecord {
                        ip: "2001:db8::5".to_string(),
                        country_code: Some("DE".to_string()),
                        country_name: Some("Germany".to_string()),
                        region_name: Some("Berlin".to_string()),
                        city: Some("Berlin".to_string()),
                        geo_source: Some("test".to_string()),
                        probe_updated_at: None,
                        geo_updated_at: None,
                        last_used_at: Some(8),
                    },
                ],
            )
            .await
            .expect("seed ip records should succeed");
        store
            .replace_probe_records(
                profile_id,
                &[crate::models::ProbeRecord {
                    proxy_name: "edge-a".to_string(),
                    ip: "5.5.5.5".to_string(),
                    target_url: "https://www.gstatic.com/generate_204".to_string(),
                    ok: true,
                    latency_ms: Some(12),
                    updated_at: 1,
                }],
            )
            .await
            .expect("seed probe records should succeed");
        store
            .insert_session(
                profile_id,
                &crate::models::SessionRecord {
                    session_id: "seed-session".to_string(),
                    listen: "127.0.0.1".to_string(),
                    port: 18080,
                    selected_ip: "5.5.5.5".to_string(),
                    proxy_name: "edge-a".to_string(),
                    created_at: 1,
                },
            )
            .await
            .expect("seed session should succeed");
        store
            .upsert_profile_sync_config(&ProfileSyncConfig {
                profile_id: profile_id.to_string(),
                source: SubscriptionSource::Url("https://example.com/subscription".to_string()),
                enabled: true,
                sync_every_sec: 600,
                full_refresh_every_sec: 86_400,
                last_sync_due_at: Some(now_epoch_sec() + 600),
                last_sync_started_at: None,
                last_sync_finished_at: None,
                last_full_refresh_due_at: Some(now_epoch_sec() + 86_400),
                last_full_refresh_started_at: None,
                last_full_refresh_finished_at: None,
                updated_at: now_epoch_sec(),
            })
            .await
            .expect("sync config seed should succeed");
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
            .oneshot(trusted_request(
                Request::builder()
                    .uri("/api/v1/tasks?profile_id=default")
                    .body(Body::empty())
                    .unwrap(),
            ))
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
            .oneshot(trusted_request(
                Request::builder()
                    .uri("/api/v1/tasks/events")
                    .body(Body::empty())
                    .unwrap(),
            ))
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

    #[test]
    fn run_upsert_streaming_updates_currently_visible_rows() {
        let run = TaskRunSummary {
            run_id: "run-1".to_string(),
            profile_id: "default".to_string(),
            kind: TaskRunKind::SubscriptionSync,
            trigger: TaskRunTrigger::Schedule,
            status: TaskRunStatus::Succeeded,
            stage: TaskRunStage::Completed,
            progress_current: Some(1),
            progress_total: Some(1),
            created_at: 1,
            started_at: Some(1),
            finished_at: Some(2),
            summary_json: None,
            error_code: None,
            error_message: None,
        };

        let visible_run_ids = HashSet::from([run.run_id.clone()]);

        assert!(should_stream_run_upsert(&visible_run_ids, &run));
        assert!(should_stream_run_event(&visible_run_ids, &run.run_id));
    }

    #[test]
    fn run_upsert_streaming_keeps_off_scope_runs_out_of_filtered_feed() {
        let run = TaskRunSummary {
            run_id: "run-2".to_string(),
            profile_id: "other".to_string(),
            kind: TaskRunKind::SubscriptionSync,
            trigger: TaskRunTrigger::Schedule,
            status: TaskRunStatus::Succeeded,
            stage: TaskRunStage::Completed,
            progress_current: Some(1),
            progress_total: Some(1),
            created_at: 1,
            started_at: Some(1),
            finished_at: Some(2),
            summary_json: None,
            error_code: None,
            error_message: None,
        };

        let visible_run_ids = HashSet::new();

        assert!(!should_stream_run_upsert(&visible_run_ids, &run));
        assert!(!should_stream_run_event(&visible_run_ids, &run.run_id));
    }

    #[test]
    fn snapshot_visible_run_ids_tracks_rebuilt_snapshot_rows() {
        let runs = vec![
            TaskRunSummary {
                run_id: "run-1".to_string(),
                profile_id: "default".to_string(),
                kind: TaskRunKind::SubscriptionSync,
                trigger: TaskRunTrigger::Schedule,
                status: TaskRunStatus::Running,
                stage: TaskRunStage::Probing,
                progress_current: Some(1),
                progress_total: Some(2),
                created_at: 1,
                started_at: Some(1),
                finished_at: None,
                summary_json: None,
                error_code: None,
                error_message: None,
            },
            TaskRunSummary {
                run_id: "run-2".to_string(),
                profile_id: "default".to_string(),
                kind: TaskRunKind::MetadataRefreshFull,
                trigger: TaskRunTrigger::Schedule,
                status: TaskRunStatus::Queued,
                stage: TaskRunStage::Queued,
                progress_current: Some(0),
                progress_total: None,
                created_at: 2,
                started_at: None,
                finished_at: None,
                summary_json: None,
                error_code: None,
                error_message: None,
            },
        ];

        let visible_run_ids = snapshot_visible_run_ids(&runs);

        assert_eq!(visible_run_ids.len(), 2);
        assert!(visible_run_ids.contains("run-1"));
        assert!(visible_run_ids.contains("run-2"));
    }

    #[test]
    fn upsert_stream_matching_runs_replaces_visible_run_without_requery() {
        let query = TaskListQuery {
            profile_id: Some("default".to_string()),
            ..TaskListQuery::default()
        };
        let mut matching_runs = vec![TaskRunSummary {
            run_id: "run-1".to_string(),
            profile_id: "default".to_string(),
            kind: TaskRunKind::SubscriptionSync,
            trigger: TaskRunTrigger::Schedule,
            status: TaskRunStatus::Running,
            stage: TaskRunStage::Probing,
            progress_current: Some(1),
            progress_total: Some(2),
            created_at: 1,
            started_at: Some(1),
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
        }];

        let updated_run = TaskRunSummary {
            status: TaskRunStatus::Succeeded,
            stage: TaskRunStage::Completed,
            finished_at: Some(2),
            ..matching_runs[0].clone()
        };
        upsert_stream_matching_runs(&mut matching_runs, &query, &updated_run);

        assert_eq!(matching_runs.len(), 1);
        assert_eq!(matching_runs[0].status, TaskRunStatus::Succeeded);
    }

    #[test]
    fn upsert_stream_matching_runs_drops_runs_that_leave_the_filter() {
        let query = TaskListQuery {
            running_only: true,
            ..TaskListQuery::default()
        };
        let mut matching_runs = vec![TaskRunSummary {
            run_id: "run-1".to_string(),
            profile_id: "default".to_string(),
            kind: TaskRunKind::SubscriptionSync,
            trigger: TaskRunTrigger::Schedule,
            status: TaskRunStatus::Running,
            stage: TaskRunStage::Probing,
            progress_current: Some(1),
            progress_total: Some(2),
            created_at: 1,
            started_at: Some(1),
            finished_at: None,
            summary_json: None,
            error_code: None,
            error_message: None,
        }];

        let updated_run = TaskRunSummary {
            status: TaskRunStatus::Succeeded,
            stage: TaskRunStage::Completed,
            finished_at: Some(2),
            ..matching_runs[0].clone()
        };
        upsert_stream_matching_runs(&mut matching_runs, &query, &updated_run);

        assert!(matching_runs.is_empty());
    }

    #[tokio::test]
    async fn query_nodes_endpoint_returns_node_rows() {
        let profile_id = "default";
        let store = Arc::new(MemoryStore::new());
        seed_nodes_profile(&store, profile_id).await;

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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/profiles/{profile_id}/nodes/query"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"page":1,"page_size":25}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["total"], 1);
        assert_eq!(payload["items"][0]["node_id"], "edge-a");
        assert_eq!(payload["items"][0]["preferred_ip"], "5.5.5.5");
        assert_eq!(payload["items"][0]["session_count"], 1);
    }

    #[tokio::test]
    async fn export_nodes_endpoint_returns_csv_attachment() {
        let profile_id = "default";
        let store = Arc::new(MemoryStore::new());
        seed_nodes_profile(&store, profile_id).await;

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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/profiles/{profile_id}/nodes/export"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"node_ids":["edge-a"]}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/csv; charset=utf-8")
        );
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some("attachment; filename=\"proxy-broker-nodes.csv\"")
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let csv = String::from_utf8(body.to_vec()).expect("csv body should be utf-8");
        assert!(csv.starts_with("node_id,proxy_name,proxy_type"));
        assert!(csv.contains("edge-a"));
    }

    #[tokio::test]
    async fn export_nodes_endpoint_returns_link_lines_attachment() {
        let profile_id = "default";
        let store = Arc::new(MemoryStore::new());
        seed_nodes_profile(&store, profile_id).await;

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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/profiles/{profile_id}/nodes/export"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"node_ids":["edge-a"],"format":"link_lines"}"#,
                    ))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some("attachment; filename=\"proxy-broker-node-links.txt\"")
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let links = String::from_utf8(body.to_vec()).expect("body should be utf-8");
        assert_eq!(links, "socks5://edge.example:1080#edge-a");
    }

    #[tokio::test]
    async fn open_node_sessions_endpoint_returns_partial_failures() {
        let profile_id = "default";
        let store = Arc::new(MemoryStore::new());
        seed_nodes_profile(&store, profile_id).await;

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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/v1/profiles/{profile_id}/nodes/open-sessions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"node_ids":["edge-a","missing"],"ip_family_priority":"ipv4_first"}"#,
                    ))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["sessions"].as_array().map(Vec::len), Some(1));
        assert_eq!(payload["sessions"][0]["selected_ip"], "5.5.5.5");
        assert_eq!(payload["failures"].as_array().map(Vec::len), Some(1));
        assert_eq!(payload["failures"][0]["node_id"], "missing");
        assert_eq!(payload["failures"][0]["code"], "ip_not_found");
    }

    #[tokio::test]
    async fn query_nodes_endpoint_requires_auth_when_enforced() {
        let service = Arc::new(BrokerService::new(
            Arc::new(MemoryStore::new()),
            Arc::new(ApiTestRuntime),
            BrokerServiceOptions::default(),
        ));
        let app = build_router(AppState {
            service,
            auth: Arc::new(enforce_auth()),
        });

        let response = app
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/profiles/default/nodes/query")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"page":1,"page_size":25}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["code"], "authentication_required");
    }

    #[tokio::test]
    async fn query_nodes_endpoint_returns_profile_not_found() {
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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/profiles/missing/nodes/query")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"page":1,"page_size":25}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["code"], "profile_not_found");
    }

    #[tokio::test]
    async fn export_nodes_endpoint_returns_profile_not_found() {
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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/profiles/missing/nodes/export")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"node_ids":["edge-a"]}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["code"], "profile_not_found");
    }

    #[tokio::test]
    async fn open_node_sessions_endpoint_returns_profile_not_found() {
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
            .oneshot(trusted_request(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/profiles/missing/nodes/open-sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"node_ids":["edge-a"]}"#))
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should be json");
        assert_eq!(payload["code"], "profile_not_found");
    }
}
