//! Модуль двусторонней синхронизации (Offline-First Sync Engine).
//!
//! Реализует протокол дельта-синхронизации (LWW — Last-Write-Wins)
//! на основе UUID (`sync_id`), меток времени (`updated_at`) и мягкого удаления (`deleted_at`).

use std::fs;
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{Film, FilmRelation, FilmScreenshot, MediaType};

/// Пакет изменений для передачи между устройствами (дельта).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncPayload {
    /// Уникальный идентификатор устройства/клиента
    pub client_id: String,
    /// Временная метка последней синхронизации (ISO-8601 UTC).
    /// Если `None` — запрашивается/передаётся вся библиотека.
    pub since: Option<String>,
    /// Изменённые или удалённые карточки медиа
    pub films: Vec<Film>,
    /// Изменённые или удалённые связи между тайтлами
    pub relations: Vec<FilmRelation>,
    /// Изменённые или удалённые скриншоты/кадры
    pub screenshots: Vec<FilmScreenshot>,
}

/// Сводка результатов применения изменений.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncSummary {
    pub applied_films: usize,
    pub applied_relations: usize,
    pub applied_screenshots: usize,
    pub server_time: String,
}

impl Database {
    /// Возвращает все изменения (включая мягко удалённые записи) с момента `since`.
    pub fn get_sync_delta(&self, since: Option<&str>, client_id: &str) -> AppResult<SyncPayload> {
        let since_param = since.filter(|s| !s.trim().is_empty());

        // 1. Фильмы и игры
        let mut film_stmt = self.conn.prepare(
            "SELECT id, title, original_title, category, director, year, genres, country,
                    runtime_minutes, imdb_id, tags, status, rating, description, thoughts,
                    poster_path, seasons_total, watched_seasons, quotes, parent_id,
                    media_type, platform, launcher, playtime_hours, completion_status,
                    sync_id, updated_at, deleted_at
             FROM films
             WHERE ?1 IS NULL OR updated_at > ?1
             ORDER BY updated_at ASC",
        )?;

        let film_rows = film_stmt.query_map([since_param], |row| {
            let seasons_total: u16 = row.get("seasons_total")?;
            let watched_seasons: String = row.get("watched_seasons")?;
            let quotes_str: String = row.get("quotes").unwrap_or_else(|_| "[]".to_owned());
            let media_type_str: Option<String> = row.get("media_type").unwrap_or(None);
            let media_type = media_type_str
                .as_deref()
                .and_then(MediaType::from_db_str)
                .unwrap_or(MediaType::Movie);

            Ok(Film {
                id: row.get("id")?,
                title: row.get("title")?,
                original_title: row.get("original_title")?,
                category: row.get("category")?,
                director: row.get("director")?,
                year: row.get("year")?,
                genres: row.get("genres")?,
                country: row.get("country")?,
                runtime_minutes: row.get("runtime_minutes")?,
                imdb_id: row.get("imdb_id")?,
                tags: row.get("tags")?,
                status: row.get("status")?,
                rating: row.get("rating")?,
                description: row.get("description")?,
                thoughts: row.get("thoughts")?,
                poster_path: row.get("poster_path")?,
                seasons_total,
                watched_seasons: crate::models::parse_watched_seasons(&watched_seasons, seasons_total),
                quotes: crate::models::parse_quotes(&quotes_str),
                parent_id: row.get("parent_id")?,
                media_type,
                platform: row.get("platform").unwrap_or(None),
                launcher: row.get("launcher").unwrap_or(None),
                playtime_hours: row.get("playtime_hours").unwrap_or(None),
                completion_status: row.get("completion_status").unwrap_or(None),
                sync_id: row.get("sync_id").unwrap_or_default(),
                updated_at: row.get("updated_at").unwrap_or_default(),
                deleted_at: row.get("deleted_at").unwrap_or(None),
            })
        })?;

        let mut films = Vec::new();
        for f in film_rows {
            films.push(f?);
        }

        // 2. Связи
        let mut rel_stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, kind, sync_id, updated_at, deleted_at
             FROM film_relations
             WHERE ?1 IS NULL OR updated_at > ?1
             ORDER BY updated_at ASC",
        )?;

