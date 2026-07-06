use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::builder::{build_config, to_yaml};
use crate::error::{ProxyError, Result};
use crate::fetch::{deduplicate_names, fetch_subscription};
use crate::i18n::t;
use crate::parser::parse_lines;
use crate::types::RuleMode;

#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub url: String,
    pub port: u16,
    pub interval_min: u64,
    pub rule_mode: RuleMode,
}

struct CacheEntry {
    yaml: String,
    node_count: usize,
    updated_at: u64,
}

struct AppState {
    opts: ServerOptions,
    cache: Option<CacheEntry>,
    refresh_error: Option<String>,
    refreshing: bool,
}

pub struct ServerHandle {
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
    refresh_handle: JoinHandle<()>,
}

impl ServerHandle {
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        self.refresh_handle.abort();
        let _ = self.refresh_handle.await;
        let _ = self.join_handle.await;
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    nodes: usize,
    updated_at: Option<String>,
    next_refresh_min: u64,
    last_error: Option<String>,
}

#[derive(Serialize)]
struct RefreshResponse {
    ok: bool,
    skipped: bool,
    nodes: usize,
}

#[derive(Serialize)]
struct RefreshErrorResponse {
    ok: bool,
    error: &'static str,
}

pub async fn start_server(opts: ServerOptions) -> Result<ServerHandle> {
    let state = Arc::new(Mutex::new(AppState {
        opts: opts.clone(),
        cache: None,
        refresh_error: None,
        refreshing: false,
    }));

    println!("🔗 {}", t("refreshing", &[]));
    refresh_state(&state).await?;
    println!(
        "📡 {}\n",
        t("http_start", &[("interval", &opts.interval_min.to_string())])
    );

    let refresh_state_clone = Arc::clone(&state);
    let interval = opts.interval_min;
    let refresh_handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval * 60));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let _ = refresh_state(&refresh_state_clone).await;
        }
    });

    let app_state = Arc::clone(&state);
    let app = Router::new()
        .route("/clash.yaml", get(clash_yaml))
        .route("/health", get(health))
        .route(
            "/refresh",
            post(refresh).get(refresh_method_not_allowed),
        )
        .layer(middleware::from_fn(add_cors))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], opts.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                ProxyError::msg(t("port_in_use", &[("port", &opts.port.to_string())]))
            } else {
                ProxyError::Io(err)
            }
        })?;

    print_banner(opts.port);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let join_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
    });

    Ok(ServerHandle {
        shutdown_tx,
        join_handle,
        refresh_handle,
    })
}

async fn refresh_state(state: &Arc<Mutex<AppState>>) -> Result<()> {
    let mut guard = state.lock().await;
    if guard.refreshing {
        println!("  ⏭️  {}", t("refresh_skip", &[]));
        return Ok(());
    }
    guard.refreshing = true;
    let opts = guard.opts.clone();
    drop(guard);

    let result = async {
        println!(
            "[{}] 🔄 {}",
            chrono_like_time(),
            t("refreshing", &[])
        );
        let lines = fetch_subscription(&opts.url).await?;
        let mut nodes = parse_lines(&lines);
        deduplicate_names(&mut nodes);
        let config = build_config(&nodes, opts.rule_mode);
        let yaml = to_yaml(&config)?;
        let count = nodes.len();
        Ok::<_, ProxyError>((yaml, count))
    }
    .await;

    let mut guard = state.lock().await;
    guard.refreshing = false;

    match result {
        Ok((yaml, count)) => {
            guard.cache = Some(CacheEntry {
                yaml,
                node_count: count,
                updated_at: now_ms(),
            });
            guard.refresh_error = None;
            println!(
                "  ✅ {}",
                t("refresh_ok", &[("count", &count.to_string())])
            );
            Ok(())
        }
        Err(err) => {
            let msg = err.to_string();
            guard.refresh_error = Some(msg.clone());
            if guard.cache.is_some() {
                println!("  ⚠️  {}", t("refresh_fail", &[("msg", &msg)]));
                Ok(())
            } else {
                eprintln!("  ❌ {}", t("first_fetch_fail", &[("msg", &msg)]));
                Err(err)
            }
        }
    }
}

