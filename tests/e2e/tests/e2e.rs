use axon_core::AxonConfig;
use axon_server::{app::AppState, build_router};
use serde_json::Value;

async fn start() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = AxonConfig::default();
    config.storage.sqlite_path = dir.path().join("test.db").to_string_lossy().into_owned();
    let state = AppState::new(config, dir.path().to_path_buf()).unwrap();
    std::mem::forget(dir);
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    (addr, handle)
}

fn url(addr: std::net::SocketAddr, path: &str) -> String {
    format!("http://{addr}{path}")
}

#[tokio::test]
async fn healthz_returns_ok() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/healthz")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn readyz_returns_ready() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/readyz")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn status_returns_running() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/status")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "running");
    assert!(body["uptime_secs"].as_u64().is_some());
}

#[tokio::test]
async fn metrics_returns_prometheus_text() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/metrics")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert!(text.contains("axon_requests_total") || text.contains("axon metrics"));
}

#[tokio::test]
async fn list_agents_returns_array() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/v1/agents")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn list_tools_returns_array() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/v1/tools")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn usage_returns_stats() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/v1/usage")).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn list_models_returns_object() {
    let (addr, _h) = start().await;
    let resp = reqwest::get(url(addr, "/v1/models")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_object());
}

#[tokio::test]
async fn conversation_create_and_list() {
    let (addr, _h) = start().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(url(addr, "/v1/conversations"))
        .json(&serde_json::json!({"agent_id": "test-agent", "title": "test conv"}))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let text = resp.text().await.unwrap();
    assert_eq!(status, 200, "POST /v1/conversations body: {text}");
    let body: Value = serde_json::from_str(&text).expect("invalid json");
    assert!(body.get("id").is_some(), "missing id in {body}");
    let resp = client
        .get(url(addr, "/v1/conversations"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.is_array());
    assert!(!body.as_array().unwrap().is_empty());
}