        let rel_rows = rel_stmt.query_map([since_param], |row| {
            Ok(FilmRelation {
                id: row.get("id")?,
                source_id: row.get("source_id")?,
                target_id: row.get("target_id")?,
                kind: row.get("kind")?,
                sync_id: row.get("sync_id").unwrap_or_default(),
                updated_at: row.get("updated_at").unwrap_or_default(),
                deleted_at: row.get("deleted_at").unwrap_or(None),
            })
        })?;

        let mut relations = Vec::new();
        for r in rel_rows {
            relations.push(r?);
        }

        // 3. Скриншоты
        let mut screen_stmt = self.conn.prepare(
            "SELECT id, film_id, image_path, caption, position, sync_id, updated_at, deleted_at
             FROM film_screenshots
             WHERE ?1 IS NULL OR updated_at > ?1
             ORDER BY updated_at ASC",
        )?;

        let screen_rows = screen_stmt.query_map([since_param], |row| {
            Ok(FilmScreenshot {
                id: row.get(0)?,
                film_id: row.get(1)?,
                image_path: row.get(2)?,
                caption: row.get(3)?,
                position: row.get(4)?,
                sync_id: row.get("sync_id").unwrap_or_default(),
                updated_at: row.get("updated_at").unwrap_or_default(),
                deleted_at: row.get("deleted_at").unwrap_or(None),
            })
        })?;

        let mut screenshots = Vec::new();
        for s in screen_rows {
            screenshots.push(s?);
        }

