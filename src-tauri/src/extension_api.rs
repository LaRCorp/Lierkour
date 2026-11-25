use std::sync::{Arc, Mutex, RwLock};

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lierkour_core::models::{
    Film, FilmStatus, MAX_RUNTIME_MINUTES, MAX_SEASONS, MAX_YEAR, MIN_YEAR, MediaType, NewFilm,
};
use lierkour_core::{AppPaths, Database, metadata, storage};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const EXTENSION_API_PORT: u16 = 8765;
pub const EXTENSION_API_ADDR: &str = "127.0.0.1:8765";
pub const EXTENSION_TOKEN_SETTING: &str = "lierex_token";
pub const EXTENSION_TOKEN_HEADER: &str = "x-lierex-token";
pub const EXTENSION_IMPORT_EVENT: &str = "extension-import-draft";
pub const EXTENSION_SERVER_ENABLED_SETTING: &str = "extension_server_enabled";
pub const EXTENSION_SERVER_PORT_SETTING: &str = "extension_server_port";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionServerStatus {
    pub active: bool,
    pub address: String,
    #[serde(default = "default_ext_port")]
    pub port: u16,
    pub error: Option<String>,
}

fn default_ext_port() -> u16 {
    EXTENSION_API_PORT
}

impl Default for ExtensionServerStatus {
    fn default() -> Self {
        Self {
            active: false,
            address: EXTENSION_API_ADDR.to_owned(),
            port: EXTENSION_API_PORT,
            error: None,
        }
    }
}

#[derive(Default)]
pub struct ExtensionServerHandle {
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub status: ExtensionServerStatus,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExtensionImportPayload {
    pub title: String,
    #[serde(default)]
    pub original_title: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default, rename = "type")]
    pub media_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub director: String,
    #[serde(default)]
    pub genres: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub runtime_minutes: Option<u16>,
    #[serde(default)]
    pub seasons_total: Option<u16>,
    #[serde(default)]
    pub imdb_id: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub poster_url: Option<String>,
    #[serde(default)]
    pub poster_candidates: Vec<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub launcher: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionImportResponse {
    pub ok: bool,
    pub film: Film,
    pub poster_imported: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub app: String,
    pub version: String,
    pub authorized: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone)]
pub enum ImportError {
    BadRequest(String),
    Internal(String),
}

pub type ImportCallback =
    Arc<dyn Fn(ExtensionImportPayload) -> Result<(Film, bool), ImportError> + Send + Sync>;

#[derive(Clone)]
pub struct ExtensionApiState {
    pub token: Arc<RwLock<String>>,
    pub on_import: ImportCallback,
    pub db: Arc<Mutex<Database>>,
    pub paths: AppPaths,
}

#[derive(Debug, Deserialize)]
struct EcosystemSearchQuery {
    #[serde(default)]
    q: String,
}

pub fn generate_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn load_or_create_token(db: &mut Database) -> Result<String, String> {
    if let Some(existing) = db
        .get_setting(EXTENSION_TOKEN_SETTING)
        .map_err(|e| e.to_string())?
    {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }

    let token = generate_token();
    db.set_setting(EXTENSION_TOKEN_SETTING, &token)
        .map_err(|e| e.to_string())?;
    Ok(token)
}

fn tokens_match(expected: &str, provided: &str) -> bool {
    let expected = expected.trim().as_bytes();
    let provided = provided.trim().as_bytes();
    if expected.is_empty() || expected.len() != provided.len() {
        return false;
    }

    let mut diff = 0u8;
    for (&a, &b) in expected.iter().zip(provided.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn parse_media_type(raw: Option<&str>) -> Result<MediaType, ImportError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(MediaType::Movie),
        Some(value) if value.eq_ignore_ascii_case("movie") => Ok(MediaType::Movie),
        Some(value) if value.eq_ignore_ascii_case("game") => Ok(MediaType::Game),
        Some(other) => Err(ImportError::BadRequest(format!(
            "Unsupported media type: {other}"
        ))),
    }
}

