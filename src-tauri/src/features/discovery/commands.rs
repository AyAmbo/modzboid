use tauri::State;

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::app_core::types::ModInfo;
use super::{cache, scanner};

/// Scan mod directories, cache the results, and return them.
/// Conflict detection file trees are cached in the background to avoid blocking the UI.
#[tauri::command]
pub async fn discover_mods(state: State<'_, AppState>) -> Result<Vec<ModInfo>, AppError> {
    let config = state.config.read().await;
    let mods = scanner::scan_mod_directories(
        config.workshop_path.as_deref(),
        config.local_mods_path.as_deref(),
        config.game_path.as_deref(),
        config.game_version.as_deref().unwrap_or("42.15"),
    )?;
    drop(config);
    cache::cache_mods(&state.db, &mods).await?;

    // Cache file trees and script IDs for conflict detection in the background
    let db = state.db.clone();
    let mods_clone: Vec<ModInfo> = mods.clone();
    tokio::spawn(async move {
        for m in &mods_clone {
            // Use version-aware media paths: version folder + common, or fallback to root
            let mut all_entries = Vec::new();
            if let Some(ref vf) = m.active_version_folder {
                let ver_media = m.source_path.join(vf).join("media");
                if ver_media.exists() {
                    all_entries.extend(crate::features::conflicts::scanner::collect_mod_files(&ver_media));
                }
            }
            let common_media = m.source_path.join("common").join("media");
            if common_media.exists() {
                all_entries.extend(crate::features::conflicts::scanner::collect_mod_files(&common_media));
            }
            if all_entries.is_empty() {
                let root_media = m.source_path.join("media");
                all_entries.extend(crate::features::conflicts::scanner::collect_mod_files(&root_media));
            }
            let entries = all_entries;
            if let Err(e) = crate::features::conflicts::scanner::cache_mod_files(&db, &m.id, &entries).await {
                log::warn!("Failed to cache file tree for {}: {}", m.id, e);
                continue;
            }

            let mut all_lua_globals = Vec::new();
            let mut all_event_hooks = Vec::new();

            // Resolve file paths: try version folder, then common, then root
            let resolve_path = |rel: &str| -> Option<std::path::PathBuf> {
                if let Some(ref vf) = m.active_version_folder {
                    let p = m.source_path.join(vf).join("media").join(rel);
                    if p.exists() { return Some(p); }
                }
                let p = m.source_path.join("common").join("media").join(rel);
                if p.exists() { return Some(p); }
                let p = m.source_path.join("media").join(rel);
                if p.exists() { return Some(p); }
                None
            };

            for entry in &entries {
                if entry.file_type == "script" {
                    if let Some(script_path) = resolve_path(&entry.relative_path) {
                        if let Ok(content) = std::fs::read_to_string(&script_path) {
                            let script_ids = crate::features::conflicts::scanner::extract_script_ids(&content, &entry.relative_path);
                            if let Err(e) = crate::features::conflicts::scanner::cache_script_ids(&db, &m.id, &script_ids).await {
                                log::warn!("Failed to cache script IDs for {}: {}", m.id, e);
                            }
                        }
                    }
                }

                // Extract Lua globals and event hooks from .lua files
                if entry.file_type == "lua" {
                    if let Some(lua_path) = resolve_path(&entry.relative_path) {
                    if let Ok(content) = std::fs::read_to_string(&lua_path) {
                        let (globals, hooks) = crate::features::conflicts::scanner::extract_lua_globals_and_hooks(&content, &entry.relative_path);
                        all_lua_globals.extend(globals);
                        all_event_hooks.extend(hooks);
                    }
                    }
                }
            }

            // Cache Lua analysis results
            if let Err(e) = crate::features::conflicts::scanner::cache_lua_globals(&db, &m.id, &all_lua_globals).await {
                log::warn!("Failed to cache Lua globals for {}: {}", m.id, e);
            }
            if let Err(e) = crate::features::conflicts::scanner::cache_event_hooks(&db, &m.id, &all_event_hooks).await {
                log::warn!("Failed to cache event hooks for {}: {}", m.id, e);
            }
        }
        log::info!("Background conflict scanning complete for {} mods", mods_clone.len());
    });

    Ok(mods)
}

/// Look up a single mod by ID from the cache.
#[tauri::command]
pub async fn get_mod_details(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<ModInfo, AppError> {
    cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))
}

/// Rescan a single mod with a specific version folder override.
/// Returns the updated ModInfo with metadata from the selected version.
#[tauri::command]
pub async fn rescan_mod_version(
    mod_id: String,
    version_folder: Option<String>,
    state: State<'_, AppState>,
) -> Result<ModInfo, AppError> {
    // Look up the mod from cache to get its source_path and source type
    let existing = cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))?;

    let config = state.config.read().await;
    let game_version = config.game_version.as_deref().unwrap_or("42.15");

    let updated = scanner::rescan_mod_with_version(
        &existing.source_path,
        existing.source,
        existing.workshop_id.clone(),
        version_folder.as_deref(),
        game_version,
    )?
    .ok_or_else(|| AppError::NotFound("Could not parse mod with selected version".into()))?;

    Ok(updated)
}

/// Clear the mod cache, rescan, and return fresh results.
#[tauri::command]
pub async fn refresh_mods(state: State<'_, AppState>) -> Result<Vec<ModInfo>, AppError> {
    cache::clear_cache(&state.db).await?;
    discover_mods(state).await
}
