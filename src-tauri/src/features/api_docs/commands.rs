//! Tauri commands for the in-app API documentation viewer.

use std::sync::OnceLock;
use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::search::*;

/// Lazily loaded API snapshot — loaded on first use, cached for the session.
static API_SNAPSHOT: OnceLock<Option<RawSnapshot>> = OnceLock::new();

fn get_or_load_snapshot(app_data_dir: &std::path::Path) -> Option<&'static RawSnapshot> {
    API_SNAPSHOT.get_or_init(|| {
        let path = app_data_dir.join("api-snapshot.json");
        if !path.exists() {
            // Try bundled snapshot in resources
            let bundled = app_data_dir.join("resources").join("api-snapshot.json");
            if bundled.exists() {
                return load_snapshot(&bundled).ok();
            }
            return None;
        }
        load_snapshot(&path).ok()
    }).as_ref()
}

/// Search the API for classes, methods, events matching a query.
#[tauri::command]
pub async fn search_api_cmd(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ApiSearchResult>, AppError> {
    let snapshot = get_or_load_snapshot(&state.app_data_dir)
        .ok_or_else(|| AppError::NotFound("API snapshot not loaded. Place api-snapshot.json in app data directory.".into()))?;

    Ok(search_api(snapshot, &query, limit.unwrap_or(50)))
}

/// Get full details for a specific class.
#[tauri::command]
pub async fn get_api_class_cmd(
    class_name: String,
    state: State<'_, AppState>,
) -> Result<ApiClassDetail, AppError> {
    let snapshot = get_or_load_snapshot(&state.app_data_dir)
        .ok_or_else(|| AppError::NotFound("API snapshot not loaded.".into()))?;

    get_class_detail(snapshot, &class_name)
        .ok_or_else(|| AppError::NotFound(format!("Class not found: {}", class_name)))
}

/// Get all events.
#[tauri::command]
pub async fn get_api_events_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ApiEventInfo>, AppError> {
    let snapshot = get_or_load_snapshot(&state.app_data_dir)
        .ok_or_else(|| AppError::NotFound("API snapshot not loaded.".into()))?;

    Ok(get_events(snapshot))
}

/// Get API stats (class counts, method counts, version).
#[tauri::command]
pub async fn get_api_stats_cmd(
    state: State<'_, AppState>,
) -> Result<ApiStats, AppError> {
    let snapshot = get_or_load_snapshot(&state.app_data_dir)
        .ok_or_else(|| AppError::NotFound("API snapshot not loaded.".into()))?;

    Ok(get_stats(snapshot))
}

/// List all class names (for browsing).
#[tauri::command]
pub async fn list_api_classes_cmd(
    kind: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let snapshot = get_or_load_snapshot(&state.app_data_dir)
        .ok_or_else(|| AppError::NotFound("API snapshot not loaded.".into()))?;

    let filter = kind.as_deref().unwrap_or("all");
    let mut names: Vec<String> = Vec::new();

    if filter == "all" || filter == "java" {
        names.extend(snapshot.java_classes.keys().cloned());
    }
    if filter == "all" || filter == "lua" {
        names.extend(snapshot.lua_classes.keys().cloned());
    }

    names.sort();
    Ok(names)
}