fn parse_film_status(raw: Option<&str>) -> Result<FilmStatus, ImportError> {
    let Some(value) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(FilmStatus::WantToWatch);
    };
    match value.to_ascii_lowercase().as_str() {
        "want_to_watch" => Ok(FilmStatus::WantToWatch),
        "watching" => Ok(FilmStatus::Watching),
        "watched" => Ok(FilmStatus::Watched),
        "dropped" => Ok(FilmStatus::Dropped),
        _ => Err(ImportError::BadRequest(format!(
            "Unsupported status: {value}"
        ))),
    }
}

fn detect_game_platform_launcher(source_lower: &str) -> (Option<String>, Option<String>) {
    if source_lower.contains("store.steampowered.com") {
        (Some("ПК".to_owned()), Some("Steam".to_owned()))
    } else if source_lower.contains("epicgames.com") {
        (Some("ПК".to_owned()), Some("Epic Games".to_owned()))
    } else if source_lower.contains("gog.com") {
        (Some("ПК".to_owned()), Some("GOG".to_owned()))
    } else if source_lower.contains("itch.io") {
        (Some("ПК".to_owned()), Some("Itch.io".to_owned()))
    } else {
        (None, None)
    }
}

fn game_completion_for_status(status: FilmStatus) -> Option<String> {
    match status {
        FilmStatus::WantToWatch => None,
        FilmStatus::Watching => Some("В процессе".to_owned()),
        FilmStatus::Watched => Some("Пройдена".to_owned()),
        FilmStatus::Dropped => Some("Брошена".to_owned()),
    }
}

pub fn extract_imdb_id(source_url: Option<&str>) -> String {
    let Some(url) = source_url else {
        return String::new();
    };
    let lower = url.to_ascii_lowercase();
    if !lower.contains("imdb.com") {
        return String::new();
    }

    let Some(pos) = lower.find("/title/tt") else {
        return String::new();
    };
    let rest = &lower[pos + "/title/".len()..];
    let id: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if id.starts_with("tt") && id.len() >= 4 && id[2..].chars().all(|c| c.is_ascii_digit()) {
        truncate_chars(&id, NewFilm::MAX_IMDB_ID_CHARS)
    } else {
        String::new()
    }
}

fn normalize_imdb_id(explicit: Option<&str>, source_url: Option<&str>) -> String {
    if let Some(raw) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        let lower = raw.to_ascii_lowercase();
        if lower.starts_with("tt")
            && lower.len() >= 4
            && lower[2..].chars().all(|c| c.is_ascii_digit())
        {
            return truncate_chars(&lower, NewFilm::MAX_IMDB_ID_CHARS);
        }
    }
    extract_imdb_id(source_url)
}

