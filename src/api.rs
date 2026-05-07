use std::{collections::HashSet, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    middleware,
    response::sse::{Event, KeepAlive, Sse},
    routing::{delete, get, patch, post},
};

use crate::{
    auth::{AuthConfig, AuthContext, resolve_request_auth},
    error::BrokerError,
    models::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateProjectRequest, CreateProjectResponse,
        HealthResponse, LoadSubscriptionRequest, OpenBatchByIpRequest, OpenBatchByNodeRequest,
        OpenBatchRequest, OpenSessionByIpRequest, OpenSessionByNodeRequest, OpenSessionRequest,
        ProjectProxySettings, ProxyCatalogQuery, ProxyImportListQuery, ProxyInventoryListQuery,
        ProxyOperationRequest, ProxyScope, RefreshRequest, SearchSessionIpNodeOptionsRequest,
        SearchSessionNodeOptionsRequest, SearchSessionOptionsRequest, SuggestedPortResponse,
        TaskListQuery, TaskRunDetail, TaskRunSummary, TaskStreamEnvelope,
        UpdateProjectProxySettingsRequest, UpdateProxyAllocationRequest,
        UpdateProxyImportAllocationRequest, UpdateSessionNodeRequest, UpdateSystemSettingsRequest,
    },
    service::BrokerService,
    tasks::{TaskBusEvent, build_task_list_response, matches_task_query},
    web_ui::spa_fallback,
};

const GLOBAL_TASK_PROJECT_ID: &str = "__global__";
const SESSION_DISPLAY_HOST_HEADER: &str = "x-proxy-broker-display-host";

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<BrokerService>,
    pub auth: Arc<AuthConfig>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/auth/me", get(auth_me))
        .route("/api/v1/projects", get(list_projects).post(create_project))
        .route("/api/v1/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/v1/api-keys/{key_id}", delete(revoke_api_key))
        .route(
            "/api/v1/proxies/global/subscriptions/load",
            post(load_global_subscription),
        )
        .route("/api/v1/proxy-imports", get(list_proxy_imports))
        .route("/api/v1/proxy-catalog", get(list_proxy_catalog))
        .route(
            "/api/v1/proxy-ops/refresh",
            post(refresh_proxy_catalog_metadata),
        )
        .route("/api/v1/proxy-ops/probe", post(probe_proxy_catalog_latency))
        .route(
            "/api/v1/proxy-imports/{import_id}/allocation",
            patch(update_proxy_import_allocation),
        )
        .route(
            "/api/v1/proxy-imports/{import_id}",
            delete(delete_proxy_import),
        )
        .route("/api/v1/proxies", get(list_proxy_inventory))
        .route(
            "/api/v1/proxies/{node_id}/allocation",
            patch(update_proxy_allocation),
        )
        .route(
            "/api/v1/proxies/{node_id}",
            delete(delete_proxy_inventory_node),
        )
        .route(
            "/api/v1/projects/{project_id}/proxy-settings",
            get(get_project_proxy_settings).patch(update_project_proxy_settings),
        )
        .route("/api/v1/tasks", get(list_tasks))
        .route(
            "/api/v1/system-settings",
            get(get_system_settings).patch(update_system_settings),
        )
        .route("/api/v1/tasks/events", get(stream_tasks))
        .route("/api/v1/tasks/{run_id}", get(get_task_run_detail))
        .route(
            "/api/v1/projects/{project_id}/subscriptions/load",
            post(load_subscription),
        )
        .route(
            "/api/v1/projects/{project_id}/refresh",
            post(refresh_project),
        )
        .route(
            "/api/v1/projects/{project_id}/ips/extract",
            post(extract_ips),
        )
        .route(
            "/api/v1/projects/{project_id}/ips/options/search",
            post(search_session_options),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open",
            post(open_session),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open-batch",
            post(open_batch),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open-by-node",
            post(open_session_by_node),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open-batch-by-node",
            post(open_batch_by_node),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open-by-ip",
            post(open_session_by_ip),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/open-batch-by-ip",
            post(open_batch_by_ip),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/suggested-port",
            get(suggested_port),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/{session_id}/node-options/search",
            post(search_session_node_options),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/ip-node-options/search",
            post(search_session_ip_node_options),
        )
        .route("/api/v1/projects/{project_id}/sessions", get(list_sessions))
        .route(
            "/api/v1/projects/{project_id}/sessions/{session_id}",
            delete(close_session),
        )
        .route(
            "/api/v1/projects/{project_id}/sessions/{session_id}/node",
            patch(update_session_node),
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

async fn list_projects(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<crate::models::ListProjectsResponse>, BrokerError> {
    auth.require_admin()?;
    let resp = state.service.list_projects().await?;
    Ok(Json(resp))
}

async fn create_project(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<CreateProjectRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateProjectResponse>), BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "create_project")?;
    let resp = state.service.create_project(&request.project_id).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

async fn load_global_subscription(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<LoadSubscriptionRequest>, JsonRejection>,
) -> Result<Json<crate::models::LoadSubscriptionResponse>, BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "load_global_subscription")?;
    let resp = state
        .service
        .load_global_subscription_request(&request)
        .await?;
    Ok(Json(resp))
}

