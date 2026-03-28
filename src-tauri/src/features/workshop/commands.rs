use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::acf_parser::{self, WorkshopItemInfo};
use super::steam_api::{self, WorkshopItemMeta};

/// Get workshop item info from Steam's manifest file.
#[tauri::command]
pub async fn get_workshop_items_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<WorkshopItemInfo>, AppError> {
    let config = state.config.read().await;

    // Try to find ACF from steam_path or derive from workshop_path
    let acf_path = if let Some(ref steam_path) = config.steam_path {
        acf_parser::find_acf_path(steam_path)
    } else if let Some(ref workshop_path) = config.workshop_path {
        // workshop_path is typically .../steamapps/workshop/content/108600
        // ACF is at .../steamapps/workshop/appworkshop_108600.acf
        let workshop_root = workshop_path.parent().and_then(|p| p.parent());
        workshop_root.and_then(|p| {
            let acf = p.join("appworkshop_108600.acf");
            if acf.exists() { Some(acf) } else { None }
        })
    } else {
        None
    };
    drop(config);

    match acf_path {
        Some(path) => acf_parser::parse_workshop_acf(&path),
        None => Ok(vec![]),
    }
}

/// Open a Workshop page in the default browser.
#[tauri::command]
pub async fn open_workshop_page_cmd(workshop_id: String) -> Result<(), AppError> {
    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", workshop_id);
    open::that(&url).map_err(|e| AppError::Io(format!("Failed to open browser: {}", e)))?;
    Ok(())
}

/// Fetch Steam Workshop metadata for a list of mod IDs.
/// Uses the Steam Web API (no API key required for public data).
#[tauri::command]
pub async fn fetch_workshop_meta_cmd(
    workshop_ids: Vec<String>,
) -> Result<Vec<WorkshopItemMeta>, AppError> {
    if workshop_ids.is_empty() {
        return Ok(vec![]);
    }
    steam_api::get_published_file_details(&workshop_ids)
        .await
        .map_err(|e| AppError::Io(e))
}

/// Fetch Steam Workshop metadata for a single mod.
#[tauri::command]
pub async fn fetch_single_workshop_meta_cmd(
    workshop_id: String,
) -> Result<Option<WorkshopItemMeta>, AppError> {
    let results = steam_api::get_published_file_details(&[workshop_id])
        .await
        .map_err(|e| AppError::Io(e))?;
    Ok(results.into_iter().find(|r| r.found))
}

/// Search the Steam Workshop for PZ mods.
#[tauri::command]
pub async fn search_workshop_cmd(
    query: String,
    page: Option<u32>,
) -> Result<Vec<WorkshopItemMeta>, AppError> {
    steam_api::search_workshop(&query, page.unwrap_or(1), 20)
        .await
        .map_err(|e| AppError::Io(e))
}