pub fn build_draft_from_payload(
    payload: &ExtensionImportPayload,
    poster_path: Option<String>,
) -> Result<NewFilm, ImportError> {
    let title = truncate_chars(&payload.title, NewFilm::MAX_TITLE_CHARS);
    if title.is_empty() {
        return Err(ImportError::BadRequest(
            "Title must not be empty".to_owned(),
        ));
    }

    let media_type = parse_media_type(payload.media_type.as_deref())?;
    let status = parse_film_status(payload.status.as_deref())?;
    let source_lower = payload
        .source_url
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    let year = payload.year.filter(|y| (MIN_YEAR..=MAX_YEAR).contains(y));
    let original_title = truncate_chars(&payload.original_title, NewFilm::MAX_ORIGINAL_TITLE_CHARS);
    let director = truncate_chars(&payload.director, NewFilm::MAX_DIRECTOR_CHARS);
    let genres_raw = metadata::translate_genres(payload.genres.trim());
    let genres = truncate_chars(&genres_raw, NewFilm::MAX_GENRES_CHARS);
    let country = truncate_chars(&payload.country, NewFilm::MAX_COUNTRY_CHARS);
    let runtime_minutes = payload
        .runtime_minutes
        .filter(|m| (1..=MAX_RUNTIME_MINUTES).contains(m));
    let description = payload.description.trim().to_owned();

    let requested_category = payload
        .category
        .as_deref()
        .map(|s| truncate_chars(s, NewFilm::MAX_CATEGORY_CHARS))
        .filter(|s| !s.is_empty());

    let mut draft = NewFilm {
        title,
        original_title,
        director,
        year,
        genres,
        country,
        runtime_minutes,
        description,
        poster_path,
        status,
        media_type,
        ..Default::default()
    };

    if media_type == MediaType::Game {
        draft.category = requested_category.unwrap_or_else(|| "Игры".to_owned());
        draft.completion_status = game_completion_for_status(status);
        let (detected_platform, detected_launcher) = detect_game_platform_launcher(&source_lower);
        draft.platform = payload
            .platform
            .as_deref()
            .map(|s| truncate_chars(s, NewFilm::MAX_PLATFORM_CHARS))
            .filter(|s| !s.is_empty())
            .or(detected_platform);
        draft.launcher = payload
            .launcher
            .as_deref()
            .map(|s| truncate_chars(s, NewFilm::MAX_LAUNCHER_CHARS))
            .filter(|s| !s.is_empty())
            .or(detected_launcher);
    } else {
        let seasons_total = payload
            .seasons_total
            .filter(|&s| s <= MAX_SEASONS)
            .unwrap_or(0);
        draft.seasons_total = seasons_total;
        draft.category = requested_category.unwrap_or_else(|| {
            if source_lower.contains("kinopoisk.ru/series/") || seasons_total > 0 {
                "Сериалы".to_owned()
            } else {
                "Фильмы".to_owned()
            }
        });
        draft.imdb_id =
            normalize_imdb_id(payload.imdb_id.as_deref(), payload.source_url.as_deref());
    }

    Ok(draft)
}

fn collect_poster_urls(payload: &ExtensionImportPayload) -> Vec<String> {
    let mut urls = Vec::new();
    for raw in payload
        .poster_url
        .iter()
        .chain(payload.poster_candidates.iter())
    {
        let trimmed = raw.trim();
        if !trimmed.is_empty() && !urls.iter().any(|existing| existing == trimmed) {
            urls.push(trimmed.to_owned());
        }
    }
    urls
}

/// Frames a landscape game banner into a 600x900 vertical 2:3 poster so
/// horizontal Steam store artwork looks crisp inside Lierkour's poster grid.
fn frame_horizontal_game_poster(source: &image::DynamicImage) -> image::RgbaImage {
    const TARGET_W: u32 = 600;
    const TARGET_H: u32 = 900;

    let backdrop = source.resize_to_fill(TARGET_W, TARGET_H, image::imageops::FilterType::Triangle);
    let mut canvas = backdrop.blur(22.0).to_rgba8();
    for px in canvas.pixels_mut() {
        px[0] = ((px[0] as u16 * 38) / 100) as u8;
        px[1] = ((px[1] as u16 * 38) / 100) as u8;
        px[2] = ((px[2] as u16 * 38) / 100) as u8;
        px[3] = 255;
    }

    let fg = source
        .resize(TARGET_W - 44, 560, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    let x = ((TARGET_W - fg.width()) / 2) as i64;
    let y = ((TARGET_H - fg.height()) / 2) as i64;
    image::imageops::overlay(&mut canvas, &fg, x, y);
    canvas
}

fn import_poster_for_payload(
    payload: &ExtensionImportPayload,
    media_type: MediaType,
    paths: &AppPaths,
) -> (Option<String>, bool) {
    for url in collect_poster_urls(payload) {
        let bytes = match metadata::download_poster_url(&url) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("[LierEX] Failed to download poster from {url}: {err}");
                continue;
            }
        };

        let decoded = match image::load_from_memory(&bytes) {
            Ok(img) => img,
            Err(err) => {
                eprintln!("[LierEX] Invalid poster image from {url}: {err}");
                continue;
            }
        };

        let rgba = if media_type == MediaType::Game && decoded.width() > decoded.height() {
            frame_horizontal_game_poster(&decoded)
        } else {
            decoded.to_rgba8()
        };

        match storage::import_poster_image(&rgba, paths) {
            Ok(imported) => return (Some(imported.relative_path), true),
            Err(err) => {
                eprintln!("[LierEX] Failed to save poster from {url}: {err}");
            }
        }
    }

    (None, false)
}

