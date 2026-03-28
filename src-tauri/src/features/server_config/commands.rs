use std::path::PathBuf;
use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::parser::{self, ServerConfig, ServerConfigInfo, ServerSettingUpdate};

fn validate_server_path(path: &str, zomboid_dir: &std::path::Path) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);

    if !p.exists() {
        return Err(AppError::NotFound(format!("Config not found: {}", path)));
    }

    // Canonicalize both paths to resolve symlinks and normalize separators.
    // On Windows, canonicalize may add \\?\ prefix — strip it for comparison.
    let canonical = p.canonicalize()
        .map_err(|e| AppError::NotFound(format!("Cannot resolve {}: {}", path, e)))?;
    let base = zomboid_dir.canonicalize()
        .map_err(|e| AppError::NotFound(format!("Cannot resolve zomboid dir: {}", e)))?;

    // Normalize to strings for cross-platform comparison
    let canonical_str = canonical.to_string_lossy().replace('\\', "/");
    let base_str = base.to_string_lossy().replace('\\', "/");

    // Strip Windows \\?\ extended path prefix if present
    let canonical_clean = canonical_str.strip_prefix("//?/").unwrap_or(&canonical_str);
    let base_clean = base_str.strip_prefix("//?/").unwrap_or(&base_str);

    if !canonical_clean.to_lowercase().starts_with(&base_clean.to_lowercase()) {
        log::warn!(
            "Path validation failed: {:?} is not inside {:?}",
            canonical_clean, base_clean
        );
        return Err(AppError::Validation(format!(
            "Path is outside the Zomboid directory: {} not in {}",
            canonical_clean, base_clean
        )));
    }

    Ok(canonical)
}

#[tauri::command]
pub async fn list_server_configs_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ServerConfigInfo>, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);
    Ok(parser::list_server_configs(&zomboid_dir))
}

/// Check if a server config file exists at the given path.
#[tauri::command]
pub async fn validate_server_config_cmd(
    file_path: String,
) -> Result<bool, AppError> {
    Ok(std::path::Path::new(&file_path).exists())
}

#[tauri::command]
pub async fn load_server_config_cmd(
    path: String,
    state: State<'_, AppState>,
) -> Result<ServerConfig, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);
    let validated = validate_server_path(&path, &zomboid_dir)?;
    parser::parse_server_ini(&validated)
}

#[tauri::command]
pub async fn save_server_config_cmd(
    path: String,
    settings: Vec<ServerSettingUpdate>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);
    let validated = validate_server_path(&path, &zomboid_dir)?;
    parser::save_server_ini(&validated, &settings)
}
