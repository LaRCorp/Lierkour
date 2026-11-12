//! JNI-мост для интеграции ядра Lierkour (Rust) с Android (Kotlin / Jetpack Compose).
//!
//! Экспортирует функции под пакет `com.larcorp.lierkour.RustBridge`.
//! Все данные передаются в формате JSON для простоты и гибкости.

#[cfg(feature = "android")]
use std::path::PathBuf;
#[cfg(feature = "android")]
use std::sync::Mutex;

#[cfg(feature = "android")]
use jni::JNIEnv;
#[cfg(feature = "android")]
use jni::objects::{JClass, JString};
#[cfg(feature = "android")]
use jni::sys::{jboolean, jlong, jstring};

#[cfg(feature = "android")]
use crate::db::Database;
#[cfg(feature = "android")]
use crate::models::{FilmFilter, NewFilm};
#[cfg(feature = "android")]
use crate::sync::SyncPayload;

#[cfg(feature = "android")]
static DB_INSTANCE: Mutex<Option<Database>> = Mutex::new(None);
#[cfg(feature = "android")]
static APP_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[cfg(feature = "android")]
fn to_jstring<'a>(env: &mut JNIEnv<'a>, text: &str) -> jstring {
    env.new_string(text)
        .unwrap_or_else(|_| env.new_string("").unwrap())
        .into_raw()
}

#[cfg(feature = "android")]
fn get_rust_string(env: &mut JNIEnv, j_str: JString) -> String {
    if j_str.is_null() {
        return String::new();
    }
    match env.get_string(&j_str) {
        Ok(java_str) => java_str.into(),
        Err(_) => String::new(),
    }
}

/// Инициализация БД в каталоге приложения на Android.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_initDatabase(
    mut env: JNIEnv,
    _class: JClass,
    data_dir: JString,
) -> jboolean {
    let path_str = get_rust_string(&mut env, data_dir);
    if path_str.is_empty() {
        return 0;
    }

    let dir = PathBuf::from(path_str);
    let db_path = dir.join("lierkour.db");

    match Database::open(&db_path) {
        Ok(db) => {
            let mut guard = DB_INSTANCE.lock().unwrap();
            *guard = Some(db);
            let mut dir_guard = APP_DIR.lock().unwrap();
            *dir_guard = Some(dir);
            1
        }
        Err(e) => {
            eprintln!("[RustBridge] Failed to open DB: {e:?}");
            0
        }
    }
}

/// Получить все активные карточки медиа (фильмы и игры) в виде JSON.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_getAllMedia(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_ref() else {
        return to_jstring(&mut env, "[]");
    };

    match db.list_all_films() {
        Ok(films) => {
            let json = serde_json::to_string(&films).unwrap_or_else(|_| "[]".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(e) => {
            eprintln!("[RustBridge] getAllMedia error: {e:?}");
            to_jstring(&mut env, "[]")
        }
    }
}

/// Получить отфильтрованные карточки (фильтр передаётся как JSON-строка).
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_getFilteredMedia(
    mut env: JNIEnv,
    _class: JClass,
    filter_json: JString,
) -> jstring {
    let guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_ref() else {
        return to_jstring(&mut env, "[]");
    };

    let filter_str = get_rust_string(&mut env, filter_json);
    let filter: FilmFilter = serde_json::from_str(&filter_str).unwrap_or_default();

    match db.list_root_films(&filter) {
        Ok(films) => {
            let json = serde_json::to_string(&films).unwrap_or_else(|_| "[]".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(e) => {
            eprintln!("[RustBridge] getFilteredMedia error: {e:?}");
            to_jstring(&mut env, "[]")
        }
    }
}

/// Получить одну карточку по ID.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_getFilm(
    mut env: JNIEnv,
    _class: JClass,
    id: jlong,
) -> jstring {
    let guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_ref() else {
        return to_jstring(&mut env, "null");
    };

    match db.get_film(id) {
        Ok(film) => {
            let json = serde_json::to_string(&film).unwrap_or_else(|_| "null".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(_) => to_jstring(&mut env, "null"),
    }
}

/// Добавить новый фильм или игру. Принимает JSON черновика NewFilm, возвращает ID или -1.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_addMedia(
    mut env: JNIEnv,
    _class: JClass,
    draft_json: JString,
) -> jlong {
    let mut guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_mut() else {
        return -1;
    };

    let draft_str = get_rust_string(&mut env, draft_json);
    let draft: NewFilm = match serde_json::from_str(&draft_str) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[RustBridge] Invalid draft JSON: {e:?}");
            return -1;
        }
    };

    match db.insert_film(&draft) {
        Ok(new_id) => new_id,
        Err(e) => {
            eprintln!("[RustBridge] insert_film error: {e:?}");
            -1
        }
    }
}

/// Обновить карточку медиа.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_updateMedia(
    mut env: JNIEnv,
    _class: JClass,
    id: jlong,
    draft_json: JString,
) -> jboolean {
    let mut guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_mut() else {
        return 0;
    };

    let draft_str = get_rust_string(&mut env, draft_json);
    let draft: NewFilm = match serde_json::from_str(&draft_str) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[RustBridge] Invalid update draft JSON: {e:?}");
            return 0;
        }
    };

    match db.update_film(id, &draft) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("[RustBridge] update_film error: {e:?}");
            0
        }
    }
}