pub fn import_extension_draft(
    db: &Arc<Mutex<Database>>,
    paths: &AppPaths,
    payload: ExtensionImportPayload,
) -> Result<(Film, bool), ImportError> {
    if payload.title.trim().is_empty() {
        return Err(ImportError::BadRequest(
            "Title must not be empty".to_owned(),
        ));
    }
    let media_type = parse_media_type(payload.media_type.as_deref())?;
    let (poster_path, poster_imported) = import_poster_for_payload(&payload, media_type, paths);

    let draft = build_draft_from_payload(&payload, poster_path)?;

    let mut db_lock = db
        .lock()
        .map_err(|e| ImportError::Internal(format!("Database lock error: {e}")))?;

    let film_id = db_lock.insert_film(&draft).map_err(|e| {
        if let Some(ref path) = draft.poster_path {
            let _ = storage::delete_poster(paths, path);
        }
        ImportError::BadRequest(e.to_string())
    })?;

    let film = db_lock
        .get_film(film_id)
        .map_err(|e| ImportError::Internal(format!("Failed to load imported draft: {e}")))?;

    Ok((film, poster_imported))
}

pub fn create_tauri_import_callback(
    db: Arc<Mutex<Database>>,
    paths: AppPaths,
    app_handle: AppHandle,
) -> ImportCallback {
    Arc::new(move |payload| {
        let (film, poster_imported) = import_extension_draft(&db, &paths, payload)?;
        if let Err(err) = app_handle.emit(EXTENSION_IMPORT_EVENT, &film) {
            eprintln!("[LierEX] Failed to emit GUI import event: {err}");
        }
        Ok((film, poster_imported))
    })
}

async fn cors_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let mut response = if req.method() == Method::OPTIONS {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(req).await
    };

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
        HeaderValue::from_static("Content-Type, X-LierEX-Token"),
    );
    headers.insert(
        header::HeaderName::from_static("access-control-allow-private-network"),
        HeaderValue::from_static("true"),
    );

    response
}

fn is_authorized(headers: &HeaderMap, expected_token: &Arc<RwLock<String>>) -> bool {
    let Some(provided) = headers
        .get(EXTENSION_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };

    let Ok(expected) = expected_token.read() else {
        return false;
    };

    tokens_match(&expected, provided)
}

async fn health_handler(
    State(state): State<ExtensionApiState>,
    headers: HeaderMap,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        app: "Lierkour".to_owned(),
        version: "2.3.0".to_owned(),
        authorized: is_authorized(&headers, &state.token),
    })
}

async fn lier_info_handler(
    State(state): State<ExtensionApiState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "ecosystem": "LIER",
        "app": "Lierkour",
        "status": "running",
        "version": "2.3.0",
        "port": EXTENSION_API_PORT,
        "platform": std::env::consts::OS,
        "license": "GPL-3.0-only",
        "authorized": is_authorized(&headers, &state.token),
        "capabilities": ["movie_import", "game_import", "ost_search", "mobile_sync"]
    }))
}

async fn ecosystem_search_handler(
    State(state): State<ExtensionApiState>,
    Query(params): Query<EcosystemSearchQuery>,
) -> impl IntoResponse {
    let q = params.q.trim().to_ascii_lowercase();
    let db = Arc::clone(&state.db);
    let paths = state.paths.clone();

    let items = tokio::task::spawn_blocking(move || -> Vec<serde_json::Value> {
        let Ok(guard) = db.lock() else {
            return Vec::new();
        };
        let Ok(all) = guard.list_all_films() else {
            return Vec::new();
        };
        all.into_iter()
            .filter(|f| {
                q.is_empty()
                    || f.title.to_ascii_lowercase().contains(&q)
                    || f.original_title.to_ascii_lowercase().contains(&q)
                    || f.director.to_ascii_lowercase().contains(&q)
                    || f.genres.to_ascii_lowercase().contains(&q)
            })
            .take(10)
            .map(|f| {
                use base64::Engine;
                let poster_data_uri = f.poster_path.as_deref().and_then(|rel| {
                    let abs = paths.resolve_poster(rel);
                    let bytes = std::fs::read(abs).ok()?;
                    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
                        "image/png"
                    } else if bytes.starts_with(b"RIFF") {
                        "image/webp"
                    } else {
                        "image/jpeg"
                    };
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    Some(format!("data:{mime};base64,{b64}"))
                });
                serde_json::json!({
                    "id": f.id,
                    "title": f.title,
                    "original_title": if f.original_title.is_empty() { serde_json::Value::Null } else { serde_json::json!(f.original_title) },
                    "media_type": f.media_type.as_db_str(),
                    "creator": if f.director.is_empty() { serde_json::Value::Null } else { serde_json::json!(f.director) },
                    "year": f.year.map(u32::from),
                    "genres": if f.genres.is_empty() { serde_json::Value::Null } else { serde_json::json!(f.genres) },
                    "poster_data_uri": poster_data_uri
                })
            })
            .collect()
    })
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "items": items }))
}

