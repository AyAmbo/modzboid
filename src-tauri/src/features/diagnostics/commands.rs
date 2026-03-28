use std::path::Path;

use tauri::State;

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::features::discovery::cache;
use crate::features::profiles::storage;

use super::analyzer::{self, CrashReport};
use super::bisect::{self, BisectSession};
use super::preflight::{self, PreflightResult};

#[tauri::command]
pub async fn analyze_crash_log_cmd(
    state: State<'_, AppState>,
) -> Result<CrashReport, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config
        .zomboid_user_dir
        .as_ref()
        .ok_or_else(|| AppError::Validation("Zomboid user directory not configured".into()))?;

    // Get enabled mod IDs from cached mods (best effort, empty is fine)
    let all_mods = cache::get_cached_mods(&state.db).await.unwrap_or_default();
    let mod_ids: Vec<String> = all_mods.iter().map(|m| m.id.clone()).collect();

    analyzer::analyze_crash_log(Path::new(zomboid_dir), &mod_ids)
}

#[tauri::command]
pub async fn preflight_check_cmd(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<PreflightResult, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let profile = storage::get_profile(&profiles_dir, &profile_id)?;

    let all_mods = cache::get_cached_mods(&state.db).await?;
    let incompat_db = state.incompat_db.read().await;
    let config = state.config.read().await;
    let game_version = config.game_version.clone();
    drop(config);

    Ok(preflight::run_preflight(
        &profile.load_order,
        &all_mods,
        &incompat_db,
        game_version.as_deref(),
    ))
}

#[tauri::command]
pub async fn bisect_start_cmd(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<BisectSession, AppError> {
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let profile = storage::get_profile(&profiles_dir, &profile_id)?;

    if profile.load_order.is_empty() {
        return Err(AppError::Validation(
            "Profile has no mods in its load order".into(),
        ));
    }

    Ok(bisect::start_bisect(profile.load_order))
}

#[tauri::command]
pub async fn bisect_report_cmd(
    session: BisectSession,
    crashed: bool,
) -> Result<BisectSession, AppError> {
    if session.status != "testing" {
        return Err(AppError::Validation(
            "Bisect session is not in testing state".into(),
        ));
    }
    Ok(bisect::report_bisect(&session, crashed))
}