/// Удалить карточку (soft delete).
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_deleteMedia(
    _env: JNIEnv,
    _class: JClass,
    id: jlong,
) -> jboolean {
    let mut guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_mut() else {
        return 0;
    };

    match db.delete_film(id) {
        Ok(_) => 1,
        Err(e) => {
            eprintln!("[RustBridge] delete_film error: {e:?}");
            0
        }
    }
}

/// Получить сводную статистику библиотеки (Dashboard).
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_getDashboardStats(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_ref() else {
        return to_jstring(&mut env, "null");
    };

    match db.get_dashboard_stats() {
        Ok(stats) => {
            let json = serde_json::to_string(&stats).unwrap_or_else(|_| "null".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(e) => {
            eprintln!("[RustBridge] getDashboardStats error: {e:?}");
            to_jstring(&mut env, "null")
        }
    }
}

/// Синхронизация: получить локальные изменения с момента `since`.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_getSyncDelta(
    mut env: JNIEnv,
    _class: JClass,
    since: JString,
    client_id: JString,
) -> jstring {
    let guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_ref() else {
        return to_jstring(&mut env, "{}");
    };

    let since_str = get_rust_string(&mut env, since);
    let client_str = get_rust_string(&mut env, client_id);

    let since_opt = if since_str.is_empty() { None } else { Some(since_str.as_str()) };

    match db.get_sync_delta(since_opt, &client_str) {
        Ok(payload) => {
            let json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(e) => {
            eprintln!("[RustBridge] getSyncDelta error: {e:?}");
            to_jstring(&mut env, "{}")
        }
    }
}

/// Синхронизация: применить полученные изменения.
#[cfg(feature = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_larcorp_lierkour_RustBridge_applySyncDelta(
    mut env: JNIEnv,
    _class: JClass,
    payload_json: JString,
) -> jstring {
    let mut guard = DB_INSTANCE.lock().unwrap();
    let Some(db) = guard.as_mut() else {
        return to_jstring(&mut env, "null");
    };

    let payload_str = get_rust_string(&mut env, payload_json);
    let payload: SyncPayload = match serde_json::from_str(&payload_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[RustBridge] Invalid sync payload JSON: {e:?}");
            return to_jstring(&mut env, "null");
        }
    };

    match db.apply_sync_delta(&payload) {
        Ok(summary) => {
            let json = serde_json::to_string(&summary).unwrap_or_else(|_| "null".to_owned());
            to_jstring(&mut env, &json)
        }
        Err(e) => {
            eprintln!("[RustBridge] applySyncDelta error: {e:?}");
            to_jstring(&mut env, "null")
        }
    }
}
