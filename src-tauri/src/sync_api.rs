use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lierkour_core::sync::{SyncPayload, create_thumbnail};
use lierkour_core::{AppPaths, Database};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SYNC_PORT: u16 = 8767;
pub const SYNC_SERVER_ENABLED_SETTING: &str = "sync_server_enabled";
pub const SYNC_SERVER_PORT_SETTING: &str = "sync_server_port";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncServerStatus {
    pub active: bool,
    pub port: u16,
    pub local_ips: Vec<String>,
    pub recommended_url: String,
    pub all_urls: Vec<String>,
    pub error: Option<String>,
}

impl Default for SyncServerStatus {
    fn default() -> Self {
        let local_ips = get_local_ips();
        let port = DEFAULT_SYNC_PORT;
        let recommended_url = local_ips
            .first()
            .map(|ip| format!("http://{ip}:{port}"))
            .unwrap_or_else(|| format!("http://127.0.0.1:{port}"));
        let all_urls = local_ips
            .iter()
            .map(|ip| format!("http://{ip}:{port}"))
            .collect();

        Self {
            active: false,
            port,
            local_ips,
            recommended_url,
            all_urls,
            error: None,
        }
    }
}

#[derive(Default)]
pub struct SyncServerHandle {
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub status: SyncServerStatus,
}

/// Определение локальных IPv4 адресов ПК для подключения мобильных устройств.
pub fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();

    // 1. Метод UDP-маршрутизации (быстро определяет первичный сетевой интерфейс с выходом в LAN)
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
        && !addr.ip().is_loopback()
    {
        ips.push(addr.ip().to_string());
    }

    // 2. Разрешение локального имени хоста через системный DNS/NetBIOS
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    if !hostname.is_empty()
        && let Ok(addrs) = format!("{hostname}:0").to_socket_addrs()
    {
        for addr in addrs {
            if let std::net::IpAddr::V4(ipv4) = addr.ip() {
                let s = ipv4.to_string();
                if !ipv4.is_loopback() && !ips.contains(&s) {
                    ips.push(s);
                }
            }
        }
    }

    if ips.is_empty() {
        ips.push("127.0.0.1".to_string());
    }

    ips
}

#[derive(Clone)]
pub struct SyncServerState {
    pub db: Arc<Mutex<Database>>,
    pub paths: AppPaths,
}

pub fn stop_sync_server(handle_arc: &Arc<Mutex<SyncServerHandle>>) -> SyncServerStatus {
    let mut handle = match handle_arc.lock() {
        Ok(h) => h,
        Err(e) => e.into_inner(),
    };

    if let Some(tx) = handle.shutdown_tx.take() {
        let _ = tx.send(());
    }

    handle.status.active = false;
    handle.status.error = None;
    handle.status.clone()
}

