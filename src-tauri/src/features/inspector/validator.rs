use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::app_core::types::ModInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub mod_id: String,
    pub mod_name: String,
    pub checks: Vec<InspectionCheck>,
    pub score: u32,          // 0-100 compatibility/quality score
    pub lua_file_count: u32,
    pub script_file_count: u32,
    pub texture_count: u32,
    pub total_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionCheck {
    pub name: String,
    pub passed: bool,
    pub severity: String,    // "error", "warning", "info"
    pub message: String,
}

/// Run all validation checks on a mod.
pub fn inspect_mod(mod_info: &ModInfo) -> InspectionReport {
    let mut checks = Vec::new();
    let mut score: i32 = 100;

    // 1. Check mod.info exists
    let mod_info_exists = mod_info.mod_info_path.exists();
    checks.push(InspectionCheck {
        name: "mod.info exists".into(),
        passed: mod_info_exists,
        severity: if mod_info_exists { "info" } else { "error" }.into(),
        message: if mod_info_exists {
            "mod.info file found".into()
        } else {
            "mod.info file is missing".into()
        },
    });
    if !mod_info_exists { score -= 30; }

    // 2. Check required fields
    let has_name = !mod_info.name.is_empty() && mod_info.name != mod_info.id;
    checks.push(InspectionCheck {
        name: "Has descriptive name".into(),
        passed: has_name,
        severity: if has_name { "info" } else { "warning" }.into(),
        message: if has_name {
            format!("Name: {}", mod_info.name)
        } else {
            "Mod name is missing or same as ID".into()
        },
    });
    if !has_name { score -= 10; }

    let has_id = !mod_info.id.is_empty();
    checks.push(InspectionCheck {
        name: "Has mod ID".into(),
        passed: has_id,
        severity: if has_id { "info" } else { "error" }.into(),
        message: if has_id {
            format!("ID: {}", mod_info.id)
        } else {
            "Mod ID is missing".into()
        },
    });
    if !has_id { score -= 20; }

    // 3. Check description
    let has_description = !mod_info.description.is_empty();
    checks.push(InspectionCheck {
        name: "Has description".into(),
        passed: has_description,
        severity: "info".into(),
        message: if has_description {
            format!("Description: {} chars", mod_info.description.len())
        } else {
            "No description provided".into()
        },
    });
    if !has_description { score -= 5; }

    // 4. Check authors
    let has_authors = !mod_info.authors.is_empty();
    checks.push(InspectionCheck {
        name: "Has author(s)".into(),
        passed: has_authors,
        severity: if has_authors { "info" } else { "warning" }.into(),
        message: if has_authors {
            format!("Authors: {}", mod_info.authors.join(", "))
        } else {
            "No authors specified".into()
        },
    });
    if !has_authors { score -= 5; }

    // 5. Check version fields
    let has_version_info = mod_info.version_min.is_some() || mod_info.version_max.is_some();
    checks.push(InspectionCheck {
        name: "Has version compatibility info".into(),
        passed: has_version_info,
        severity: if has_version_info { "info" } else { "warning" }.into(),
        message: if has_version_info {
            format!("Version range: {} - {}",
                mod_info.version_min.as_deref().unwrap_or("any"),
                mod_info.version_max.as_deref().unwrap_or("any"))
        } else {
            "No versionMin/versionMax specified — may not work across PZ versions".into()
        },
    });
    if !has_version_info { score -= 5; }

    // 6. Check media directory exists (version-folder aware)
    let media_path = if let Some(ref vf) = mod_info.active_version_folder {
        let ver_media = mod_info.source_path.join(vf).join("media");
        if ver_media.exists() { ver_media } else { mod_info.source_path.join("media") }
    } else {
        let common_media = mod_info.source_path.join("common").join("media");
        if common_media.exists() { common_media } else { mod_info.source_path.join("media") }
    };
    let has_media = media_path.exists();
    checks.push(InspectionCheck {
        name: "Has media/ directory".into(),
        passed: has_media,
        severity: if has_media { "info" } else { "error" }.into(),
        message: if has_media {
            format!("media/ directory found at {}", media_path.strip_prefix(&mod_info.source_path).unwrap_or(&media_path).display())
        } else {
            "media/ directory is missing — mod has no content".into()
        },
    });
    if !has_media { score -= 20; }

    // 7. Check poster image
    let has_poster = mod_info.poster_path.is_some();
    checks.push(InspectionCheck {
        name: "Has poster image".into(),
        passed: has_poster,
        severity: "info".into(),
        message: if has_poster {
            "Poster image found".into()
        } else {
            "No poster image — mod will have no thumbnail".into()
        },
    });
    if !has_poster { score -= 5; }

    // 8. Check for Workshop origin
    let is_workshop = mod_info.workshop_id.is_some();
    checks.push(InspectionCheck {
        name: "Workshop origin".into(),
        passed: is_workshop,
        severity: "info".into(),
        message: if is_workshop {
            format!("Workshop ID: {}", mod_info.workshop_id.as_deref().unwrap_or(""))
        } else {
            "Local mod (not from Workshop)".into()
        },
    });

    // 9. Count files
    let (lua_count, script_count, texture_count, total_count) = if has_media {
        count_mod_files(&media_path)
    } else {
        (0, 0, 0, 0)
    };

    checks.push(InspectionCheck {
        name: "Content analysis".into(),
        passed: total_count > 0,
        severity: "info".into(),
        message: format!("{} files ({} Lua, {} scripts, {} textures)",
            total_count, lua_count, script_count, texture_count),
    });

    // 10. Check mod version
    let has_mod_version = mod_info.mod_version.is_some();
    checks.push(InspectionCheck {
        name: "Has mod version".into(),
        passed: has_mod_version,
        severity: "info".into(),
        message: if has_mod_version {
            format!("Version: {}", mod_info.mod_version.as_deref().unwrap_or(""))
        } else {
            "No mod version specified".into()
        },
    });

    score = score.max(0).min(100);

    InspectionReport {
        mod_id: mod_info.id.clone(),
        mod_name: mod_info.name.clone(),
        checks,
        score: score as u32,
        lua_file_count: lua_count,
        script_file_count: script_count,
        texture_count,
        total_files: total_count,
    }
}

fn count_mod_files(media_path: &Path) -> (u32, u32, u32, u32) {
    let mut lua = 0u32;
    let mut scripts = 0u32;
    let mut textures = 0u32;
    let mut total = 0u32;

    for entry in walkdir::WalkDir::new(media_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        total += 1;
        let path = entry.path().to_string_lossy().to_lowercase();
        if path.ends_with(".lua") {
            lua += 1;
        } else if path.contains("scripts") && path.ends_with(".txt") {
            scripts += 1;
        } else if path.ends_with(".png") || path.ends_with(".dds") {
            textures += 1;
        }
    }

    (lua, scripts, textures, total)
}