        Ok(SyncPayload {
            client_id: client_id.to_owned(),
            since: since.map(str::to_owned),
            films,
            relations,
            screenshots,
        })
    }

    /// Применяет входящие изменения от другого устройства, разрешая конфликты по принципу LWW.
    pub fn apply_sync_delta(&mut self, payload: &SyncPayload) -> AppResult<SyncSummary> {
        let tx = self.conn.transaction()?;

        let mut applied_films = 0;
        let mut applied_relations = 0;
        let mut applied_screenshots = 0;

        // 1. Применяем фильмы
        for film in &payload.films {
            if film.sync_id.trim().is_empty() {
                continue;
            }

            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, updated_at FROM films WHERE sync_id = ?1",
                    [&film.sync_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            match existing {
                Some((local_id, local_updated_at)) => {
                    // Конфликт LWW: перезаписываем только если удалённая запись новее
                    if film.updated_at >= local_updated_at {
                        let quotes_json = serde_json::to_string(&film.quotes).unwrap_or_else(|_| "[]".to_owned());
                        let watched_seasons_str = film
                            .watched_seasons
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join(",");

                        tx.execute(
                            "UPDATE films SET
                                title = ?2,
                                original_title = ?3,
                                category = ?4,
                                director = ?5,
                                year = ?6,
                                genres = ?7,
                                country = ?8,
                                runtime_minutes = ?9,
                                imdb_id = ?10,
                                tags = ?11,
                                status = ?12,
                                rating = ?13,
                                description = ?14,
                                thoughts = ?15,
                                poster_path = ?16,
                                seasons_total = ?17,
                                watched_seasons = ?18,
                                quotes = ?19,
                                parent_id = ?20,
                                media_type = ?21,
                                platform = ?22,
                                launcher = ?23,
                                playtime_hours = ?24,
                                completion_status = ?25,
                                updated_at = ?26,
                                deleted_at = ?27
                             WHERE id = ?1",
                            params![
                                local_id,
                                film.title,
                                film.original_title,
                                film.category,
                                film.director,
                                film.year,
                                film.genres,
                                film.country,
                                film.runtime_minutes,
                                film.imdb_id,
                                film.tags,
                                film.status.as_db_str(),
                                film.rating,
                                film.description,
                                film.thoughts,
                                film.poster_path,
                                film.seasons_total,
                                watched_seasons_str,
                                quotes_json,
                                film.parent_id,
                                film.media_type.as_db_str(),
                                film.platform,
                                film.launcher,
                                film.playtime_hours,
                                film.completion_status,
                                film.updated_at,
                                film.deleted_at,
                            ],
                        )?;
                        applied_films += 1;
                    }
                }
                None => {
                    // Новая запись с другого устройства
                    let quotes_json = serde_json::to_string(&film.quotes).unwrap_or_else(|_| "[]".to_owned());
                    let watched_seasons_str = film
                        .watched_seasons
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(",");

                    tx.execute(
                        "INSERT INTO films (
                            title, original_title, category, director, year, genres, country,
                            runtime_minutes, imdb_id, tags, status, rating, description, thoughts,
                            poster_path, seasons_total, watched_seasons, quotes, parent_id,
                            media_type, platform, launcher, playtime_hours, completion_status,
                            sync_id, updated_at, deleted_at, sort_order
                        ) VALUES (
                            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27,
                            (SELECT COALESCE(MIN(sort_order), 0) - 1 FROM films)
                        )",
                        params![
                            film.title,
                            film.original_title,
                            film.category,
                            film.director,
                            film.year,
                            film.genres,
                            film.country,
                            film.runtime_minutes,
                            film.imdb_id,
                            film.tags,
                            film.status.as_db_str(),
                            film.rating,
                            film.description,
                            film.thoughts,
                            film.poster_path,
                            film.seasons_total,
                            watched_seasons_str,
                            quotes_json,
                            film.parent_id,
                            film.media_type.as_db_str(),
                            film.platform,
                            film.launcher,
                            film.playtime_hours,
                            film.completion_status,
                            film.sync_id,
                            film.updated_at,
                            film.deleted_at,
                        ],
                    )?;
                    applied_films += 1;
                }
            }
        }

        // 2. Применяем связи между тайтлами
        for rel in &payload.relations {
            if rel.sync_id.trim().is_empty() {
                continue;
            }

            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, updated_at FROM film_relations WHERE sync_id = ?1",
                    [&rel.sync_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            match existing {
                Some((local_id, local_updated_at)) => {
                    if rel.updated_at >= local_updated_at {
                        tx.execute(
                            "UPDATE film_relations SET
                                source_id = ?2,
                                target_id = ?3,
                                kind = ?4,
                                updated_at = ?5,
                                deleted_at = ?6
                             WHERE id = ?1",
                            params![
                                local_id,
                                rel.source_id,
                                rel.target_id,
                                rel.kind.as_db_str(),
                                rel.updated_at,
                                rel.deleted_at,
                            ],
                        )?;
                        applied_relations += 1;
                    }
                }
                None => {
                    let _ = tx.execute(
                        "INSERT OR IGNORE INTO film_relations (
                            source_id, target_id, kind, sync_id, updated_at, deleted_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            rel.source_id,
                            rel.target_id,
                            rel.kind.as_db_str(),
                            rel.sync_id,
                            rel.updated_at,
                            rel.deleted_at,
                        ],
                    );
                    applied_relations += 1;
                }
            }
        }

        // 3. Применяем скриншоты
        for screen in &payload.screenshots {
            if screen.sync_id.trim().is_empty() {
                continue;
            }

            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, updated_at FROM film_screenshots WHERE sync_id = ?1",
                    [&screen.sync_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            match existing {
                Some((local_id, local_updated_at)) => {
                    if screen.updated_at >= local_updated_at {
                        tx.execute(
                            "UPDATE film_screenshots SET
                                film_id = ?2,
                                image_path = ?3,
                                caption = ?4,
                                position = ?5,
                                updated_at = ?6,
                                deleted_at = ?7
                             WHERE id = ?1",
                            params![
                                local_id,
                                screen.film_id,
                                screen.image_path,
                                screen.caption,
                                screen.position,
                                screen.updated_at,
                                screen.deleted_at,
                            ],
                        )?;
                        applied_screenshots += 1;
                    }
                }
                None => {
                    let _ = tx.execute(
                        "INSERT INTO film_screenshots (
                            film_id, image_path, caption, position, sync_id, updated_at, deleted_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            screen.film_id,
                            screen.image_path,
                            screen.caption,
                            screen.position,
                            screen.sync_id,
                            screen.updated_at,
                            screen.deleted_at,
                        ],
                    );
                    applied_screenshots += 1;
                }
            }
        }

        let now: String = tx.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |r| r.get(0))?;

        tx.commit()?;

        Ok(SyncSummary {
            applied_films,
            applied_relations,
            applied_screenshots,
            server_time: now,
        })
    }
}

/// Создаёт сжатую миниатюру изображения для мобильных устройств (экономия трафика и памяти).
pub fn create_thumbnail(src: &Path, dst: &Path, max_width: u32) -> AppResult<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    let img = image::open(src).map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    let thumb = img.thumbnail(max_width, max_width * 2);
    thumb.save(dst).map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    Ok(())
}
