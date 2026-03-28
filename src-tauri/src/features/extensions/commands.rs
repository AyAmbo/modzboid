use std::path::{Path, PathBuf};
use tauri::State;

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use super::loader::{self, ExtensionInfo, Replacement};

#[tauri::command]
pub async fn list_extensions_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ExtensionInfo>, AppError> {
    Ok(loader::list_extensions(&state.app_data_dir))
}

#[tauri::command]
pub async fn install_extension_cmd(
    source_path: String,
    state: State<'_, AppState>,
) -> Result<ExtensionInfo, AppError> {
    loader::install_extension(&state.app_data_dir, Path::new(&source_path))
}

#[tauri::command]
pub async fn toggle_extension_cmd(
    extension_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    loader::toggle_extension(&state.app_data_dir, &extension_id, enabled)
}

#[tauri::command]
pub async fn uninstall_extension_cmd(
    extension_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    loader::uninstall_extension(&state.app_data_dir, &extension_id)
}

#[tauri::command]
pub async fn get_replacements_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<Replacement>, AppError> {
    Ok(loader::get_all_replacements(&state.app_data_dir))
}

#[tauri::command]
pub async fn export_extension_cmd(
    extension_id: String,
    output_path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let ext_dir = loader::extensions_dir(&state.app_data_dir).join(&extension_id);
    if !ext_dir.exists() {
        return Err(AppError::NotFound(format!("Extension not found: {}", extension_id)));
    }

    let output = PathBuf::from(&output_path);
    let file = std::fs::File::create(&output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    for entry in walkdir::WalkDir::new(&ext_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(&ext_dir).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if path.is_file() {
            zip.start_file(&rel_str, options)?;
            let content = std::fs::read(path)?;
            use std::io::Write;
            zip.write_all(&content)?;
        }
    }

    zip.finish()?;
    Ok(())
}
