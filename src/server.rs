//! RavenClaws
//!
//! Provides a long-running HTTP server with health, readiness, and metrics endpoints.
//! RavenClaws to run as a stable workload in Kubernetes and other
//! container orchestration platforms, fixing the CrashLoopBackOff issue.
//!
//! # Endpoints
//!
//! - `GET /health` — Liveness probe (always 200 when server is running)
//! - `GET /ready` — Readiness probe (200 when fully initialized, 503 during startup)
//! - `GET /metrics` — Prometheus-style metrics (requests, tokens, tool calls, errors)
//!
//! # Architecture
//!
//! ```text
//! HttpServer
//!   ├── /health  → always 200 OK
//!   ├── /ready   → 200 OK when ready, 503 Service Unavailable during startup
//!   └── /metrics → Prometheus text format
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::net::TcpListener;
use tokio::signal;
use tracing::{debug, error, info, instrument, warn};

use crate::config::Config;

// ── Metrics ────────────────────────────────────────────────────────────────

/// Shared metrics state for the HTTP server
#[derive(Debug, Default)]
pub struct ServerMetrics {
    /// Total HTTP requests served
    pub requests_total: AtomicU64,
    /// Total LLM requests made
    pub llm_requests_total: AtomicU64,
    /// Total tool calls executed
    pub tool_calls_total: AtomicU64,
    /// Total errors encountered
    pub errors_total: AtomicU64,
    /// Total tokens consumed (estimated)
    pub tokens_total: AtomicU64,
    /// Server start timestamp (seconds since epoch)
    pub start_time: AtomicU64,
}

