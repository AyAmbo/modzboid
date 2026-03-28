use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::types::{Profile, ProfileType};
use crate::app_core::error::AppError;
use super::storage;

#[tauri::command]
pub async fn list_profiles_cmd(state: State<'_, AppState>) -> Result<Vec<Profile>, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    storage::list_profiles(&profiles_dir)
}

#[tauri::command]
pub async fn get_profile_cmd(profile_id: String, state: State<'_, AppState>) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    storage::get_profile(&profiles_dir, &profile_id)
}

#[tauri::command]
pub async fn create_profile_cmd(
    name: String,
    profile_type: ProfileType,
    server_config_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let mut profile = storage::create_profile(&profiles_dir, &name, profile_type)?;
    if let Some(path) = server_config_path {
        profile.server_config_path = Some(std::path::PathBuf::from(path));
        profile.updated_at = chrono::Utc::now().to_rfc3339();
        storage::save_profile(&profiles_dir, &profile)?;
    }
    Ok(profile)
}

#[tauri::command]
pub async fn update_profile_cmd(profile: Profile, state: State<'_, AppState>) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let mut profile = profile;
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    storage::save_profile(&profiles_dir, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn delete_profile_cmd(profile_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    storage::delete_profile(&profiles_dir, &profile_id)
}

#[tauri::command]
pub async fn duplicate_profile_cmd(profile_id: String, new_name: String, state: State<'_, AppState>) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    storage::duplicate_profile(&profiles_dir, &profile_id, &new_name)
}

#[tauri::command]
pub async fn export_profile_cmd(profile_id: String, state: State<'_, AppState>) -> Result<String, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let profile = storage::get_profile(&profiles_dir, &profile_id)?;
    Ok(serde_json::to_string_pretty(&profile)?)
}

#[tauri::command]
pub async fn import_profile_cmd(json: String, state: State<'_, AppState>) -> Result<Profile, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let mut profile: Profile = serde_json::from_str(&json)?;
    // Assign new ID to avoid collisions
    profile.id = uuid::Uuid::new_v4().to_string();
    profile.is_default = false;
    profile.created_at = chrono::Utc::now().to_rfc3339();
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    storage::save_profile(&profiles_dir, &profile)?;
    Ok(profile)
}
