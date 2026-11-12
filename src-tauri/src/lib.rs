mod commands;
mod extension_api;
mod sync_api;

use commands::*;
use extension_api::{create_tauri_import_callback, load_or_create_token};
use lierkour_core::{AppPaths, Database};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Portable mode: the launcher scripts (`run-portable.bat` on
            // Windows and `run-portable.sh` on Linux) point this variable at
            // the `data` folder next to the executable. Otherwise use the
            // platform's per-user application data directory.
            let data_dir = std::env::var_os(lierkour_core::storage::DATA_DIR_ENV)
                .or_else(|| std::env::var_os(lierkour_core::storage::LEGACY_DATA_DIR_ENV))
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    app.path()
                        .app_data_dir()
                        .expect("Failed to resolve app data directory")
                });

            let paths = AppPaths::from_data_dir(data_dir).expect("Failed to initialize app paths");

            // Seamless migration from legacy directory (v1.0/v1.1 MyFilmLibrary)
            if !paths.db_path.exists()
                && let Ok(legacy) = AppPaths::discover()
                && legacy.db_path.exists()
                && legacy.db_path != paths.db_path
            {
                let _ = fs::copy(&legacy.db_path, &paths.db_path);
                if legacy.posters_dir.exists()
                    && let Ok(entries) = fs::read_dir(&legacy.posters_dir)
                {
                    for entry in entries.flatten() {
                        let dest = paths.posters_dir.join(entry.file_name());
                        if !dest.exists() {
                            let _ = fs::copy(entry.path(), dest);
                        }
                    }
                }
                if legacy.screenshots_dir.exists()
                    && let Ok(entries) = fs::read_dir(&legacy.screenshots_dir)
                {
                    for entry in entries.flatten() {
                        let dest = paths.screenshots_dir.join(entry.file_name());
                        if !dest.exists() {
                            let _ = fs::copy(entry.path(), dest);
                        }
                    }
                }
            }

            let mut db = Database::open(&paths.db_path).expect("Failed to open SQLite database");
            let token_str =
                load_or_create_token(&mut db).unwrap_or_else(|_| extension_api::generate_token());

            let ext_enabled = db
                .get_setting(extension_api::EXTENSION_SERVER_ENABLED_SETTING)
                .unwrap_or(None)
                .map(|v| v != "false")
                .unwrap_or(true);
            let ext_port = db
                .get_setting(extension_api::EXTENSION_SERVER_PORT_SETTING)
                .unwrap_or(None)
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(extension_api::EXTENSION_API_PORT);

            let sync_enabled = db
                .get_setting(sync_api::SYNC_SERVER_ENABLED_SETTING)
                .unwrap_or(None)
                .map(|v| v != "false")
                .unwrap_or(true);
            let sync_port = db
                .get_setting(sync_api::SYNC_SERVER_PORT_SETTING)
                .unwrap_or(None)
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(sync_api::DEFAULT_SYNC_PORT);

            let db_arc = Arc::new(Mutex::new(db));
            let extension_token = Arc::new(RwLock::new(token_str));
            let extension_handle = Arc::new(Mutex::new(extension_api::ExtensionServerHandle::default()));
            let sync_handle = Arc::new(Mutex::new(sync_api::SyncServerHandle::default()));

            let on_import = create_tauri_import_callback(
                Arc::clone(&db_arc),
                paths.clone(),
                app.handle().clone(),
            );

            if ext_enabled {
                extension_api::start_extension_server(
                    &extension_handle,
                    ext_port,
                    Arc::clone(&extension_token),
                    Arc::clone(&on_import),
                );
            }
            if sync_enabled {
                sync_api::start_sync_server(
                    &sync_handle,
                    sync_port,
                    Arc::clone(&db_arc),
                    paths.clone(),
                );
            }

            app.manage(AppState {
                db: db_arc,
                paths,
                extension_token,
                extension_handle,
                sync_handle,
                on_import,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_films,
            get_filtered_films,
            get_seasons,
            detach_season,
            get_film,
            add_film,
            update_film,
            delete_film,
            get_categories,
            get_stats,
            get_theme,
            set_theme,
            get_setting,
            set_setting,
            get_extension_info,
            toggle_extension_server,
            regenerate_extension_token,
            get_sync_server_status,
            toggle_sync_server,
            get_network_ips,
            import_poster_file,
            import_poster_from_base64,
            resolve_poster_asset,
            fetch_omdb_metadata,
            get_film_relations,
            add_film_relation,
            remove_film_relation,
            get_film_screenshots,
            resolve_screenshot_asset,
            add_film_screenshot,
            delete_film_screenshot,
            export_backup_zip,
            import_backup_zip,
            export_films_csv,
            export_films_markdown
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
