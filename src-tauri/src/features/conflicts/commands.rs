use tauri::State;

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::app_core::types::ModConflict;
use crate::features::discovery::cache;

use super::detector;

#[tauri::command]
pub async fn detect_conflicts_cmd(
    mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ModConflict>, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let config = state.config.read().await;
    let game_version = config.game_version.as_deref();
    let incompat_db = state.incompat_db.read().await;

    detector::detect_conflicts(
        &state.db,
        &mod_ids,
        &all_mods,
        game_version,
        &incompat_db,
    )
    .await
}
