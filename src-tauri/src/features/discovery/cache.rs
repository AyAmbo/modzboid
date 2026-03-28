use sqlx::sqlite::SqlitePool;

use crate::app_core::error::AppError;
use crate::app_core::types::{ModCategory, ModInfo, ModSource};

// ─── Serialisation helpers ─────────────────────────────────────────────────────

fn vec_to_json(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
}

fn json_to_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn source_to_str(source: &ModSource) -> &'static str {
    match source {
        ModSource::Workshop => "workshop",
        ModSource::Local => "local",
    }
}

fn str_to_source(s: &str) -> ModSource {
    match s {
        "workshop" => ModSource::Workshop,
        _ => ModSource::Local,
    }
}

fn category_to_str(cat: &ModCategory) -> &'static str {
    match cat {
        ModCategory::Framework => "framework",
        ModCategory::Map => "map",
        ModCategory::Content => "content",
        ModCategory::Overhaul => "overhaul",
    }
}

fn str_to_category(s: &str) -> Option<ModCategory> {
    match s {
        "framework" => Some(ModCategory::Framework),
        "map" => Some(ModCategory::Map),
        "content" => Some(ModCategory::Content),
        "overhaul" => Some(ModCategory::Overhaul),
        _ => None,
    }
}

// ─── CRUD ──────────────────────────────────────────────────────────────────────

/// Insert or replace all mods into the mods table.
pub async fn cache_mods(pool: &SqlitePool, mods: &[ModInfo]) -> Result<(), AppError> {
    let now = chrono::Utc::now().to_rfc3339();

    for m in mods {
        let authors_json = vec_to_json(&m.authors);
        let requires_json = vec_to_json(&m.requires);
        let version_folders_json = vec_to_json(&m.version_folders);
        let tile_def_json = vec_to_json(&m.tile_def);
        let source_str = source_to_str(&m.source);
        let detected_category_str = m.detected_category.as_ref().map(category_to_str);
        let source_path_str = m.source_path.to_string_lossy().into_owned();
        let mod_info_path_str = m.mod_info_path.to_string_lossy().into_owned();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO mods (
                id, raw_id, workshop_id, name, description, authors,
                url, mod_version, poster_path, icon_path,
                version_min, version_max, version_folders, active_version_folder,
                requires, pack, tile_def, category, source,
                source_path, mod_info_path, size_bytes, last_modified,
                detected_category, cached_at
            ) VALUES (
                ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?
            )
            "#,
        )
        .bind(&m.id)
        .bind(&m.raw_id)
        .bind(&m.workshop_id)
        .bind(&m.name)
        .bind(&m.description)
        .bind(&authors_json)
        .bind(&m.url)
        .bind(&m.mod_version)
        .bind(&m.poster_path)
        .bind(&m.icon_path)
        .bind(&m.version_min)
        .bind(&m.version_max)
        .bind(&version_folders_json)
        .bind(&m.active_version_folder)
        .bind(&requires_json)
        .bind(&m.pack)
        .bind(&tile_def_json)
        .bind(&m.category)
        .bind(source_str)
        .bind(&source_path_str)
        .bind(&mod_info_path_str)
        .bind(m.size_bytes.map(|s| s as i64))
        .bind(&m.last_modified)
        .bind(detected_category_str)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to cache mod '{}': {}", m.id, e)))?;
    }

    Ok(())
}

/// Select all mods from the cache.
pub async fn get_cached_mods(pool: &SqlitePool) -> Result<Vec<ModInfo>, AppError> {
    let rows = sqlx::query_as::<_, ModRow>(
        r#"SELECT id, raw_id, workshop_id, name, description, authors,
                  url, mod_version, poster_path, icon_path,
                  version_min, version_max, version_folders, active_version_folder,
                  requires, pack, tile_def, category, source,
                  source_path, mod_info_path, size_bytes, last_modified,
                  detected_category
           FROM mods"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch cached mods: {}", e)))?;

    Ok(rows.into_iter().map(row_to_mod_info).collect())
}

/// Delete all mods from the cache.
pub async fn clear_cache(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("DELETE FROM mods")
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to clear mod cache: {}", e)))?;
    sqlx::query("DELETE FROM mod_files")
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to clear mod_files cache: {}", e)))?;
    sqlx::query("DELETE FROM script_ids")
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to clear script_ids cache: {}", e)))?;
    Ok(())
}

/// Retrieve a single mod by ID.
pub async fn get_mod_by_id(pool: &SqlitePool, id: &str) -> Result<Option<ModInfo>, AppError> {
    let row = sqlx::query_as::<_, ModRow>(
        r#"SELECT id, raw_id, workshop_id, name, description, authors,
                  url, mod_version, poster_path, icon_path,
                  version_min, version_max, version_folders, active_version_folder,
                  requires, pack, tile_def, category, source,
                  source_path, mod_info_path, size_bytes, last_modified,
                  detected_category
           FROM mods WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch mod '{}': {}", id, e)))?;

    Ok(row.map(row_to_mod_info))
}

// ─── Row type for sqlx ─────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ModRow {
    id: String,
    raw_id: String,
    workshop_id: Option<String>,
    name: String,
    description: String,
    authors: String,
    url: Option<String>,
    mod_version: Option<String>,
    poster_path: Option<String>,
    icon_path: Option<String>,
    version_min: Option<String>,
    version_max: Option<String>,
    version_folders: String,
    active_version_folder: Option<String>,
    requires: String,
    pack: Option<String>,
    tile_def: String,
    category: Option<String>,
    source: String,
    source_path: String,
    mod_info_path: String,
    size_bytes: Option<i64>,
    last_modified: String,
    detected_category: Option<String>,
}

fn row_to_mod_info(row: ModRow) -> ModInfo {
    ModInfo {
        id: row.id,
        raw_id: row.raw_id,
        workshop_id: row.workshop_id,
        name: row.name,
        description: row.description,
        authors: json_to_vec(&row.authors),
        url: row.url,
        mod_version: row.mod_version,
        poster_path: row.poster_path,
        icon_path: row.icon_path,
        version_min: row.version_min,
        version_max: row.version_max,
        version_folders: json_to_vec(&row.version_folders),
        active_version_folder: row.active_version_folder,
        requires: json_to_vec(&row.requires),
        pack: row.pack,
        tile_def: json_to_vec(&row.tile_def),
        category: row.category,
        source: str_to_source(&row.source),
        source_path: std::path::PathBuf::from(row.source_path),
        mod_info_path: std::path::PathBuf::from(row.mod_info_path),
        size_bytes: row.size_bytes.map(|s| s as u64),
        last_modified: row.last_modified,
        detected_category: row.detected_category.as_deref().and_then(str_to_category),
    }
}
