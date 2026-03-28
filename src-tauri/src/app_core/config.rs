use std::path::{Path, PathBuf};
use crate::app_core::error::AppError;
use crate::app_core::types::{AppConfig, IncompatDb};
use crate::features::discovery::watcher::WatcherHandle;
use sqlx::sqlite::SqlitePool;
use tauri::State;
use tokio::sync::RwLock;

pub struct AppState {
    pub db: SqlitePool,
    pub config: RwLock<AppConfig>,
    pub app_data_dir: PathBuf,
    pub watcher_handle: std::sync::Mutex<Option<WatcherHandle>>,
    pub incompat_db: RwLock<IncompatDb>,
}

pub fn default_config() -> AppConfig {
    AppConfig {
        game_path: None,
        steam_path: None,
        workshop_path: None,
        local_mods_path: None,
        zomboid_user_dir: None,
        game_version: None,
        is_first_run: true,
        theme: "dark".to_string(),
        locale: "en".to_string(),
        check_updates: true,
        ui_scale: 100,
        font_size: 14,
    }
}

pub fn load_config(app_data_dir: &Path) -> Result<AppConfig, AppError> {
    let config_path = app_data_dir.join("config.json");
    if !config_path.exists() {
        return Ok(default_config());
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config: AppConfig = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn save_config(app_data_dir: &Path, config: &AppConfig) -> Result<(), AppError> {
    let config_path = app_data_dir.join("config.json");
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;
    Ok(())
}

#[tauri::command]
pub async fn get_config_cmd(state: State<'_, AppState>) -> Result<AppConfig, AppError> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config_cmd(
    config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    save_config(&state.app_data_dir, &config)?;
    let mut current = state.config.write().await;
    *current = config;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let config = default_config();
        assert!(config.is_first_run);
        assert_eq!(config.theme, "dark");
        assert_eq!(config.locale, "en");
        assert!(config.check_updates);
        assert_eq!(config.ui_scale, 100);
        assert_eq!(config.font_size, 14);
        assert!(config.game_path.is_none());
        assert!(config.steam_path.is_none());
        assert!(config.workshop_path.is_none());
        assert!(config.local_mods_path.is_none());
        assert!(config.zomboid_user_dir.is_none());
        assert!(config.game_version.is_none());
    }

    #[test]
    fn load_config_returns_default_when_no_file() {
        let dir = std::env::temp_dir().join(format!("modzboid-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = load_config(&dir).unwrap();
        assert!(config.is_first_run);
        assert_eq!(config.theme, "dark");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("modzboid-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut config = default_config();
        config.game_path = Some(PathBuf::from("/games/pz"));
        config.is_first_run = false;
        config.theme = "light".to_string();
        config.ui_scale = 125;
        config.font_size = 16;

        save_config(&dir, &config).unwrap();
        let loaded = load_config(&dir).unwrap();

        assert_eq!(loaded.game_path, Some(PathBuf::from("/games/pz")));
        assert!(!loaded.is_first_run);
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.ui_scale, 125);
        assert_eq!(loaded.font_size, 16);
        assert_eq!(loaded.locale, "en");
        assert!(loaded.check_updates);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_config_returns_error_for_invalid_json() {
        let dir = std::env::temp_dir().join(format!("modzboid-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), "not valid json {{{").unwrap();

        let result = load_config(&dir);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