async fn list_proxy_inventory(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ProxyInventoryListQuery>,
) -> Result<Json<crate::models::ListProxyInventoryResponse>, BrokerError> {
    auth.require_admin()?;
    let resp = state
        .service
        .list_proxy_inventory(query.scope.as_deref(), query.project_id.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn list_proxy_imports(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ProxyImportListQuery>,
) -> Result<Json<crate::models::ListProxyImportResponse>, BrokerError> {
    auth.require_admin()?;
    let resp = state
        .service
        .list_proxy_imports(query.scope.as_deref(), query.project_id.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn list_proxy_catalog(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ProxyCatalogQuery>,
) -> Result<Json<crate::models::ProxyCatalogResponse>, BrokerError> {
    authorize_proxy_catalog_access(&auth, &query)?;
    let resp = state.service.list_proxy_catalog(&query).await?;
    Ok(Json(resp))
}

async fn update_proxy_allocation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    payload: Result<Json<UpdateProxyAllocationRequest>, JsonRejection>,
) -> Result<Json<crate::models::ProxyInventoryItem>, BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "update_proxy_allocation")?;
    let resp = state
        .service
        .update_proxy_allocation(&node_id, &request.allocation_scope)
        .await?;
    Ok(Json(resp))
}

async fn delete_proxy_inventory_node(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<StatusCode, BrokerError> {
    auth.require_admin()?;
    state.service.delete_proxy_inventory_node(&node_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_proxy_import_allocation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(import_id): Path<String>,
    payload: Result<Json<UpdateProxyImportAllocationRequest>, JsonRejection>,
) -> Result<Json<crate::models::ProxyImportItem>, BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "update_proxy_import_allocation")?;
    let resp = state
        .service
        .update_proxy_import_allocation(&import_id, &request.allocation_scope)
        .await?;
    Ok(Json(resp))
}

async fn delete_proxy_import(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(import_id): Path<String>,
) -> Result<StatusCode, BrokerError> {
    let record = state.service.get_proxy_import(&import_id).await?;
    match (&record.source_scope, &record.allocation_scope) {
        (ProxyScope::Global, _) => auth.require_admin()?,
        (
            ProxyScope::Project {
                project_id: source_project_id,
            },
            ProxyScope::Project {
                project_id: allocation_project_id,
            },
        ) if source_project_id == allocation_project_id => {
            auth.require_project_access(source_project_id)?
        }
        _ => auth.require_admin()?,
    };
    state.service.delete_proxy_import(&import_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_project_proxy_settings(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectProxySettings>, BrokerError> {
    auth.require_admin()?;
    let resp = state
        .service
        .get_project_proxy_settings(&project_id)
        .await?;
    Ok(Json(resp))
}

async fn update_project_proxy_settings(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    payload: Result<Json<UpdateProjectProxySettingsRequest>, JsonRejection>,
) -> Result<Json<ProjectProxySettings>, BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "update_project_proxy_settings")?;
    let resp = state
        .service
        .update_project_proxy_settings(&project_id, request.use_global_proxies)
        .await?;
    Ok(Json(resp))
}

async fn get_system_settings(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<crate::models::SystemSettings>, BrokerError> {
    auth.require_admin()?;
    let resp = state.service.get_system_settings().await?;
    Ok(Json(resp))
}

async fn update_system_settings(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<UpdateSystemSettingsRequest>, JsonRejection>,
) -> Result<Json<crate::models::SystemSettings>, BrokerError> {
    auth.require_admin()?;
    let request = parse_json_payload(payload, "update_system_settings")?;
    let resp = state
        .service
        .update_system_settings(request.proxy_probe_interval_sec)
        .await?;
    Ok(Json(resp))
}

async fn list_tasks(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<crate::models::TaskListResponse>, BrokerError> {
    authorize_task_query_access(&auth, &query)?;
    let resp = state.service.list_tasks(&query).await?;
    Ok(Json(resp))
}

async fn get_task_run_detail(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<TaskRunDetail>, BrokerError> {
    let resp = state.service.get_task_run_detail(&run_id).await?;
    authorize_task_run_access(&auth, &resp.run)?;
    Ok(Json(resp))
}

async fn stream_tasks(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, BrokerError> {
    authorize_task_query_access(&auth, &query)?;

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
    Path(project_id): Path<String>,
    payload: Result<Json<LoadSubscriptionRequest>, JsonRejection>,
) -> Result<Json<crate::models::LoadSubscriptionResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "load_subscription")?;
    let resp = state
        .service
        .load_subscription_request(&project_id, &request)
        .await?;
    Ok(Json(resp))
}

async fn refresh_project(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    body: Bytes,
) -> Result<Json<crate::models::RefreshResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = decode_refresh_request(&body)?;
    let resp = state.service.refresh(&project_id, &request).await?;
    Ok(Json(resp))
}

async fn refresh_proxy_catalog_metadata(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<ProxyOperationRequest>, JsonRejection>,
) -> Result<
    (
        StatusCode,
        Json<crate::models::ProxyOperationAcceptedResponse>,
    ),
    BrokerError,
> {
    let request = parse_json_payload(payload, "refresh_proxy_catalog_metadata")?;
    authorize_proxy_operation_access(&auth, &request)?;
    let resp = state.service.queue_proxy_metadata_refresh(&request).await?;
    Ok((StatusCode::ACCEPTED, Json(resp)))
}

async fn probe_proxy_catalog_latency(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<ProxyOperationRequest>, JsonRejection>,
) -> Result<
    (
        StatusCode,
        Json<crate::models::ProxyOperationAcceptedResponse>,
    ),
    BrokerError,
> {
    let request = parse_json_payload(payload, "probe_proxy_catalog_latency")?;
    authorize_proxy_operation_access(&auth, &request)?;
    let resp = state.service.queue_proxy_latency_probe(&request).await?;
    Ok((StatusCode::ACCEPTED, Json(resp)))
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

fn authorize_proxy_catalog_access(
    auth: &AuthContext,
    query: &ProxyCatalogQuery,
) -> Result<(), BrokerError> {
    match query.view.as_deref().unwrap_or("global") {
        "global" => {
            auth.require_admin()?;
        }
        "project" => {
            let project_id = query.project_id.as_deref().ok_or_else(|| {
                BrokerError::InvalidRequest("project_id is required when view=project".to_string())
            })?;
            auth.require_project_access(project_id)?;
        }
        other => {
            return Err(BrokerError::InvalidRequest(format!(
                "unsupported proxy catalog view `{other}`"
            )));
        }
    }
    Ok(())
}

fn authorize_task_query_access(
    auth: &AuthContext,
    query: &TaskListQuery,
) -> Result<(), BrokerError> {
    match query.project_id.as_deref() {
        Some(project_id) if project_id != "all" && project_id != GLOBAL_TASK_PROJECT_ID => {
            auth.require_project_access(project_id)?;
        }
        _ => {
            auth.require_admin()?;
        }
    }
    Ok(())
}

fn authorize_task_run_access(
    auth: &AuthContext,
    run: &crate::models::TaskRunSummary,
) -> Result<(), BrokerError> {
    if run.project_id == GLOBAL_TASK_PROJECT_ID {
        auth.require_admin()?;
    } else {
        auth.require_project_access(&run.project_id)?;
    }
    Ok(())
}

fn authorize_proxy_operation_access(
    auth: &AuthContext,
    request: &ProxyOperationRequest,
) -> Result<(), BrokerError> {
    match request.view.as_str() {
        "global" => {
            auth.require_admin()?;
        }
        "project" => {
            let project_id = request.project_id.as_deref().ok_or_else(|| {
                BrokerError::InvalidRequest("project_id is required when view=project".to_string())
            })?;
            auth.require_project_access(project_id)?;
        }
        other => {
            return Err(BrokerError::InvalidRequest(format!(
                "unsupported proxy catalog view `{other}`"
            )));
        }
    }
    Ok(())
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
    Path(project_id): Path<String>,
    payload: Result<Json<crate::models::ExtractIpRequest>, JsonRejection>,
) -> Result<Json<crate::models::ExtractIpResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "extract_ips")?;
    let resp = state.service.extract_ips(&project_id, &request).await?;
    Ok(Json(resp))
}

async fn open_session(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenSessionRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_session")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_session(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn open_batch(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenBatchRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenBatchResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_batch")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_batch(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn open_session_by_node(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenSessionByNodeRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_session_by_node")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_session_by_node(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn open_batch_by_node(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenBatchByNodeRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenBatchResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_batch_by_node")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_batch_by_node(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn open_session_by_ip(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenSessionByIpRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_session_by_ip")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_session_by_ip(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn open_batch_by_ip(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<OpenBatchByIpRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenBatchResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "open_batch_by_ip")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .open_batch_by_ip(&project_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn suggested_port(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<SuggestedPortResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let resp = state.service.suggested_port(&project_id).await?;
    Ok(Json(resp))
}

async fn search_session_options(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    payload: Result<Json<SearchSessionOptionsRequest>, JsonRejection>,
) -> Result<Json<crate::models::SearchSessionOptionsResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "search_session_options")?;
    let resp = state
        .service
        .search_session_options(&project_id, &request)
        .await?;
    Ok(Json(resp))
}

async fn list_sessions(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<crate::models::ListSessionsResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .list_sessions(&project_id, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

async fn close_session(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((project_id, session_id)): Path<(String, String)>,
) -> Result<StatusCode, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    state
        .service
        .close_session(&project_id, &session_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn search_session_node_options(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((project_id, session_id)): Path<(String, String)>,
    payload: Result<Json<SearchSessionNodeOptionsRequest>, JsonRejection>,
) -> Result<Json<crate::models::SearchSessionNodeOptionsResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "search_session_node_options")?;
    let resp = state
        .service
        .search_session_node_options(&project_id, &session_id, &request)
        .await?;
    Ok(Json(resp))
}

async fn search_session_ip_node_options(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    payload: Result<Json<SearchSessionIpNodeOptionsRequest>, JsonRejection>,
) -> Result<Json<crate::models::SearchSessionIpNodeOptionsResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "search_session_ip_node_options")?;
    let resp = state
        .service
        .search_session_ip_node_options(&project_id, &request)
        .await?;
    Ok(Json(resp))
}

async fn update_session_node(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((project_id, session_id)): Path<(String, String)>,
    headers: HeaderMap,
    payload: Result<Json<UpdateSessionNodeRequest>, JsonRejection>,
) -> Result<Json<crate::models::OpenSessionResponse>, BrokerError> {
    auth.require_project_access(&project_id)?;
    state.service.require_project_exists(&project_id).await?;
    let request = parse_json_payload(payload, "update_session_node")?;
    let display_host = resolve_session_display_host_hint(&headers);
    let resp = state
        .service
        .update_session_node(&project_id, &session_id, &request, display_host.as_deref())
        .await?;
    Ok(Json(resp))
}

fn resolve_session_display_host_hint(headers: &HeaderMap) -> Option<String> {
    [SESSION_DISPLAY_HOST_HEADER, "x-forwarded-host", "host"]
        .into_iter()
        .find_map(|header_name| {
            headers
                .get(header_name)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

async fn list_api_keys(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<crate::models::ListApiKeysResponse>, BrokerError> {
    let principal = auth.require_admin()?;
    let response = state.service.list_api_keys(&principal.subject).await?;
    Ok(Json(response))
}

async fn create_api_key(
    auth: AuthContext,
    State(state): State<AppState>,
    payload: Result<Json<CreateApiKeyRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), BrokerError> {
    let principal = auth.require_admin()?;
    let request = parse_json_payload(payload, "create_api_key")?;
    let response = state
        .service
        .create_api_key(&request, &principal.subject)
        .await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn revoke_api_key(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(key_id): Path<String>,
) -> Result<StatusCode, BrokerError> {
    let principal = auth.require_admin()?;
    state
        .service
        .revoke_api_key(&principal.subject, &key_id)
        .await?;
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
    use axum::{body::Body, extract::ConnectInfo, http::Request};
    use tower::ServiceExt;

    use super::{
        AppState, build_router, decode_refresh_request, should_stream_run_event,
        should_stream_run_upsert, snapshot_visible_run_ids, upsert_stream_matching_runs,
    };
    use crate::{
        auth::{AuthConfig, AuthConfigOptions},
        models::{
            ProjectSyncConfig, ProxyNode, SessionRecord, SubscriptionSource, TaskListQuery,
            TaskRunKind, TaskRunScope, TaskRunStage, TaskRunStatus, TaskRunSummary, TaskRunTrigger,
            now_epoch_sec,
        },
        runtime::MihomoRuntime,
        service::{BrokerService, BrokerServiceOptions},
        store::{BrokerStore, MemoryStore},
    };

    struct ApiTestRuntime;

    #[async_trait]
    impl MihomoRuntime for ApiTestRuntime {
        async fn ensure_started(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn shutdown_project(&self, _project_id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn controller_meta(
            &self,
            _project_id: &str,
        ) -> anyhow::Result<(String, Option<String>)> {
            Ok(("127.0.0.1:9090".to_string(), None))
        }

        async fn controller_addr(&self, _project_id: &str) -> anyhow::Result<String> {
            Ok("127.0.0.1:9090".to_string())
        }

        async fn apply_config(&self, _project_id: &str, _payload: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn measure_proxy_delay(
            &self,
            _project_id: &str,
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
            .upsert_project_sync_config(&ProjectSyncConfig {
                import_id: "imp-M7n2Qa8Wx4Rp7Ts1".to_string(),
                project_id: "default".to_string(),
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
                run_id: "run-H6r2Lp8XmQ4Tn7Vc".to_string(),
                project_id: "default".to_string(),
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
                    .uri("/api/v1/tasks?project_id=default")
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

    #[tokio::test]
    async fn list_sessions_endpoint_uses_request_display_host_for_wildcard_binds() {
        let store = Arc::new(MemoryStore::new());
        let service = Arc::new(BrokerService::new(
            store.clone(),
            Arc::new(ApiTestRuntime),
            BrokerServiceOptions::default(),
        ));
        service
            .create_project("default")
            .await
            .expect("project should be created");
        store
            .replace_subscription(
                "default",
                &[ProxyNode {
                    proxy_name: "jp-edge".to_string(),
                    proxy_type: "socks5".to_string(),
                    server: "203.0.113.10".to_string(),
                    resolved_ips: vec!["203.0.113.10".to_string()],
                    raw_proxy: serde_json::json!({
                        "name": "jp-edge",
                        "type": "socks5",
                        "server": "203.0.113.10",
                    }),
                    node_id: Some("node-jp-edge".to_string()),
                }],
            )
            .await
            .expect("subscription seed should succeed");
        store
            .insert_session(
                "default",
                &SessionRecord {
                    session_id: "sess-123".to_string(),
                    listen: "0.0.0.0".to_string(),
                    port: 20002,
                    selected_ip: "203.0.113.10".to_string(),
                    proxy_name: "jp-edge".to_string(),
                    node_id: "node-jp-edge".to_string(),
                    candidate_node_ids: vec!["node-jp-edge".to_string()],
                    created_at: 1,
                },
            )
            .await
            .expect("session seed should succeed");
        let app = build_router(AppState {
            service,
            auth: Arc::new(dev_auth()),
        });

        let response = app
            .oneshot(trusted_request(
                Request::builder()
                    .uri("/api/v1/projects/default/sessions")
                    .header(super::SESSION_DISPLAY_HOST_HEADER, "panel.example.test")
                    .body(Body::empty())
                    .unwrap(),
            ))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should decode");
        let payload: crate::models::ListSessionsResponse =
            serde_json::from_slice(&bytes).expect("json should decode");
        assert_eq!(payload.sessions[0].listen, "0.0.0.0:20002");
        assert_eq!(payload.sessions[0].bind_host, "0.0.0.0");
        assert_eq!(payload.sessions[0].display_host, "panel.example.test");
        assert_eq!(
            payload.sessions[0].display_address,
            "panel.example.test:20002"
        );
    }

    #[test]
    fn run_upsert_streaming_updates_currently_visible_rows() {
        let run = TaskRunSummary {
            run_id: "run-J5w3Ns9Qa1Ze6Ru2".to_string(),
            project_id: "default".to_string(),
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
            run_id: "run-P4v8Kb2Yt7Lm1Cx5".to_string(),
            project_id: "other".to_string(),
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
                run_id: "run-J5w3Ns9Qa1Ze6Ru2".to_string(),
                project_id: "default".to_string(),
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
                run_id: "run-P4v8Kb2Yt7Lm1Cx5".to_string(),
                project_id: "default".to_string(),
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
        assert!(visible_run_ids.contains("run-J5w3Ns9Qa1Ze6Ru2"));
        assert!(visible_run_ids.contains("run-P4v8Kb2Yt7Lm1Cx5"));
    }

    #[test]
    fn upsert_stream_matching_runs_replaces_visible_run_without_requery() {
        let query = TaskListQuery {
            project_id: Some("default".to_string()),
            ..TaskListQuery::default()
        };
        let mut matching_runs = vec![TaskRunSummary {
            run_id: "run-J5w3Ns9Qa1Ze6Ru2".to_string(),
            project_id: "default".to_string(),
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
            run_id: "run-J5w3Ns9Qa1Ze6Ru2".to_string(),
            project_id: "default".to_string(),
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
}
