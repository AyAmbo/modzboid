use std::path::PathBuf;
use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::manager::{self, BackupInfo};

fn validate_backup_path(backup_path: &str, app_data_dir: &std::path::Path) -> Result<PathBuf, AppError> {
    let path = PathBuf::from(backup_path);
    let backups_dir = app_data_dir.join("backups");
    let canonical = path.canonicalize()
        .map_err(|_| AppError::NotFound(format!("Backup not found: {}", backup_path)))?;
    let canonical_base = backups_dir.canonicalize()
        .map_err(|_| AppError::NotFound("Backups directory not found".into()))?;
    if !canonical.starts_with(&canonical_base) {
        return Err(AppError::Validation("Backup path is outside the backups directory".into()));
    }
    Ok(canonical)
}

#[tauri::command]
pub async fn create_backup_cmd(
    name: String,
    state: State<'_, AppState>,
) -> Result<BackupInfo, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone();
    drop(config);
    manager::create_backup(&state.app_data_dir, zomboid_dir.as_deref(), &name)
}

#[tauri::command]
pub async fn list_backups_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<BackupInfo>, AppError> {
    Ok(manager::list_backups(&state.app_data_dir))
}

#[tauri::command]
pub async fn restore_backup_cmd(
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let validated = validate_backup_path(&backup_path, &state.app_data_dir)?;
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone();
    drop(config);
    manager::restore_backup(&validated, &state.app_data_dir, zomboid_dir.as_deref())
}

#[tauri::command]
pub async fn delete_backup_cmd(
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let validated = validate_backup_path(&backup_path, &state.app_data_dir)?;
    manager::delete_backup(&validated)
}
