use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::app_core::error::AppError;
use crate::app_core::types::{ModCategory, ModInfo, ModSource};
use super::parser::{parse_mod_info, ParsedModInfo};

// ─── Version folder helpers ────────────────────────────────────────────────────

/// Returns true if the folder name matches `<digits>` or `<digits>.<digits>`.
pub fn is_version_folder(name: &str) -> bool {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    match parts.as_slice() {
        [major] => major.chars().all(|c| c.is_ascii_digit()) && !major.is_empty(),
        [major, minor] => {
            !major.is_empty()
                && !minor.is_empty()
                && major.chars().all(|c| c.is_ascii_digit())
                && minor.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// Parse a version folder name into (major, minor) tuple.
/// "42" → (42, 0), "42.15" → (42, 15)
fn parse_folder_version(name: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = name.splitn(2, '.').collect();
    match parts.as_slice() {
        [major] => {
            let maj = major.parse::<u64>().ok()?;
            Some((maj, 0))
        }
        [major, minor] => {
            let maj = major.parse::<u64>().ok()?;
            let min = minor.parse::<u64>().ok()?;
            Some((maj, min))
        }
        _ => None,
    }
}

/// Parse a game version string into (major, minor).
/// "42.15.2" → (42, 15), "42.15" → (42, 15), "42" → (42, 0)
fn parse_game_version(version: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = version.splitn(3, '.').collect();
    match parts.as_slice() {
        [major] => Some((major.parse().ok()?, 0)),
        [major, minor] | [major, minor, _] => {
            Some((major.parse().ok()?, minor.parse().ok()?))
        }
        _ => None,
    }
}

/// Resolve the best matching version folder for a given game version.
///
/// - Parse game_version into (major, minor)
/// - For each folder, parse into (major, minor)
/// - Filter to folders where (major, minor) <= game (major, minor)
/// - Return the highest matching folder (numerically)
/// - Return None if no version folders exist
pub fn resolve_version_folder(
    available_folders: &[String],
    game_version: &str,
) -> Option<String> {
    if available_folders.is_empty() {
        return None;
    }

    let (game_major, game_minor) = parse_game_version(game_version)?;

    let mut candidates: Vec<(&String, u64, u64)> = available_folders
        .iter()
        .filter_map(|folder| {
            let (maj, min) = parse_folder_version(folder)?;
            // Only include if folder version <= game version
            if (maj, min) <= (game_major, game_minor) {
                Some((folder, maj, min))
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Sort by (major, minor) descending to get the highest matching version
    candidates.sort_by(|a, b| (b.1, b.2).cmp(&(a.1, a.2)));

    Some(candidates[0].0.clone())
}

// ─── Last-modified helper ──────────────────────────────────────────────────────

fn system_time_to_iso(t: SystemTime) -> String {
    use chrono::TimeZone;
    let secs = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    chrono::Utc
        .timestamp_opt(secs as i64, 0)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

fn last_modified_for_dir(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(system_time_to_iso)
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

// ─── ModInfo construction ──────────────────────────────────────────────────────

/// Search multiple directories for an image file, returning the first existing path.
fn find_image_in_dirs(dirs: &[&Path], filename: &str) -> Option<String> {
    if filename.is_empty() {
        return None;
    }
    for dir in dirs {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return candidate.to_str().map(String::from);
        }
    }
    None
}

/// Find any image file (.png, .jpg, .jpeg) in the given directory.
fn find_any_image(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if ext_lower == "png" || ext_lower == "jpg" || ext_lower == "jpeg" {
                return path.to_str().map(String::from);
            }
        }
    }
    None
}

fn build_mod_info(
    parsed: ParsedModInfo,
    source: ModSource,
    workshop_id: Option<String>,
    source_path: PathBuf,
    mod_info_path: PathBuf,
    version_folders: Vec<String>,
    active_version_folder: Option<String>,
) -> ModInfo {
    let mod_info_dir = mod_info_path.parent().unwrap_or(&source_path);

    // Search directories: version folder (mod_info_dir), then mod root (source_path)
    let search_dirs: Vec<&Path> = if mod_info_dir != source_path.as_path() {
        vec![mod_info_dir, &source_path]
    } else {
        vec![mod_info_dir]
    };

    // Cascading image resolution: poster → icon → any image in dirs
    let poster_path = parsed
        .poster
        .as_deref()
        .and_then(|p| find_image_in_dirs(&search_dirs, p))
        .or_else(|| parsed.icon.as_deref().and_then(|p| find_image_in_dirs(&search_dirs, p)))
        .or_else(|| {
            for dir in &search_dirs {
                if let Some(img) = find_any_image(dir) {
                    return Some(img);
                }
            }
            None
        });
    let icon_path = parsed
        .icon
        .as_deref()
        .and_then(|p| find_image_in_dirs(&search_dirs, p));

    let last_modified = last_modified_for_dir(&source_path);

    // detected_category: Map if pack or tile_def is non-empty; None otherwise.
    // Framework detection is applied in post-processing.
    let detected_category = if parsed.pack.is_some() || !parsed.tile_def.is_empty() {
        Some(ModCategory::Map)
    } else {
        None
    };

    ModInfo {
        id: parsed.id,
        raw_id: parsed.raw_id,
        workshop_id,
        name: parsed.name,
        description: parsed.description,
        authors: parsed.authors,
        url: parsed.url,
        mod_version: parsed.mod_version,
        poster_path,
        icon_path,
        version_min: parsed.version_min,
        version_max: parsed.version_max,
        version_folders,
        active_version_folder,
        requires: parsed.requires,
        pack: parsed.pack,
        tile_def: parsed.tile_def,
        category: parsed.category,
        source,
        source_path,
        mod_info_path,
        size_bytes: None,
        last_modified,
        detected_category,
    }
}

// ─── Version folder detection ──────────────────────────────────────────────────

/// Collect all version folder names from a mod directory.
fn collect_version_folders(mod_dir: &Path) -> Vec<String> {
    let mut folders = Vec::new();
    let entries = match std::fs::read_dir(mod_dir) {
        Ok(e) => e,
        Err(_) => return folders,
    };
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if is_version_folder(&name_str) {
            folders.push(name_str.into_owned());
        }
    }
    // Sort ascending by version number
    folders.sort_by(|a, b| {
        let va = parse_folder_version(a).unwrap_or((0, 0));
        let vb = parse_folder_version(b).unwrap_or((0, 0));
        va.cmp(&vb)
    });
    folders
}

// ─── Single mod scanning ──────────────────────────────────────────────────────

/// Attempt to scan a single mod directory.
/// Returns Ok(Some(ModInfo)) on success, Ok(None) if no parseable mod.info was found.
fn scan_mod_dir(
    mod_dir: &Path,
    source: ModSource,
    workshop_id: Option<String>,
    game_version: &str,
) -> Result<Option<ModInfo>, AppError> {
    let version_folders = collect_version_folders(mod_dir);
    let active_version_folder = resolve_version_folder(&version_folders, game_version);

    // Determine which mod.info to parse
    // Priority: version folder > root > common/ (PZ B42 pattern)
    let mod_info_path = if let Some(ref vf) = active_version_folder {
        let versioned_path = mod_dir.join(vf).join("mod.info");
        if versioned_path.exists() {
            versioned_path
        } else {
            mod_dir.join("mod.info")
        }
    } else {
        mod_dir.join("mod.info")
    };

    // Fallback to common/mod.info (some B42 mods put mod.info there)
    let mod_info_path = if mod_info_path.exists() {
        mod_info_path
    } else {
        let common_path = mod_dir.join("common").join("mod.info");
        if common_path.exists() {
            common_path
        } else {
            return Ok(None);
        }
    };

    let parsed = match parse_mod_info(&mod_info_path) {
        Ok(p) => p,
        Err(AppError::Parse(msg)) => {
            // Skip mods with unparseable mod.info (log and continue)
            log::warn!("Skipping mod at {:?}: parse error: {}", mod_dir, msg);
            return Ok(None);
        }
        Err(e) => return Err(e),
    };

    let info = build_mod_info(
        parsed,
        source,
        workshop_id,
        mod_dir.to_path_buf(),
        mod_info_path,
        version_folders,
        active_version_folder,
    );

    Ok(Some(info))
}

/// Rescan a single mod using a specific version folder override.
/// Used when the user manually selects a different version for a mod.
pub fn rescan_mod_with_version(
    source_path: &Path,
    source: ModSource,
    workshop_id: Option<String>,
    version_folder: Option<&str>,
    game_version: &str,
) -> Result<Option<ModInfo>, AppError> {
    let version_folders = collect_version_folders(source_path);
    let active_version_folder = match version_folder {
        Some(vf) if version_folders.contains(&vf.to_string()) => Some(vf.to_string()),
        _ => resolve_version_folder(&version_folders, game_version),
    };

    let mod_info_path = if let Some(ref vf) = active_version_folder {
        let versioned_path = source_path.join(vf).join("mod.info");
        if versioned_path.exists() {
            versioned_path
        } else {
            source_path.join("mod.info")
        }
    } else {
        source_path.join("mod.info")
    };

    let mod_info_path = if mod_info_path.exists() {
        mod_info_path
    } else {
        let common_path = source_path.join("common").join("mod.info");
        if common_path.exists() {
            common_path
        } else {
            return Ok(None);
        }
    };

    let parsed = parse_mod_info(&mod_info_path)?;
    let info = build_mod_info(
        parsed,
        source,
        workshop_id,
        source_path.to_path_buf(),
        mod_info_path,
        version_folders,
        active_version_folder,
    );
    Ok(Some(info))
}

// ─── Framework post-processing ─────────────────────────────────────────────────

/// After all mods are scanned, any mod required by 3+ other mods is marked Framework.
fn detect_frameworks(mods: &mut Vec<ModInfo>) {
    // Count how many mods require each mod ID
    let mut required_by_count: HashMap<String, usize> = HashMap::new();
    for m in mods.iter() {
        for req in &m.requires {
            *required_by_count.entry(req.clone()).or_insert(0) += 1;
        }
    }

    for m in mods.iter_mut() {
        if required_by_count.get(&m.id).copied().unwrap_or(0) >= 3 {
            m.detected_category = Some(ModCategory::Framework);
        }
    }
}

// ─── Main scanner ──────────────────────────────────────────────────────────────

/// Scan workshop, local, and game-bundled mod directories and return all discovered mods.
///
/// Workshop path structure: `<workshopPath>/<workshopId>/mods/<modName>/`
/// Local path structure: `<localModsPath>/<modName>/`
/// Game mods path structure: `<gamePath>/mods/<modName>/`
pub fn scan_mod_directories(
    workshop_path: Option<&Path>,
    local_mods_path: Option<&Path>,
    game_path: Option<&Path>,
    game_version: &str,
) -> Result<Vec<ModInfo>, AppError> {
    let mut mods = Vec::new();

    // ── Workshop mods ──
    if let Some(workshop_root) = workshop_path {
        if workshop_root.exists() {
            // Each direct child is a numeric workshop ID directory
            let entries = std::fs::read_dir(workshop_root)
                .map_err(|e| AppError::Io(format!("Cannot read workshop dir {:?}: {}", workshop_root, e)))?;

            for entry in entries.flatten() {
                let workshop_id_dir = entry.path();
                if !workshop_id_dir.is_dir() {
                    continue;
                }
                let dir_name = entry.file_name();
                let workshop_id = dir_name.to_string_lossy().into_owned();

                // Must be all digits to be a valid workshop ID
                if !workshop_id.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }

                let mods_subdir = workshop_id_dir.join("mods");
                if !mods_subdir.exists() {
                    continue;
                }

                // Each child of mods/ is an individual mod folder
                let mod_entries = match std::fs::read_dir(&mods_subdir) {
                    Ok(e) => e,
                    Err(e) => {
                        log::warn!("Cannot read mods dir {:?}: {}", mods_subdir, e);
                        continue;
                    }
                };

                for mod_entry in mod_entries.flatten() {
                    let mod_dir = mod_entry.path();
                    if !mod_dir.is_dir() {
                        continue;
                    }

                    match scan_mod_dir(
                        &mod_dir,
                        ModSource::Workshop,
                        Some(workshop_id.clone()),
                        game_version,
                    ) {
                        Ok(Some(info)) => mods.push(info),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("Error scanning mod at {:?}: {}", mod_dir, e);
                        }
                    }
                }
            }
        }
    }

    // ── Local mods ──
    if let Some(local_root) = local_mods_path {
        if local_root.exists() {
            let entries = std::fs::read_dir(local_root)
                .map_err(|e| AppError::Io(format!("Cannot read local mods dir {:?}: {}", local_root, e)))?;

            for entry in entries.flatten() {
                let mod_dir = entry.path();
                if !mod_dir.is_dir() {
                    continue;
                }

                let dir_name = entry.file_name();
                let dir_name_str = dir_name.to_string_lossy();

                // Skip the example mod
                if dir_name_str == "examplemod" {
                    continue;
                }

                match scan_mod_dir(&mod_dir, ModSource::Local, None, game_version) {
                    Ok(Some(info)) => mods.push(info),
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!("Error scanning local mod at {:?}: {}", mod_dir, e);
                    }
                }
            }
        }
    }

    // ── Game built-in mods ──
    if let Some(game_root) = game_path {
        let game_mods_dir = game_root.join("mods");
        if game_mods_dir.exists() {
            let entries = std::fs::read_dir(&game_mods_dir)
                .map_err(|e| AppError::Io(format!("Cannot read game mods dir {:?}: {}", game_mods_dir, e)))?;

            for entry in entries.flatten() {
                let mod_dir = entry.path();
                if !mod_dir.is_dir() {
                    continue;
                }

                let dir_name = entry.file_name();
                let dir_name_str = dir_name.to_string_lossy();

                // Skip the example mod
                if dir_name_str == "examplemod" {
                    continue;
                }

                // Skip if we already discovered this mod from workshop or local
                // (workshop/local takes precedence over game-bundled)
                match scan_mod_dir(&mod_dir, ModSource::Local, None, game_version) {
                    Ok(Some(info)) => {
                        if !mods.iter().any(|m| m.id == info.id) {
                            mods.push(info);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!("Error scanning game mod at {:?}: {}", mod_dir, e);
                    }
                }
            }
        }
    }

    // Post-processing: detect framework mods
    detect_frameworks(&mut mods);

    Ok(mods)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