fn chrono_like_time() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn iso_timestamp(ms: u64) -> String {
    let secs = ms / 1000;
    let datetime = time_from_unix(secs);
    format!("{datetime}")
}

fn time_from_unix(secs: u64) -> String {
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;

    let mut year = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let year_days = if leap { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        year += 1;
    }

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &md in &month_days {
        if remaining_days < md as i64 {
            break;
        }
        remaining_days -= md as i64;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.000Z")
}

async fn add_cors(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    response
}

async fn refresh_method_not_allowed() -> impl IntoResponse {
    (StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
}

async fn clash_yaml(State(state): State<Arc<Mutex<AppState>>>) -> Response {
    let guard = state.lock().await;
    match &guard.cache {
        Some(cache) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/yaml; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            cache.yaml.clone(),
        )
            .into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "Service unavailable").into_response(),
    }
}

async fn health(State(state): State<Arc<Mutex<AppState>>>) -> Response {
    let guard = state.lock().await;
    let now = now_ms();
    let next_refresh = match &guard.cache {
        Some(cache) => {
            let elapsed_min = (now.saturating_sub(cache.updated_at)) / 60_000;
            guard.opts.interval_min.saturating_sub(elapsed_min)
        }
        None => guard.opts.interval_min,
    };

    let body = HealthResponse {
        status: if guard.cache.is_some() {
            "ok"
        } else {
            "initializing"
        },
        nodes: guard.cache.as_ref().map(|c| c.node_count).unwrap_or(0),
        updated_at: guard
            .cache
            .as_ref()
            .map(|c| iso_timestamp(c.updated_at)),
        next_refresh_min: next_refresh,
        last_error: guard.refresh_error.clone(),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        Json(body),
    )
        .into_response()
}

async fn refresh(State(state): State<Arc<Mutex<AppState>>>) -> Response {
    let skipped = {
        let guard = state.lock().await;
        guard.refreshing
    };

    if skipped {
        let guard = state.lock().await;
        let nodes = guard.cache.as_ref().map(|c| c.node_count).unwrap_or(0);
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            Json(RefreshResponse {
                ok: true,
                skipped: true,
                nodes,
            }),
        )
            .into_response();
    }

    match refresh_state(&state).await {
        Ok(()) => {
            let guard = state.lock().await;
            let nodes = guard.cache.as_ref().map(|c| c.node_count).unwrap_or(0);
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
                Json(RefreshResponse {
                    ok: true,
                    skipped: false,
                    nodes,
                }),
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            Json(RefreshErrorResponse {
                ok: false,
                error: "Refresh failed",
            }),
        )
            .into_response(),
    }
}

fn print_banner(port: u16) {
    let base = format!("http://127.0.0.1:{port}");
    let c = BANNER_COLORS;
    let title = format!(
        "{}{}R{}I{}N{}O{}V{}A{}{}  {}{}",
        c.b, c.g1, c.g2, c.g3, c.c1, c.c2, c.c3, c.rst, c.d, t("server_title", &[]), c.rst
    );
    println!();
    println!("{}╔══════════════════════════════════════════════════╗{}", c.n2, c.rst);
    println!("{}║{}  {}{}║{}", c.n2, c.rst, title, c.n2, c.rst);
    println!("{}║{}                                              {}║{}", c.n2, c.rst, c.n2, c.rst);
    println!(
        "{}║{}  {}▸ {}{}{}/clash.yaml{}          {}║{}",
        c.n2, c.rst, c.y, c.b, c.w, base, c.rst, c.n2, c.rst
    );
    println!(
        "{}║{}  {}▸ {}{}{}{}{}║{}",
        c.n2,
        c.rst,
        c.y,
        c.d,
        c.dim,
        t("server_banner_clash", &[]),
        "                            ",
        c.n2,
        c.rst
    );
    println!("{}║{}                                              {}║{}", c.n2, c.rst, c.n2, c.rst);
    println!(
        "{}║{}  {}▸ {}{}{}/health{}              {}║{}",
        c.n2, c.rst, c.y, c.b, c.w, base, c.rst, c.n2, c.rst
    );
    println!(
        "{}║{}  {}▸ {}{}{}{}{}║{}",
        c.n2,
        c.rst,
        c.y,
        c.d,
        c.dim,
        t("server_banner_health", &[]),
        "                            ",
        c.n2,
        c.rst
    );
    println!("{}╚══════════════════════════════════════════════════╝{}\n", c.n2, c.rst);
}

