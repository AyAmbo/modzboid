use std::collections::HashMap;
use std::path::Path;

use crate::app_core::error::AppError;

/// Intermediate parsed representation of a mod.info file.
/// Does NOT include filesystem-level fields (source_path, workshop_id, size_bytes, etc.)
/// — those are added by the scanner (Task 5).
#[derive(Debug, Clone)]
pub struct ParsedModInfo {
    /// Normalized mod ID (workshop prefix stripped, e.g. "ToadTraits")
    pub id: String,
    /// Raw id value as found in the file (e.g. "1299328280/ToadTraits")
    pub raw_id: String,
    pub name: String,
    /// Description with multiple `description=` lines joined by `\n`
    pub description: String,
    pub authors: Vec<String>,
    pub url: Option<String>,
    pub mod_version: Option<String>,
    /// Raw filename from mod.info (e.g. "poster.png")
    pub poster: Option<String>,
    /// Raw filename from mod.info (e.g. "icon.png")
    pub icon: Option<String>,
    pub version_min: Option<String>,
    pub version_max: Option<String>,
    /// Deduplicated, normalized list of required mod IDs
    pub requires: Vec<String>,
    pub pack: Option<String>,
    pub tile_def: Vec<String>,
    pub category: Option<String>,
    /// All unrecognised key=value pairs (keys are lowercased).
    /// Retained for future use (e.g. exposing raw mod.info data in the UI).
    #[allow(dead_code)]
    pub extras: HashMap<String, String>,
}

/// Strip a leading `<digits>/` prefix from a mod ID string.
///
/// Examples:
/// - `"ToadTraits"` → `"ToadTraits"`
/// - `"1299328280/ToadTraits"` → `"ToadTraits"`
pub fn normalize_id(raw: &str) -> String {
    // Pattern: one or more digits followed by '/' at the start
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b'/' {
        raw[i + 1..].to_string()
    } else {
        raw.to_string()
    }
}