async fn import_handler(
    State(state): State<ExtensionApiState>,
    headers: HeaderMap,
    Json(payload): Json<ExtensionImportPayload>,
) -> Response {
    if !is_authorized(&headers, &state.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid or missing X-LierEX-Token".to_owned(),
            }),
        )
            .into_response();
    }

    if payload.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Title must not be empty".to_owned(),
            }),
        )
            .into_response();
    }

    let on_import = Arc::clone(&state.on_import);
    let result = tokio::task::spawn_blocking(move || on_import(payload)).await;

    match result {
        Ok(Ok((film, poster_imported))) => (
            StatusCode::OK,
            Json(ExtensionImportResponse {
                ok: true,
                film,
                poster_imported,
            }),
        )
            .into_response(),
        Ok(Err(ImportError::BadRequest(msg))) => {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: msg })).into_response()
        }
        Ok(Err(ImportError::Internal(msg))) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: msg }),
        )
            .into_response(),
        Err(join_err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Execution error: {join_err}"),
            }),
        )
            .into_response(),
    }
}

pub fn build_router(state: ExtensionApiState) -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/lier/info", get(lier_info_handler))
        .route("/api/ecosystem/search", get(ecosystem_search_handler))
        .route("/api/import", post(import_handler))
        .with_state(state)
        .layer(middleware::from_fn(cors_middleware))
}

