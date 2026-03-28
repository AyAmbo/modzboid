use std::path::PathBuf;
use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::parser::{self, SandboxVarsConfig, SandboxSettingUpdate};

fn validate_sandbox_path(path: &str, zomboid_dir: &std::path::Path) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);
    let canonical = p.canonicalize()
        .map_err(|_| AppError::NotFound(format!("Sandbox vars not found: {}", path)))?;
    let base = zomboid_dir.canonicalize()
        .map_err(|_| AppError::NotFound("Zomboid directory not found".into()))?;
    if !canonical.starts_with(&base) {
        return Err(AppError::Validation("Path is outside the Zomboid directory".into()));
    }
    Ok(canonical)
}

#[tauri::command]
pub async fn load_sandbox_vars_cmd(
    path: String,
    state: State<'_, AppState>,
) -> Result<SandboxVarsConfig, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);
    let validated = validate_sandbox_path(&path, &zomboid_dir)?;
    parser::parse_sandbox_vars(&validated)
}

#[tauri::command]
pub async fn save_sandbox_vars_cmd(
    path: String,
    updates: Vec<SandboxSettingUpdate>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);
    let validated = validate_sandbox_path(&path, &zomboid_dir)?;
    parser::save_sandbox_vars(&validated, &updates)
}
