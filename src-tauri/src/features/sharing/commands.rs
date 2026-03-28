use std::path::PathBuf;
use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::app_core::types::Profile;
use crate::features::discovery::cache;
use crate::features::profiles::storage;
use super::formats;
use super::sync;
use super::server_ini;

/// Export a profile's mod list in the requested format ("json", "csv", or "text").
#[tauri::command]
pub async fn export_mod_list_cmd(
    profile_id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let profile = storage::get_profile(&profiles_dir, &profile_id)?;

    let config = state.config.read().await;
    let game_version = config.game_version.clone();
    drop(config);

    let all_mods = cache::get_cached_mods(&state.db).await?;

    // Filter to mods in this profile's load order, preserving order
    let profile_mods: Vec<_> = profile
        .load_order
        .iter()
        .filter_map(|mod_id| all_mods.iter().find(|m| &m.id == mod_id))
        .cloned()
        .collect();

    match format.as_str() {
        "json" => formats::export_as_json(
            &profile.name,
            game_version.as_deref(),
            &profile_mods,
        ),
        "csv" => Ok(formats::export_as_csv(&profile_mods)),
        "text" => Ok(formats::export_as_text(&profile.name, &profile_mods)),
        _ => Err(AppError::Validation(format!(
            "Unsupported export format: {}. Use 'json', 'csv', or 'text'.",
            format
        ))),
    }
}

/// Parse an imported mod list string and return a preview of found/missing mods.
#[tauri::command]
pub async fn parse_mod_list_import_cmd(
    content: String,
    state: State<'_, AppState>,
) -> Result<formats::ImportPreview, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let known_ids: Vec<String> = all_mods.iter().map(|m| m.id.clone()).collect();
    formats::parse_import(&content, &known_ids)
}

/// Apply an imported mod list by creating a new profile with the given mod IDs.
#[tauri::command]
pub async fn apply_mod_list_import_cmd(
    profile_name: String,
    mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;

    // Filter to only installed mods
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let known_ids: Vec<String> = all_mods.iter().map(|m| m.id.clone()).collect();
    let filtered_load_order: Vec<String> = mod_ids
        .into_iter()
        .filter(|id| known_ids.contains(id))
        .collect();

    let mut profile = storage::create_profile(
        &profiles_dir,
        &profile_name,
        crate::app_core::types::ProfileType::Singleplayer,
    )?;
    profile.load_order = filtered_load_order;
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    storage::save_profile(&profiles_dir, &profile)?;
    Ok(profile)
}

/// Load mod IDs from a server.ini file's Mods= line.
/// Returns the list of mod IDs found in the file.
#[tauri::command]
pub async fn load_mods_from_server_ini_cmd(
    file_path: PathBuf,
) -> Result<Vec<String>, AppError> {
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| AppError::Io(format!("Cannot read server.ini at {:?}: {}", file_path, e)))?;

    let data = server_ini::parse_server_ini(&content)?;
    Ok(data.mod_ids)
}

/// Write a profile's load order back to a server.ini file's Mods= and WorkshopItems= lines.
#[tauri::command]
pub async fn save_mods_to_server_ini_cmd(
    file_path: PathBuf,
    load_order: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let sync_result = sync::fix_mod_workshop_sync(&load_order, &all_mods);

    // Build Mods= and WorkshopItems= values
    let mods_value = sync_result.mod_ids.join(";");
    let workshop_value = sync_result.workshop_ids
        .iter()
        .filter(|id| !id.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(";");

    // Use the server config save mechanism to update these two fields
    use crate::features::server_config::parser;
    let updates = vec![
        parser::ServerSettingUpdate {
            key: "Mods".to_string(),
            value: mods_value,
        },
        parser::ServerSettingUpdate {
            key: "WorkshopItems".to_string(),
            value: workshop_value,
        },
    ];
    parser::save_server_ini(&file_path, &updates)?;
    Ok(())
}