pub fn start_sync_server(
    handle_arc: &Arc<Mutex<SyncServerHandle>>,
    port: u16,
    db: Arc<Mutex<Database>>,
    paths: AppPaths,
) -> SyncServerStatus {
    // Останавливаем предыдущий сервер, если работал
    stop_sync_server(handle_arc);

    let state = SyncServerState { db, paths };
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let local_ips = get_local_ips();
    let recommended_url = local_ips
        .first()
        .map(|ip| format!("http://{ip}:{port}"))
        .unwrap_or_else(|| format!("http://127.0.0.1:{port}"));
    let all_urls = local_ips
        .iter()
        .map(|ip| format!("http://{ip}:{port}"))
        .collect();

    let std_listener = match std::net::TcpListener::bind(format!("0.0.0.0:{port}")) {
        Ok(l) => l,
        Err(err) => {
            let mut handle = match handle_arc.lock() {
                Ok(h) => h,
                Err(e) => e.into_inner(),
            };
            handle.status.active = false;
            handle.status.port = port;
            handle.status.local_ips = local_ips;
            handle.status.recommended_url = recommended_url;
            handle.status.all_urls = all_urls;
            handle.status.error = Some(err.to_string());
            return handle.status.clone();
        }
    };

    if let Err(err) = std_listener.set_nonblocking(true) {
        let mut handle = match handle_arc.lock() {
            Ok(h) => h,
            Err(e) => e.into_inner(),
        };
        handle.status.active = false;
        handle.status.port = port;
        handle.status.local_ips = local_ips;
        handle.status.recommended_url = recommended_url;
        handle.status.all_urls = all_urls;
        handle.status.error = Some(err.to_string());
        return handle.status.clone();
    }

    {
        let mut handle = match handle_arc.lock() {
            Ok(h) => h,
            Err(e) => e.into_inner(),
        };
        handle.shutdown_tx = Some(shutdown_tx);
        handle.status.active = true;
        handle.status.port = port;
        handle.status.local_ips = local_ips;
        handle.status.recommended_url = recommended_url;
        handle.status.all_urls = all_urls;
        handle.status.error = None;
    }

    let status_to_return = {
        let handle = match handle_arc.lock() {
            Ok(h) => h,
            Err(e) => e.into_inner(),
        };
        handle.status.clone()
    };

    let handle_for_task = Arc::clone(handle_arc);

    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(err) => {
                if let Ok(mut h) = handle_for_task.lock() {
                    h.status.active = false;
                    h.status.error = Some(err.to_string());
                }
                eprintln!("[Lierkour Sync] Failed to adopt tokio listener: {err}");
                return;
            }
        };

        let app = Router::new()
            .route("/api/v1/sync/ping", get(sync_ping_handler))
            .route("/api/lier/info", get(sync_ping_handler))
            .route("/api/v1/sync/exchange", post(exchange_handler))
            .route("/api/v1/sync/media/{kind}/{file_name}", get(media_handler))
            .route(
                "/api/v1/sync/media/{kind}/{file_name}/thumb",
                get(thumbnail_handler),
            )
            .with_state(state)
            .layer(middleware::from_fn(cors_middleware));

        println!("[Lierkour Sync] Listening for mobile devices on http://0.0.0.0:{port}");

        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

        if let Err(err) = server.await {
            if let Ok(mut h) = handle_for_task.lock() {
                h.status.active = false;
                h.status.error = Some(err.to_string());
            }
            eprintln!("[Lierkour Sync] Sync server error: {err}");
        } else if let Ok(mut h) = handle_for_task.lock() {
            h.status.active = false;
        }
    });

    status_to_return
}

async fn cors_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    if req.method() == Method::OPTIONS {
        return StatusCode::NO_CONTENT.into_response();
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
    response
}

async fn sync_ping_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "ecosystem": "LIER",
        "app": "Lierkour Sync",
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "port": DEFAULT_SYNC_PORT,
        "platform": std::env::consts::OS,
        "license": "GPL-3.0-only"
    }))
}

async fn exchange_handler(
    State(state): State<SyncServerState>,
    Json(client_payload): Json<SyncPayload>,
) -> Result<Json<SyncPayload>, StatusCode> {
    let mut db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _summary = db.apply_sync_delta(&client_payload).map_err(|e| {
        eprintln!("[Lierkour Sync] Apply delta error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let server_payload = db
        .get_sync_delta(client_payload.since.as_deref(), "desktop")
        .map_err(|e| {
            eprintln!("[Lierkour Sync] Get delta error: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(server_payload))
}

async fn media_handler(
    State(state): State<SyncServerState>,
    AxumPath((kind, file_name)): AxumPath<(String, String)>,
) -> Response {
    let base_dir = match kind.as_str() {
        "posters" => &state.paths.posters_dir,
        "screenshots" => &state.paths.screenshots_dir,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let file_path = base_dir.join(&file_name);
    if !file_path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/jpeg")], bytes).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn thumbnail_handler(
    State(state): State<SyncServerState>,
    AxumPath((kind, file_name)): AxumPath<(String, String)>,
) -> Response {
    let base_dir = match kind.as_str() {
        "posters" => &state.paths.posters_dir,
        "screenshots" => &state.paths.screenshots_dir,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let original = base_dir.join(&file_name);
    if !original.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let thumb_dir = state.paths.data_dir.join("thumbnails").join(&kind);
    let thumb_file = thumb_dir.join(&file_name);
    if !thumb_file.exists()
        && let Err(e) = create_thumbnail(&original, &thumb_file, 400)
    {
        eprintln!("[Sync] Thumbnail generation error: {e:?}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    match tokio::fs::read(&thumb_file).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/jpeg")], bytes).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