pub fn stop_extension_server(
    handle_arc: &Arc<Mutex<ExtensionServerHandle>>,
) -> ExtensionServerStatus {
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

/// Запускает локальный HTTP-сервер (`127.0.0.1:{port}`) на базе `axum` для приёма
/// импортированных данных от браузерного расширения LierEX через callback.
pub fn start_extension_server(
    handle_arc: &Arc<Mutex<ExtensionServerHandle>>,
    port: u16,
    token: Arc<RwLock<String>>,
    on_import: ImportCallback,
    db: Arc<Mutex<Database>>,
    paths: AppPaths,
) -> ExtensionServerStatus {
    stop_extension_server(handle_arc);

    let address = format!("127.0.0.1:{port}");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let std_listener = match std::net::TcpListener::bind(&address) {
        Ok(listener) => listener,
        Err(err) => {
            let mut handle = match handle_arc.lock() {
                Ok(h) => h,
                Err(e) => e.into_inner(),
            };
            handle.status.active = false;
            handle.status.port = port;
            handle.status.address = address;
            handle.status.error = Some(err.to_string());
            eprintln!("[LierEX] Failed to bind {}: {err}", handle.status.address);
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
        handle.status.address = address;
        handle.status.error = Some(err.to_string());
        eprintln!("[LierEX] Failed to configure listener: {err}");
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
        handle.status.address = address;
        handle.status.error = None;
    }

    let status_to_return = {
        let handle = match handle_arc.lock() {
            Ok(h) => h,
            Err(e) => e.into_inner(),
        };
        handle.status.clone()
    };

    let router = build_router(ExtensionApiState {
        token,
        on_import,
        db,
        paths,
    });
    let handle_for_task = Arc::clone(handle_arc);

    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(std_listener) {
            Ok(listener) => listener,
            Err(err) => {
                if let Ok(mut h) = handle_for_task.lock() {
                    h.status.active = false;
                    h.status.error = Some(err.to_string());
                }
                eprintln!("[LierEX] Failed to adopt Tokio listener: {err}");
                return;
            }
        };

        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

        if let Err(err) = server.await {
            if let Ok(mut h) = handle_for_task.lock() {
                h.status.active = false;
                h.status.error = Some(err.to_string());
            }
            eprintln!("[LierEX] Extension API server stopped: {err}");
        } else if let Ok(mut h) = handle_for_task.lock() {
            h.status.active = false;
        }
    });

    status_to_return
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("lierkour-ext-test-{name}-{nanos}"))
    }

    fn spawn_test_server(
        token: &str,
        db: Arc<Mutex<Database>>,
        paths: AppPaths,
    ) -> (std::net::SocketAddr, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let token = Arc::new(RwLock::new(token.to_owned()));
        let db_state = Arc::clone(&db);
        let paths_state = paths.clone();
        let on_import: ImportCallback =
            Arc::new(move |payload| import_extension_draft(&db, &paths, payload));
        let router = build_router(ExtensionApiState {
            token,
            on_import,
            db: db_state,
            paths: paths_state,
        });

        let listener = rt
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let addr = listener.local_addr().unwrap();

        rt.spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        (addr, rt)
    }

    async fn send_raw_http(addr: std::net::SocketAddr, raw_request: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(raw_request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).into_owned()
    }

    #[test]
    fn extracts_imdb_id_and_builds_movie_and_game_drafts() {
        assert_eq!(
            extract_imdb_id(Some(
                "https://www.imdb.com/title/tt0133093/?ref_=fn_al_tt_1"
            )),
            "tt0133093"
        );
        assert_eq!(
            extract_imdb_id(Some("https://www.kinopoisk.ru/film/326/")),
            ""
        );

        let game_payload = ExtensionImportPayload {
            title: "The Witcher 3".to_owned(),
            year: Some(2015),
            media_type: Some("game".to_owned()),
            status: Some("watched".to_owned()),
            genres: "Action, RPG".to_owned(),
            country: "CD PROJEKT RED".to_owned(),
            description: "RPG".to_owned(),
            source_url: Some("https://store.steampowered.com/app/292030/".to_owned()),
            ..Default::default()
        };

        let game_draft = build_draft_from_payload(&game_payload, None).unwrap();
        assert_eq!(game_draft.media_type, MediaType::Game);
        assert_eq!(game_draft.status, FilmStatus::Watched);
        assert_eq!(game_draft.completion_status.as_deref(), Some("Пройдена"));
        assert_eq!(game_draft.category, "Игры");
        assert_eq!(game_draft.genres, "Боевик, RPG");
        assert_eq!(game_draft.country, "CD PROJEKT RED");
        assert_eq!(game_draft.launcher.as_deref(), Some("Steam"));
        assert_eq!(game_draft.platform.as_deref(), Some("ПК"));

        let series_payload = ExtensionImportPayload {
            title: "Аркейн".to_owned(),
            original_title: "Arcane".to_owned(),
            year: Some(2021),
            director: "Паскаль Шаррю".to_owned(),
            genres: "фантастика, боевик".to_owned(),
            country: "США".to_owned(),
            runtime_minutes: Some(42),
            seasons_total: Some(2),
            description: "Анимационный сериал".to_owned(),
            source_url: Some("https://www.kinopoisk.ru/series/4445150/".to_owned()),
            ..Default::default()
        };

        let series_draft = build_draft_from_payload(&series_payload, None).unwrap();
        assert_eq!(series_draft.media_type, MediaType::Movie);
        assert_eq!(series_draft.status, FilmStatus::WantToWatch);
        assert_eq!(series_draft.category, "Сериалы");
        assert_eq!(series_draft.director, "Паскаль Шаррю");
        assert_eq!(series_draft.genres, "фантастика, боевик");
        assert_eq!(series_draft.country, "США");
        assert_eq!(series_draft.runtime_minutes, Some(42));
        assert_eq!(series_draft.seasons_total, 2);

        for (url, expected_launcher) in [
            (
                "https://store.epicgames.com/en-US/p/alan-wake-2",
                "Epic Games",
            ),
            ("https://www.gog.com/en/game/hollow_knight", "GOG"),
            ("https://tobyfox.itch.io/undertale", "Itch.io"),
        ] {
            let storefront_draft = build_draft_from_payload(
                &ExtensionImportPayload {
                    title: "Test Game".to_owned(),
                    media_type: Some("game".to_owned()),
                    genres: "Indie, Role-Playing".to_owned(),
                    source_url: Some(url.to_owned()),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
            assert_eq!(storefront_draft.platform.as_deref(), Some("ПК"));
            assert_eq!(
                storefront_draft.launcher.as_deref(),
                Some(expected_launcher)
            );
            assert_eq!(storefront_draft.genres, "Инди, RPG");
        }
    }

    #[test]
    fn frames_horizontal_game_poster_to_vertical_aspect_ratio() {
        let horizontal = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            460,
            215,
            image::Rgba([20, 80, 160, 255]),
        ));
        let framed = frame_horizontal_game_poster(&horizontal);
        assert_eq!(framed.width(), 600);
        assert_eq!(framed.height(), 900);
    }

    #[test]
    fn load_or_create_token_is_persistent() {
        let dir = temp_dir("token");
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();
        let mut db = Database::open(&paths.db_path).unwrap();

        let first = load_or_create_token(&mut db).unwrap();
        let second = load_or_create_token(&mut db).unwrap();
        assert!(!first.is_empty());
        assert_eq!(first, second);

        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_endpoint_enforces_cors_auth_validation_and_saves_draft() {
        let dir = temp_dir("http");
        let paths = AppPaths::from_data_dir(dir.clone()).unwrap();
        let db = Arc::new(Mutex::new(Database::open(&paths.db_path).unwrap()));

        let (addr, rt) = spawn_test_server("secret-token-123", Arc::clone(&db), paths);

        rt.block_on(async {
            // 1. CORS preflight
            let preflight = send_raw_http(
                addr,
                "OPTIONS /api/import HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            )
            .await;
            assert!(preflight.starts_with("HTTP/1.1 204"));
            assert!(
                preflight
                    .to_ascii_lowercase()
                    .contains("access-control-allow-origin: *")
            );
            assert!(
                preflight
                    .to_ascii_lowercase()
                    .contains("access-control-allow-headers: content-type, x-lierex-token")
            );

            // 2. Unauthorized without token
            let body = r#"{"title":"Inception","year":2010,"type":"movie"}"#;
            let unauth_req = format!(
                "POST /api/import HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let unauth = send_raw_http(addr, &unauth_req).await;
            assert!(unauth.starts_with("HTTP/1.1 401"));

            // 3. Bad request on empty title
            let empty_body = r#"{"title":"   ","year":2024}"#;
            let bad_req = format!(
                "POST /api/import HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-LierEX-Token: secret-token-123\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                empty_body.len(),
                empty_body
            );
            let bad = send_raw_http(addr, &bad_req).await;
            assert!(bad.starts_with("HTTP/1.1 400"));

            // 4. Valid import saves to DB and returns 200 OK
            let valid_body = r#"{"title":"Интерстеллар","original_title":"Interstellar","year":2014,"type":"movie","description":"Sci-fi","source_url":"https://www.imdb.com/title/tt0816692/"}"#;
            let ok_req = format!(
                "POST /api/import HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nX-LierEX-Token: secret-token-123\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                valid_body.len(),
                valid_body
            );
            let ok = send_raw_http(addr, &ok_req).await;
            assert!(ok.starts_with("HTTP/1.1 200"), "unexpected response: {ok}");
            assert!(ok.contains("tt0816692"));
        });

        let films = db.lock().unwrap().list_all_films().unwrap();
        assert_eq!(films.len(), 1);
        assert_eq!(films[0].title, "Интерстеллар");
        assert_eq!(films[0].original_title, "Interstellar");
        assert_eq!(films[0].year, Some(2014));
        assert_eq!(films[0].imdb_id, "tt0816692");

        drop(rt);
        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }
}
