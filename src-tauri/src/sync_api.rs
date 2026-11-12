use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lierkour_core::sync::{SyncPayload, create_thumbnail};
use lierkour_core::{AppPaths, Database};

#[derive(Clone)]
pub struct SyncServerState {
    pub db: Arc<Mutex<Database>>,
    pub paths: AppPaths,
}

pub fn start_sync_server(db: Arc<Mutex<Database>>, paths: AppPaths) {
    let state = SyncServerState { db, paths };

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime for Sync Server");

        rt.block_on(async move {
            let app = Router::new()
                .route("/api/v1/sync/exchange", post(exchange_handler))
                .route("/api/v1/sync/media/{kind}/{file_name}", get(media_handler))
                .route("/api/v1/sync/media/{kind}/{file_name}/thumb", get(thumbnail_handler))
                .with_state(state)
                .layer(middleware::from_fn(cors_middleware));

            let addr: SocketAddr = "0.0.0.0:8766".parse().unwrap();
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(err) => {
                    eprintln!("[Lierkour Sync] Failed to bind 0.0.0.0:8766: {err}");
                    return;
                }
            };
            println!("[Lierkour Sync] Listening for mobile devices on http://0.0.0.0:8766");
            let _ = axum::serve(listener, app).await;
        });
    });
}

async fn cors_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    if req.method() == Method::OPTIONS {
        return StatusCode::NO_CONTENT.into_response();
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, OPTIONS"));
    headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type"));
    response
}

async fn exchange_handler(
    State(state): State<SyncServerState>,
    Json(client_payload): Json<SyncPayload>,
) -> Result<Json<SyncPayload>, StatusCode> {
    let mut db = state.db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 1. Применяем входящие изменения от клиента (мобильного приложения)
    let _summary = db.apply_sync_delta(&client_payload).map_err(|e| {
        eprintln!("[Lierkour Sync] Apply delta error: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 2. Извлекаем изменения на ПК с момента since клиента
    let server_payload = db.get_sync_delta(client_payload.since.as_deref(), "desktop").map_err(|e| {
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
    if !thumb_file.exists() {
        if let Err(e) = create_thumbnail(&original, &thumb_file, 400) {
            eprintln!("[Sync] Thumbnail generation error: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    match tokio::fs::read(&thumb_file).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/jpeg")], bytes).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