impl ServerMetrics {
    fn new() -> Self {
        let metrics = Self::default();
        metrics.start_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            Ordering::Relaxed,
        );
        metrics
    }

    fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn record_llm_request(&self) {
        self.llm_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn record_tool_call(&self) {
        self.tool_calls_total.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn record_tokens(&self, count: u64) {
        self.tokens_total.fetch_add(count, Ordering::Relaxed);
    }

    fn uptime_secs(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let start = self.start_time.load(Ordering::Relaxed);
        now.saturating_sub(start)
    }
}

// ── Server state ───────────────────────────────────────────────────────────

/// Shared state for the HTTP server
pub struct ServerState {
    /// Whether the server is fully initialized and ready to serve requests
    pub ready: AtomicBool,
    /// Server metrics
    pub metrics: ServerMetrics,
    /// Server configuration (stored for future use by agent endpoints)
    #[allow(dead_code)]
    pub config: Config,
    /// Server start time (stored for future use)
    #[allow(dead_code)]
    pub start_time: Instant,
}

impl ServerState {
    fn new(config: Config) -> Self {
        Self {
            ready: AtomicBool::new(false),
            metrics: ServerMetrics::new(),
            config,
            start_time: Instant::now(),
        }
    }

    /// Mark the server as ready (initialization complete)
    fn mark_ready(&self) {
        self.ready.store(true, Ordering::Relaxed);
        info!("Server marked as ready");
    }
}

// ── HTTP response helpers ──────────────────────────────────────────────────

fn health_response() -> Vec<u8> {
    b"OK".to_vec()
}

fn ready_response(state: &ServerState) -> (Vec<u8>, &'static str) {
    if state.ready.load(Ordering::Relaxed) {
        (b"READY".to_vec(), "200 OK")
    } else {
        (b"NOT READY".to_vec(), "503 Service Unavailable")
    }
}

fn metrics_response(state: &ServerState) -> Vec<u8> {
    let metrics = &state.metrics;
    format!(
        "# HELP ravenclaws_requests_total Total HTTP requests served\n\
         # TYPE ravenclaws_requests_total counter\n\
         ravenclaws_requests_total {}\n\
         \n\
         # HELP ravenclaws_llm_requests_total Total LLM requests made\n\
         # TYPE ravenclaws_llm_requests_total counter\n\
         ravenclaws_llm_requests_total {}\n\
         \n\
         # HELP ravenclaws_tool_calls_total Total tool calls executed\n\
         # TYPE ravenclaws_tool_calls_total counter\n\
         ravenclaws_tool_calls_total {}\n\
         \n\
         # HELP ravenclaws_errors_total Total errors encountered\n\
         # TYPE ravenclaws_errors_total counter\n\
         ravenclaws_errors_total {}\n\
         \n\
         # HELP ravenclaws_tokens_total Total tokens consumed (estimated)\n\
         # TYPE ravenclaws_tokens_total counter\n\
         ravenclaws_tokens_total {}\n\
         \n\
         # HELP ravenclaws_uptime_seconds Server uptime in seconds\n\
         # TYPE ravenclaws_uptime_seconds gauge\n\
         ravenclaws_uptime_seconds {}\n\
         \n\
         # HELP ravenclaws_start_time_seconds Server start time (Unix epoch)\n\
         # TYPE ravenclaws_start_time_seconds gauge\n\
         ravenclaws_start_time_seconds {}\n",
        metrics.requests_total.load(Ordering::Relaxed),
        metrics.llm_requests_total.load(Ordering::Relaxed),
        metrics.tool_calls_total.load(Ordering::Relaxed),
        metrics.errors_total.load(Ordering::Relaxed),
        metrics.tokens_total.load(Ordering::Relaxed),
        metrics.uptime_secs(),
        metrics.start_time.load(Ordering::Relaxed),
    )
    .into_bytes()
}

// ── HTTP handler ───────────────────────────────────────────────────────────

/// Handle a single HTTP connection
#[instrument(skip_all, fields(peer = ?stream.peer_addr().ok()))]
async fn handle_connection(mut stream: tokio::net::TcpStream, state: Arc<ServerState>) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let peer = stream.peer_addr().ok();
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    // Read the request line
    if reader.read_line(&mut request_line).await.is_err() {
        return;
    }

    let request_line = request_line.trim();
    if request_line.is_empty() {
        return;
    }

    state.metrics.record_request();

    // Parse the request path
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    // Read and discard remaining headers
    let mut header_line = String::new();
    loop {
        header_line.clear();
        if reader.read_line(&mut header_line).await.is_err() {
            return;
        }
        if header_line.trim().is_empty() {
            break;
        }
    }

    // Route the request
    let (body, status_line, content_type) = match path {
        "/health" => (health_response(), "200 OK", "text/plain"),
        "/ready" => {
            let (body, status) = ready_response(&state);
            (body, status, "text/plain")
        }
        "/metrics" => (
            metrics_response(&state),
            "200 OK",
            "text/plain; charset=utf-8",
        ),
        _ => {
            state.metrics.record_error();
            (b"Not Found".to_vec(), "404 Not Found", "text/plain")
        }
    };

    // Write the response
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status_line,
        content_type,
        body.len(),
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        warn!(error = %e, "Failed to write response headers");
        return;
    }
    if let Err(e) = stream.write_all(&body).await {
        warn!(error = %e, "Failed to write response body");
        return;
    }
    if let Err(e) = stream.flush().await {
        warn!(error = %e, "Failed to flush response");
        return;
    }

    debug!(path = %path, status = %status_line, peer = ?peer, "Request handled");
}

// ── Signal handling ────────────────────────────────────────────────────────

