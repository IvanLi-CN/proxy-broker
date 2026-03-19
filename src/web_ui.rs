use mime_guess::mime;

use axum::{
    body::Body,
    http::{HeaderValue, Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_web_assets.rs"));
}

pub async fn spa_fallback(method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::NOT_FOUND.into_response();
    }

    let path = sanitize_path(uri.path());
    if is_reserved_path(&path) {
        return StatusCode::NOT_FOUND.into_response();
    }

    if let Some(response) = exact_asset_response(&path) {
        return response;
    }

    if should_fallback_to_index(&path) {
        return index_response();
    }

    StatusCode::NOT_FOUND.into_response()
}

fn sanitize_path(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

fn is_reserved_path(path: &str) -> bool {
    path == "api" || path.starts_with("api/") || path == "healthz"
}

fn should_fallback_to_index(path: &str) -> bool {
    path.is_empty() || !path.rsplit('/').next().unwrap_or_default().contains('.')
}

fn index_response() -> Response {
    asset_response("index.html", true)
        .unwrap_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "web ui not built").into_response())
}

fn exact_asset_response(path: &str) -> Option<Response> {
    let key = if path.is_empty() { "index.html" } else { path };
    asset_response(key, key == "index.html")
}

fn asset_response(path: &str, is_index: bool) -> Option<Response> {
    let body = embedded::get(path)?;
    let content_type = if path.ends_with(".html") {
        mime::TEXT_HTML_UTF_8
    } else {
        mime_guess::from_path(path).first_or_octet_stream()
    };
    let cache_control = if is_index {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };

    let mut response = Response::new(Body::from(body.to_vec()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref()).ok()?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::spa_fallback;
    use crate::{
        AppState, BrokerService, BrokerServiceOptions, MemoryStore, MihomoRuntime, build_router,
    };
    use anyhow::anyhow;
    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode, Uri},
    };
    use tower::ServiceExt;

    use std::sync::Arc;

    #[derive(Default)]
    struct TestRuntime;

    #[async_trait]
    impl MihomoRuntime for TestRuntime {
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

        async fn controller_addr(&self, profile_id: &str) -> anyhow::Result<String> {
            let (addr, _) = self.controller_meta(profile_id).await?;
            Ok(addr)
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
            Err(anyhow!("not implemented in router tests"))
        }
    }

    fn test_router() -> axum::Router {
        let store = Arc::new(MemoryStore::new());
        let runtime: Arc<dyn MihomoRuntime> = Arc::new(TestRuntime);
        let service = Arc::new(BrokerService::new(
            store,
            runtime,
            BrokerServiceOptions::default(),
        ));
        build_router(AppState { service })
    }

    #[tokio::test]
    async fn root_serves_embedded_index() {
        let response = test_router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn nested_frontend_route_falls_back_to_index() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/sessions/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");
    }

    #[tokio::test]
    async fn missing_asset_does_not_fall_back_to_index() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/assets/missing.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_api_path_stays_not_found() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn healthz_route_is_not_shadowed_by_spa_fallback() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }

    #[tokio::test]
    async fn profile_routes_create_and_list_profiles() {
        let app = test_router();

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/profiles")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"profile_id":"  edge-jp  "}"#))
                    .unwrap(),
            )
            .await
            .expect("router should respond");
        assert_eq!(created.status(), StatusCode::CREATED);
        let created_body = to_bytes(created.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let created_json: serde_json::Value =
            serde_json::from_slice(&created_body).expect("body should be json");
        assert_eq!(created_json["profile_id"], "edge-jp");

        let listed = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/profiles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should respond");
        assert_eq!(listed.status(), StatusCode::OK);
        let listed_body = to_bytes(listed.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let listed_json: serde_json::Value =
            serde_json::from_slice(&listed_body).expect("body should be json");
        assert_eq!(listed_json["profiles"], serde_json::json!(["edge-jp"]));
    }

    #[tokio::test]
    async fn non_get_requests_do_not_receive_spa_fallback() {
        let response = spa_fallback(Method::POST, Uri::from_static("/sessions")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
