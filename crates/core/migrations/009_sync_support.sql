-- Миграция 009: поддержка двусторонней синхронизации (Offline-First / Tombstones).

-- Уникальный идентификатор записи для синхронизации между устройствами
ALTER TABLE films ADD COLUMN sync_id TEXT NOT NULL DEFAULT '';
-- Метка мягкого удаления (soft delete / tombstone)
ALTER TABLE films ADD COLUMN deleted_at TEXT;

UPDATE films SET sync_id = lower(hex(randomblob(16))) WHERE sync_id = '';

CREATE INDEX IF NOT EXISTS idx_films_sync_id ON films (sync_id);
CREATE INDEX IF NOT EXISTS idx_films_updated_at ON films (updated_at);
CREATE INDEX IF NOT EXISTS idx_films_deleted_at ON films (deleted_at);

-- Поддержка синхронизации связей
ALTER TABLE film_relations ADD COLUMN sync_id TEXT NOT NULL DEFAULT '';
ALTER TABLE film_relations ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
ALTER TABLE film_relations ADD COLUMN deleted_at TEXT;

UPDATE film_relations SET sync_id = lower(hex(randomblob(16))) WHERE sync_id = '';
UPDATE film_relations SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE updated_at = '';

CREATE INDEX IF NOT EXISTS idx_film_relations_sync_id ON film_relations (sync_id);
CREATE INDEX IF NOT EXISTS idx_film_relations_updated_at ON film_relations (updated_at);

-- Поддержка синхронизации скриншотов
ALTER TABLE film_screenshots ADD COLUMN sync_id TEXT NOT NULL DEFAULT '';
ALTER TABLE film_screenshots ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
ALTER TABLE film_screenshots ADD COLUMN deleted_at TEXT;

UPDATE film_screenshots SET sync_id = lower(hex(randomblob(16))) WHERE sync_id = '';
UPDATE film_screenshots SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE updated_at = '';

CREATE INDEX IF NOT EXISTS idx_film_screenshots_sync_id ON film_screenshots (sync_id);
CREATE INDEX IF NOT EXISTS idx_film_screenshots_updated_at ON film_screenshots (updated_at);
