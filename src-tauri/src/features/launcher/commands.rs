use std::path::{Path, PathBuf};
use tauri::{Emitter, State};

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::app_core::types::{Profile, ProfileType};
use crate::features::profiles::storage;
use crate::features::discovery::cache;

// ─── Path Detection ─────────────────────────────────────────────────────────

/// Auto-detect the Project Zomboid install location.
/// Returns the first valid path found, or None.
#[tauri::command]
pub async fn detect_game_path_cmd() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Try registry first
        if let Some(path) = windows_registry_game_path() {
            if verify_game_path_sync(&path) {
                return Some(path);
            }
        }
        // Fall back to common paths
        let candidates = [
            r"C:\Program Files (x86)\Steam\steamapps\common\ProjectZomboid\",
            r"C:\Program Files\Steam\steamapps\common\ProjectZomboid\",
        ];
        for candidate in candidates {
            let p = PathBuf::from(candidate);
            if verify_game_path_sync(&p) {
                return Some(p);
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs_home();
        let candidates: Vec<PathBuf> = home
            .into_iter()
            .flat_map(|h| {
                vec![
                    h.join(".steam/steam/steamapps/common/ProjectZomboid"),
                    h.join(".local/share/Steam/steamapps/common/ProjectZomboid"),
                ]
            })
            .collect();

        for candidate in candidates {
            if verify_game_path_sync(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

/// Auto-detect the Steam install location.
/// Returns the first valid path found, or None.
#[tauri::command]
pub async fn detect_steam_path_cmd() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = windows_registry_steam_path() {
            if verify_steam_path_sync(&path) {
                return Some(path);
            }
        }
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam\");
        if verify_steam_path_sync(&p) {
            return Some(p);
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs_home();
        let candidates: Vec<PathBuf> = home
            .into_iter()
            .flat_map(|h| {
                vec![
                    h.join(".steam/steam"),
                    h.join(".local/share/Steam"),
                ]
            })
            .collect();

        for candidate in candidates {
            if verify_steam_path_sync(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

// ─── Path Verification ───────────────────────────────────────────────────────

/// Verify a given path is a valid PZ install directory.
#[tauri::command]
pub async fn verify_game_path_cmd(path: String) -> bool {
    verify_game_path_sync(Path::new(&path))
}

/// Verify a given path is a valid Steam install directory.
#[tauri::command]
pub async fn verify_steam_path_cmd(path: String) -> bool {
    verify_steam_path_sync(Path::new(&path))
}

/// Synchronous helper — checks for game exe and media/ directory.
pub fn verify_game_path_sync(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    #[cfg(target_os = "windows")]
    let has_exe = path.join("ProjectZomboid64.exe").exists();

    #[cfg(not(target_os = "windows"))]
    let has_exe = path.join("projectzomboid").exists()
        || path.join("ProjectZomboid64").exists()
        || path.join("ProjectZomboid64.exe").exists(); // WSL / Wine support

    has_exe && path.join("media").exists()
}

/// Synchronous helper — accepts if the directory contains any Steam indicator.
/// Case-insensitive check on Windows for steam.exe, steamapps/, etc.
pub fn verify_steam_path_sync(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Check common Steam indicators
    let indicators: &[&str] = &[
        "steam.exe",
        "Steam.exe",
        "steamapps",
        "SteamApps",
    ];

    for indicator in indicators {
        if path.join(indicator).exists() {
            return true;
        }
    }

    // On case-insensitive filesystems (Windows), the above may already match.
    // Also try the full workshop path as a positive signal.
    path.join("steamapps/workshop/content/108600").exists()
}

// ─── Version Detection ───────────────────────────────────────────────────────

/// Read SVNRevision.txt and return its content.
/// If `path` is provided, use it directly; otherwise fall back to the configured game_path.
#[tauri::command]
pub async fn detect_game_version_cmd(path: Option<String>, state: State<'_, AppState>) -> Result<Option<String>, AppError> {
    let game_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        let config = state.config.read().await;
        match &config.game_path {
            Some(p) => p.clone(),
            None => return Ok(None),
        }
    };

    let revision_path = game_path.join("SVNRevision.txt");
    if !revision_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&revision_path)?;
    let version = content.trim().to_string();
    if version.is_empty() {
        Ok(None)
    } else {
        Ok(Some(version))
    }
}

// ─── default.txt helpers ─────────────────────────────────────────────────────

/// Check if a mod ID contains only safe characters for default.txt
fn is_safe_mod_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Check if a map entry is safe (map names can contain spaces/commas — just no newlines or braces)
fn is_safe_map_entry(entry: &str) -> bool {
    !entry.is_empty() && !entry.chars().any(|c| c == '\n' || c == '\r' || c == '{' || c == '}')
}

/// Generate the content of PZ's default.txt.
pub fn format_default_txt(load_order: &[String], map_entries: &[String]) -> String {
    let mut out = String::new();
    out.push_str("VERSION = 1,\n\n");

    out.push_str("mods\n{\n");
    for mod_id in load_order {
        if !is_safe_mod_id(mod_id) { continue; }
        out.push_str(&format!("    mod {} {{}}\n", mod_id));
    }
    out.push_str("}\n\n");

    out.push_str("maps\n{\n");
    for map in map_entries {
        if !is_safe_map_entry(map) { continue; }
        out.push_str(&format!("    map {} {{}}\n", map));
    }
    out.push_str("}\n");

    out
}

/// Write a formatted default.txt to <zomboid_mods_dir>/default.txt.
pub fn write_default_txt(
    zomboid_mods_dir: &Path,
    load_order: &[String],
    map_entries: &[String],
) -> Result<(), AppError> {
    std::fs::create_dir_all(zomboid_mods_dir)?;
    let content = format_default_txt(load_order, map_entries);
    let dest = zomboid_mods_dir.join("default.txt");
    std::fs::write(&dest, content)?;
    Ok(())
}

/// Parse an existing default.txt and extract the list of mod IDs.
/// Matches lines of the form `    mod <ModId> {}`.
pub fn parse_default_txt(path: &Path) -> Result<Vec<String>, AppError> {
    let content = std::fs::read_to_string(path)?;
    let mut mods = Vec::new();
    for line in content.lines() {
        // Match exactly "    mod <id> {}"
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("mod ") {
            if let Some(id) = rest.strip_suffix(" {}") {
                mods.push(id.to_string());
            }
        }
    }
    Ok(mods)
}

// ─── Import from Game ────────────────────────────────────────────────────────

/// Read the active mod list from the game's default.txt and create a new profile.
#[tauri::command]
pub async fn import_from_game_cmd(state: State<'_, AppState>) -> Result<Profile, AppError> {
    let config = state.config.read().await;
    let zomboid_user_dir = config
        .zomboid_user_dir
        .clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);

    let default_txt_path = zomboid_user_dir.join("mods/default.txt");
    let load_order = if default_txt_path.exists() {
        parse_default_txt(&default_txt_path)?
    } else {
        vec![]
    };

    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let mut profile = storage::create_profile(&profiles_dir, "Imported from Game", ProfileType::Singleplayer)?;
    profile.load_order = load_order;
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    storage::save_profile(&profiles_dir, &profile)?;

    Ok(profile)
}

// ─── Launch Game ─────────────────────────────────────────────────────────────

/// Write default.txt for the given profile and launch the game executable.
#[tauri::command]
pub async fn launch_game_cmd(
    profile_id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    // Read profile
    let profiles_dir = storage::ensure_profiles_dir(&state.app_data_dir)?;
    let profile = storage::get_profile(&profiles_dir, &profile_id)?;

    // Read config
    let config = state.config.read().await;
    let game_path = config
        .game_path
        .clone()
        .ok_or_else(|| AppError::Validation("game_path is not configured".into()))?;
    let zomboid_user_dir = config
        .zomboid_user_dir
        .clone()
        .ok_or_else(|| AppError::Validation("zomboid_user_dir is not configured".into()))?;
    drop(config);

    // Determine map entries from cached mods
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let mut map_entries = vec!["Muldraugh, KY".to_string()];
    for m in &all_mods {
        if profile.load_order.contains(&m.id) || profile.load_order.contains(&m.raw_id) {
            // Add pack entry if present
            if let Some(ref pack) = m.pack {
                if !map_entries.contains(pack) {
                    map_entries.push(pack.clone());
                }
            }
            // Add tileDef entries as map directories
            for td in &m.tile_def {
                if !map_entries.contains(td) {
                    map_entries.push(td.clone());
                }
            }
        }
    }

    // Write default.txt
    let mods_dir = zomboid_user_dir.join("mods");
    write_default_txt(&mods_dir, &profile.load_order, &map_entries)?;

    // Launch the game executable
    #[cfg(target_os = "windows")]
    let exe = game_path.join("ProjectZomboid64.exe");

    #[cfg(not(target_os = "windows"))]
    let exe = {
        let linux_exe = game_path.join("ProjectZomboid64");
        if linux_exe.exists() {
            linux_exe
        } else {
            game_path.join("projectzomboid")
        }
    };

    if !exe.exists() {
        return Err(AppError::Game(format!(
            "Game executable not found at: {}",
            exe.display()
        )));
    }

    std::process::Command::new(&exe)
        .current_dir(&game_path)
        .spawn()
        .map_err(|e| AppError::Game(format!("Failed to launch game: {}", e)))?;

    // Emit event so the frontend knows the game launched
    app.emit("game-launched", &profile_id)
        .map_err(|e| AppError::Game(format!("Failed to emit event: {}", e)))?;

    Ok(())
}

// ─── Platform helpers ────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(target_os = "windows")]
fn windows_registry_game_path() -> Option<PathBuf> {
    // Registry lookup requires the `winreg` crate (not in deps).
    // Filesystem fallback in detect_game_path_cmd() is sufficient.
    None
}

#[cfg(target_os = "windows")]
fn windows_registry_steam_path() -> Option<PathBuf> {
    None
}

// ─── Open Folder ────────────────────────────────────────────────────────────

/// Open a folder in the system file manager.
#[tauri::command]
pub async fn open_folder_cmd(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(AppError::Io(format!("Path does not exist: {}", path)));
    }

    // Validate path is within an allowed directory
    let canonical = p.canonicalize()
        .map_err(|_| AppError::Io(format!("Cannot resolve path: {}", path)))?;
    let config = state.config.read().await;
    let allowed_roots: Vec<PathBuf> = [
        config.game_path.clone(),
        config.workshop_path.clone(),
        config.local_mods_path.clone(),
        config.zomboid_user_dir.clone(),
    ]
    .into_iter()
    .flatten()
    .chain(std::iter::once(state.app_data_dir.clone()))
    .collect();
    drop(config);

    let allowed = allowed_roots.iter().any(|root| {
        root.canonicalize()
            .map(|r| canonical.starts_with(&r))
            .unwrap_or(false)
    });
    if !allowed {
        return Err(AppError::Validation("Path is outside allowed directories".into()));
    }

    // Use the opener crate (or std::process::Command) to open the folder
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open folder: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open folder: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| AppError::Io(format!("Failed to open folder: {}", e)))?;
    }
    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

