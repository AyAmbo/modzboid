CREATE TABLE IF NOT EXISTS mods (
    row_id INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL,
    raw_id TEXT NOT NULL,
    workshop_id TEXT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    authors TEXT NOT NULL DEFAULT '[]',
    url TEXT,
    mod_version TEXT,
    poster_path TEXT,
    icon_path TEXT,
    version_min TEXT,
    version_max TEXT,
    version_folders TEXT NOT NULL DEFAULT '[]',
    active_version_folder TEXT,
    requires TEXT NOT NULL DEFAULT '[]',
    pack TEXT,
    tile_def TEXT NOT NULL DEFAULT '[]',
    category TEXT,
    source TEXT NOT NULL CHECK(source IN ('workshop', 'local')),
    source_path TEXT NOT NULL,
    mod_info_path TEXT NOT NULL,
    size_bytes INTEGER,
    last_modified TEXT NOT NULL,
    detected_category TEXT,
    cached_at TEXT NOT NULL,
    UNIQUE(id, source, source_path)
);

CREATE INDEX IF NOT EXISTS idx_mods_id ON mods(id);
CREATE INDEX IF NOT EXISTS idx_mods_workshop_id ON mods(workshop_id);
CREATE INDEX IF NOT EXISTS idx_mods_source ON mods(source);