struct BannerColors {
    rst: &'static str,
    b: &'static str,
    d: &'static str,
    g1: &'static str,
    g2: &'static str,
    g3: &'static str,
    c1: &'static str,
    c2: &'static str,
    c3: &'static str,
    n2: &'static str,
    y: &'static str,
    w: &'static str,
    dim: &'static str,
}

impl BannerColors {
    const fn new() -> Self {
        Self {
            rst: "\x1b[0m",
            b: "\x1b[1m",
            d: "\x1b[2m",
            g1: "\x1b[38;5;82m",
            g2: "\x1b[38;5;83m",
            g3: "\x1b[38;5;84m",
            c1: "\x1b[38;5;87m",
            c2: "\x1b[38;5;117m",
            c3: "\x1b[38;5;153m",
            n2: "\x1b[38;5;45m",
            y: "\x1b[38;5;228m",
            w: "\x1b[38;5;255m",
            dim: "\x1b[38;5;245m",
        }
    }
}

const BANNER_COLORS: BannerColors = BannerColors::new();

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::sync::atomic::{AtomicU16, Ordering};

    static TEST_PORT: AtomicU16 = AtomicU16::new(35500);

    fn next_port_pair() -> (u16, u16) {
        let base = TEST_PORT.fetch_add(2, Ordering::SeqCst);
        (base, base + 1)
    }

    async fn start_mock_subscription(port: u16) -> tokio::task::JoinHandle<()> {
        let proxy_data = base64::engine::general_purpose::STANDARD.encode(
            "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@us1.example.com:8388#美国-01\n\
             trojan://pass@sg1.example.com:443?security=tls&sni=sg1.example.com#新加坡-01",
        );

        tokio::spawn(async move {
            let app = Router::new().route(
                "/sub",
                get(|| async move { proxy_data.clone() }),
            );
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.ok();
        })
    }

    #[tokio::test]
    async fn server_endpoints() {
        let (sub_port, convert_port) = next_port_pair();
        let _mock = start_mock_subscription(sub_port).await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let handle = start_server(ServerOptions {
            url: format!("http://127.0.0.1:{sub_port}/sub"),
            port: convert_port,
            interval_min: 999,
            rule_mode: RuleMode::Builtin,
        })
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        let health = reqwest::get(format!("http://127.0.0.1:{convert_port}/health"))
            .await
            .unwrap();
        assert_eq!(health.status(), 200);
        assert_eq!(
            health.headers().get("access-control-allow-origin").unwrap(),
            "*"
        );
        let body: serde_json::Value = health.json().await.unwrap();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["nodes"], 2);

        let yaml = reqwest::get(format!("http://127.0.0.1:{convert_port}/clash.yaml"))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(yaml.contains("proxies:"));
        assert!(yaml.contains("美国-01"));

        let refresh = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{convert_port}/refresh"))
            .send()
            .await
            .unwrap();
        assert_eq!(refresh.status(), 200);
        let body: serde_json::Value = refresh.json().await.unwrap();
        assert_eq!(body["ok"], true);

        let get_refresh = reqwest::get(format!("http://127.0.0.1:{convert_port}/refresh"))
            .await
            .unwrap();
        assert_eq!(get_refresh.status(), 405);

        let unknown = reqwest::get(format!("http://127.0.0.1:{convert_port}/unknown"))
            .await
            .unwrap();
        assert_eq!(unknown.status(), 404);

        handle.shutdown().await;
    }
}