/// Parse the authors field.
///
/// If `is_singular` is true (field name was `author`), the entire value becomes
/// a single-element Vec without splitting on commas.
/// If false (`authors` / `Authors`), split on commas and trim each element.
pub fn parse_authors(raw: &str, is_singular: bool) -> Vec<String> {
    if is_singular {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            vec![]
        } else {
            vec![trimmed.to_string()]
        }
    } else {
        raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Parse a single `require=` value into a list of normalized mod IDs.
///
/// Handles all six known formats:
/// 1. `ToadTraits`                       → `["ToadTraits"]`
/// 2. `\damnlib`                          → `["damnlib"]`
/// 3. `\1299328280/ToadTraits`            → `["ToadTraits"]`
/// 4. `\TheyKnewB42,\TheyKnewB42.13patch` → `["TheyKnewB42", "TheyKnewB42.13patch"]`
/// 5. `\1299328280/ToadTraits,\KillCount` → `["ToadTraits", "KillCount"]`
/// 6. plain ID without backslash          → `["SomeModA"]`
pub fn normalize_require(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|token| {
            // 1. Trim whitespace
            let token = token.trim();
            // 2. Strip leading backslash (literal '\')
            let token = token.strip_prefix('\\').unwrap_or(token);
            // 3. Strip leading <digits>/ prefix
            normalize_id(token)
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a mod.info file at `path` and return a `ParsedModInfo`.
///
/// Returns `AppError::Parse` if the file is empty or missing a `name` field.
/// Returns `AppError::Io` if the file cannot be read.
pub fn parse_mod_info(path: &Path) -> Result<ParsedModInfo, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::Io(format!("Failed to read {:?}: {}", path, e)))?;

    // Collect all key=value pairs. Lines without '=' are ignored.
    // All key names are lowercased for case-insensitive matching.
    let mut raw_id: Option<String> = None;
    let mut name: Option<String> = None;
    let mut description_parts: Vec<String> = Vec::new();
    let mut authors: Vec<String> = Vec::new();
    let mut authors_set = false;
    let mut url: Option<String> = None;
    let mut mod_version: Option<String> = None;
    // poster: last value wins (some mods repeat it, e.g. PROJECTRVInterior42)
    let mut poster: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut version_min: Option<String> = None;
    let mut version_max: Option<String> = None;
    let mut require_values: Vec<String> = Vec::new();
    let mut pack: Option<String> = None;
    let mut tile_def: Vec<String> = Vec::new();
    let mut category: Option<String> = None;
    let mut extras: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        // Skip lines without '='
        let Some(eq_pos) = line.find('=') else {
            continue;
        };

        let key_raw = &line[..eq_pos];
        let value = line[eq_pos + 1..].trim().to_string();
        let key = key_raw.trim().to_lowercase();

        match key.as_str() {
            "id" => {
                raw_id = Some(value);
            }
            "name" => {
                name = Some(value);
            }
            "description" => {
                // Multiple description= lines are concatenated with newline.
                // <LINE> tags are kept raw (the spec says "for display" the frontend
                // handles it, but the task says replace with \n — we replace here
                // so downstream code sees clean newlines).
                let processed = value.replace("<LINE>", "\n");
                description_parts.push(processed);
            }
            "author" => {
                // Singular: treat entire value as one author unless already set
                // by an `authors` field. Authors field takes precedence if both appear,
                // but we process in order — first encounter wins for the type.
                if !authors_set {
                    authors = parse_authors(&value, true);
                    authors_set = true;
                }
            }
            "authors" => {
                // Plural overrides singular if we already got an author= line
                authors = parse_authors(&value, false);
                authors_set = true;
            }
            "url" => {
                url = Some(value);
            }
            "modversion" => {
                mod_version = Some(value);
            }
            "poster" => {
                // Take the last poster value (some mods list multiple)
                poster = Some(value);
            }
            "icon" => {
                icon = Some(value);
            }
            "versionmin" => {
                version_min = Some(value);
            }
            "versionmax" => {
                version_max = Some(value);
            }
            "require" => {
                require_values.push(value);
            }
            "pack" => {
                pack = Some(value);
            }
            "tiledef" => {
                if !value.is_empty() {
                    tile_def.push(value);
                }
            }
            "category" => {
                category = Some(value);
            }
            _ => {
                // Capture unknown fields. If a key appears multiple times,
                // concatenate with newline (mirrors description behaviour).
                if !value.is_empty() {
                    extras
                        .entry(key)
                        .and_modify(|existing| {
                            existing.push('\n');
                            existing.push_str(&value);
                        })
                        .or_insert(value);
                } else {
                    extras.entry(key).or_insert_with(String::new);
                }
            }
        }
    }

    // Validate required fields
    let name = name.ok_or_else(|| {
        AppError::Parse(format!(
            "mod.info at {:?} is missing required field 'name'",
            path
        ))
    })?;

    if name.is_empty() {
        return Err(AppError::Parse(format!(
            "mod.info at {:?} has an empty 'name' field",
            path
        )));
    }

    // id is optional in some mods but the spec treats it as present; default to name if absent
    let raw_id_str = raw_id.unwrap_or_else(|| name.clone());
    let id = normalize_id(&raw_id_str);

    // Build description
    let description = description_parts.join("\n");

    // Process all require= lines: collect, normalize, deduplicate
    let mut requires: Vec<String> = Vec::new();
    for raw_require in &require_values {
        for normalized in normalize_require(raw_require) {
            if !requires.contains(&normalized) {
                requires.push(normalized);
            }
        }
    }

    Ok(ParsedModInfo {
        id,
        raw_id: raw_id_str,
        name,
        description,
        authors,
        url,
        mod_version,
        poster,
        icon,
        version_min,
        version_max,
        requires,
        pack,
        tile_def,
        category,
        extras,
    })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