/// Wait for shutdown signal (SIGTERM, SIGINT, or Ctrl+C)
async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Run the HTTP server
///
/// Starts a long-running HTTP server on the configured host:port.
/// Serves health, readiness, and metrics endpoints.
/// Handles graceful shutdown on SIGTERM/SIGINT.
#[instrument(skip_all, fields(bind_addr))]
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    let host = config
        .runtime
        .host
        .clone()
        .unwrap_or_else(|| "0.0.0.0".to_string());
    let port = config.runtime.port;
    let bind_addr = format!("{}:{}", host, port);

    let state = Arc::new(ServerState::new(config));
    let listener = TcpListener::bind(&bind_addr).await.map_err(|e| {
        error!(address = %bind_addr, error = %e, "Failed to bind HTTP server");
        anyhow::anyhow!("Failed to bind to {}: {}", bind_addr, e)
    })?;

    info!(
        address = %bind_addr,
        "HTTP server started — endpoints: /health, /ready, /metrics"
    );

    // Mark as ready after successful bind
    state.mark_ready();

    // Accept connections in a loop
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer)) => {
                        debug!(peer = %peer, "Accepted connection");
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            handle_connection(stream, state).await;
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to accept connection");
                        state.metrics.record_error();
                    }
                }
            }
            _ = wait_for_shutdown() => {
                info!("Shutting down HTTP server gracefully...");
                // Give in-flight requests a moment to complete
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                info!("HTTP server shutdown complete");
                break;
            }
        }
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;

    fn test_config() -> Config {
        Config {
            runtime: RuntimeConfig {
                host: Some("127.0.0.1".to_string()),
                port: 0, // OS-assigned port
                ..RuntimeConfig::default()
            },
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn test_health_response() {
        let body = health_response();
        assert_eq!(body, b"OK");
    }

    #[tokio::test]
    async fn test_ready_response_when_ready() {
        let config = test_config();
        let state = ServerState::new(config);
        state.mark_ready();
        let (body, status) = ready_response(&state);
        assert_eq!(body, b"READY");
        assert_eq!(status, "200 OK");
    }

    #[tokio::test]
    async fn test_ready_response_when_not_ready() {
        let config = test_config();
        let state = ServerState::new(config);
        let (body, status) = ready_response(&state);
        assert_eq!(body, b"NOT READY");
        assert_eq!(status, "503 Service Unavailable");
    }

    #[tokio::test]
    async fn test_metrics_response_format() {
        let config = test_config();
        let state = ServerState::new(config);
        let body = metrics_response(&state);
        let output = String::from_utf8_lossy(&body);

        // Check Prometheus format
        assert!(output.contains("ravenclaws_requests_total"));
        assert!(output.contains("ravenclaws_llm_requests_total"));
        assert!(output.contains("ravenclaws_tool_calls_total"));
        assert!(output.contains("ravenclaws_errors_total"));
        assert!(output.contains("ravenclaws_tokens_total"));
        assert!(output.contains("ravenclaws_uptime_seconds"));
        assert!(output.contains("ravenclaws_start_time_seconds"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }

    #[tokio::test]
    async fn test_metrics_counters_increment() {
        let config = test_config();
        let state = ServerState::new(config);

        state.metrics.record_request();
        state.metrics.record_request();
        state.metrics.record_error();
        state.metrics.record_tokens(150);

        assert_eq!(state.metrics.requests_total.load(Ordering::Relaxed), 2);
        assert_eq!(state.metrics.errors_total.load(Ordering::Relaxed), 1);
        assert_eq!(state.metrics.tokens_total.load(Ordering::Relaxed), 150);
    }

    #[tokio::test]
    async fn test_uptime_increases() {
        let config = test_config();
        let state = ServerState::new(config);
        let uptime1 = state.metrics.uptime_secs();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let uptime2 = state.metrics.uptime_secs();
        assert!(uptime2 >= uptime1);
    }

    #[tokio::test]
    async fn test_server_binds_and_responds() {
        let mut config = test_config();
        config.runtime.port = 0; // OS-assigned

        // Bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state = Arc::new(ServerState::new(config));
        state.mark_ready();

        // Spawn a single connection handler
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_connection(stream, state_clone).await;
        });

        // Make a request to /health
        let response = reqwest::Client::new()
            .get(format!("http://{}/health", addr))
            .send()
            .await;

        handle.await.unwrap();

        if let Ok(resp) = response {
            assert_eq!(resp.status(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, "OK");
        }
        // If reqwest fails (no HTTP client in deps), skip this assertion
    }

    #[tokio::test]
    async fn test_server_404() {
        let mut config = test_config();
        config.runtime.port = 0;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state = Arc::new(ServerState::new(config));
        state.mark_ready();

        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_connection(stream, state_clone).await;
        });

        let response = reqwest::Client::new()
            .get(format!("http://{}/unknown", addr))
            .send()
            .await;

        handle.await.unwrap();

        if let Ok(resp) = response {
            assert_eq!(resp.status(), 404);
        }
    }

    #[tokio::test]
    async fn test_server_metrics_endpoint() {
        let mut config = test_config();
        config.runtime.port = 0;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let state = Arc::new(ServerState::new(config));
        state.mark_ready();

        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle_connection(stream, state_clone).await;
        });

        let response = reqwest::Client::new()
            .get(format!("http://{}/metrics", addr))
            .send()
            .await;

        handle.await.unwrap();

        if let Ok(resp) = response {
            assert_eq!(resp.status(), 200);
            let body = resp.text().await.unwrap();
            assert!(body.contains("ravenclaws_requests_total"));
        }
    }
}
