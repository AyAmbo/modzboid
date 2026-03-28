use serde::{Deserialize, Serialize};
use crate::app_core::types::ModInfo;
use crate::app_core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModListExport {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub exported_at: String,
    pub game_version: Option<String>,
    pub mod_count: usize,
    pub mods: Vec<ModListEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModListEntry {
    pub id: String,
    pub name: String,
    pub workshop_id: Option<String>,
    pub authors: Vec<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub total: usize,
    pub found: Vec<String>,
    pub missing: Vec<MissingMod>,
    pub detected_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingMod {
    pub id: String,
    pub name: Option<String>,
    pub workshop_id: Option<String>,
}

/// Export a mod list as pretty-printed JSON.
pub fn export_as_json(
    profile_name: &str,
    game_version: Option<&str>,
    mods: &[ModInfo],
) -> Result<String, AppError> {
    let export = ModListExport {
        format: "modzboid-modlist".to_string(),
        version: 1,
        name: profile_name.to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        game_version: game_version.map(|s| s.to_string()),
        mod_count: mods.len(),
        mods: mods
            .iter()
            .map(|m| ModListEntry {
                id: m.id.clone(),
                name: m.name.clone(),
                workshop_id: m.workshop_id.clone(),
                authors: m.authors.clone(),
                url: m.url.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&export)?;
    Ok(json)
}

/// Export a mod list as CSV with header row.
pub fn export_as_csv(mods: &[ModInfo]) -> String {
    let mut lines = Vec::with_capacity(mods.len() + 1);
    lines.push("id,name,workshopId,url".to_string());
    for m in mods {
        lines.push(format!(
            "{},{},{},{}",
            csv_escape(&m.id),
            csv_escape(&m.name),
            csv_escape(m.workshop_id.as_deref().unwrap_or("")),
            csv_escape(m.url.as_deref().unwrap_or("")),
        ));
    }
    lines.join("\n")
}

/// Escape a value for CSV output. Wraps in quotes if necessary.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Export a mod list as plain text.
pub fn export_as_text(profile_name: &str, mods: &[ModInfo]) -> String {
    let mut lines = Vec::with_capacity(mods.len() + 2);
    lines.push(format!("# Profile: {}", profile_name));
    lines.push(format!("# Exported: {}", chrono::Utc::now().to_rfc3339()));
    for m in mods {
        lines.push(m.id.clone());
    }
    lines.join("\n")
}

/// Auto-detect import format and parse mod IDs, cross-referencing with known installed mods.
pub fn parse_import(
    content: &str,
    known_mod_ids: &[String],
) -> Result<ImportPreview, AppError> {
    let trimmed = content.trim();

    if trimmed.starts_with('{') {
        return parse_json_import(trimmed, known_mod_ids);
    }

    if trimmed.starts_with("id,") {
        return Ok(parse_csv_import(trimmed, known_mod_ids));
    }

    Ok(parse_text_import(trimmed, known_mod_ids))
}

fn parse_json_import(
    content: &str,
    known_mod_ids: &[String],
) -> Result<ImportPreview, AppError> {
    let export: ModListExport = serde_json::from_str(content)?;
    let mod_ids: Vec<String> = export.mods.iter().map(|e| e.id.clone()).collect();
    let (found, missing) = split_found_missing(&mod_ids, &export.mods, known_mod_ids);

    Ok(ImportPreview {
        total: mod_ids.len(),
        found,
        missing,
        detected_format: "json".to_string(),
    })
}

fn parse_csv_import(content: &str, known_mod_ids: &[String]) -> ImportPreview {
    let mut mod_ids = Vec::new();
    let mut entries = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if i == 0 {
            continue; // skip header
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_line(line);
        if let Some(id) = fields.first() {
            let id = id.trim().to_string();
            if !id.is_empty() {
                let name = fields.get(1).cloned();
                let workshop_id = fields.get(2).and_then(|s| {
                    let s = s.trim();
                    if s.is_empty() { None } else { Some(s.to_string()) }
                });
                entries.push(ModListEntry {
                    id: id.clone(),
                    name: name.unwrap_or_default(),
                    workshop_id,
                    authors: vec![],
                    url: fields.get(3).cloned(),
                });
                mod_ids.push(id);
            }
        }
    }

    let (found, missing) = split_found_missing(&mod_ids, &entries, known_mod_ids);

    ImportPreview {
        total: mod_ids.len(),
        found,
        missing,
        detected_format: "csv".to_string(),
    }
}

fn parse_text_import(content: &str, known_mod_ids: &[String]) -> ImportPreview {
    let mut mod_ids = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        mod_ids.push(line.to_string());
    }

    let entries: Vec<ModListEntry> = mod_ids
        .iter()
        .map(|id| ModListEntry {
            id: id.clone(),
            name: String::new(),
            workshop_id: None,
            authors: vec![],
            url: None,
        })
        .collect();

    let (found, missing) = split_found_missing(&mod_ids, &entries, known_mod_ids);

    ImportPreview {
        total: mod_ids.len(),
        found,
        missing,
        detected_format: "text".to_string(),
    }
}

/// Split mod IDs into found (installed) and missing lists.
fn split_found_missing(
    mod_ids: &[String],
    entries: &[ModListEntry],
    known_mod_ids: &[String],
) -> (Vec<String>, Vec<MissingMod>) {
    let mut found = Vec::new();
    let mut missing = Vec::new();

    for (i, id) in mod_ids.iter().enumerate() {
        if known_mod_ids.contains(id) {
            found.push(id.clone());
        } else {
            let entry = entries.get(i);
            missing.push(MissingMod {
                id: id.clone(),
                name: entry.and_then(|e| {
                    if e.name.is_empty() { None } else { Some(e.name.clone()) }
                }),
                workshop_id: entry.and_then(|e| e.workshop_id.clone()),
            });
        }
    }

    (found, missing)
}

/// Simple CSV line parser that handles quoted fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::app_core::types::{ModSource, ModCategory};

    fn make_test_mod(id: &str, name: &str, workshop_id: Option<&str>) -> ModInfo {
        ModInfo {
            id: id.to_string(),
            raw_id: id.to_string(),
            workshop_id: workshop_id.map(|s| s.to_string()),
            name: name.to_string(),
            description: String::new(),
            authors: vec!["TestAuthor".to_string()],
            url: Some(format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", workshop_id.unwrap_or("0"))),
            mod_version: Some("1.0".to_string()),
            poster_path: None,
            icon_path: None,
            version_min: None,
            version_max: None,
            version_folders: vec![],
            active_version_folder: None,
            requires: vec![],
            pack: None,
            tile_def: vec![],
            category: None,
            source: ModSource::Workshop,
            source_path: PathBuf::from("/mods/test"),
            mod_info_path: PathBuf::from("/mods/test/mod.info"),
            size_bytes: Some(1024),
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            detected_category: Some(ModCategory::Content),
        }
    }

    #[test]
    fn json_round_trip() {
        let mods = vec![
            make_test_mod("mod-a", "Mod A", Some("111")),
            make_test_mod("mod-b", "Mod B", Some("222")),
        ];

        let json = export_as_json("TestProfile", Some("42.15"), &mods).unwrap();
        let known = vec!["mod-a".to_string(), "mod-b".to_string()];
        let preview = parse_import(&json, &known).unwrap();

        assert_eq!(preview.detected_format, "json");
        assert_eq!(preview.total, 2);
        assert_eq!(preview.found.len(), 2);
        assert!(preview.missing.is_empty());
    }

    #[test]
    fn csv_export_and_parse() {
        let mods = vec![
            make_test_mod("mod-a", "Mod A", Some("111")),
            make_test_mod("mod-b", "Mod B", None),
        ];

        let csv = export_as_csv(&mods);
        assert!(csv.starts_with("id,name,workshopId,url"));

        let known = vec!["mod-a".to_string(), "mod-b".to_string()];
        let preview = parse_import(&csv, &known).unwrap();

        assert_eq!(preview.detected_format, "csv");
        assert_eq!(preview.total, 2);
        assert_eq!(preview.found.len(), 2);
        assert!(preview.missing.is_empty());
    }

    #[test]
    fn text_export_and_parse() {
        let mods = vec![
            make_test_mod("mod-a", "Mod A", Some("111")),
            make_test_mod("mod-b", "Mod B", Some("222")),
        ];

        let text = export_as_text("TestProfile", &mods);
        assert!(text.contains("# Profile: TestProfile"));
        assert!(text.contains("mod-a"));
        assert!(text.contains("mod-b"));

        let known = vec!["mod-a".to_string(), "mod-b".to_string()];
        let preview = parse_import(&text, &known).unwrap();

        assert_eq!(preview.detected_format, "text");
        assert_eq!(preview.total, 2);
        assert_eq!(preview.found.len(), 2);
        assert!(preview.missing.is_empty());
    }

    #[test]
    fn import_with_missing_mods() {
        let mods = vec![
            make_test_mod("mod-a", "Mod A", Some("111")),
            make_test_mod("mod-b", "Mod B", Some("222")),
            make_test_mod("mod-c", "Mod C", Some("333")),
        ];

        let json = export_as_json("Mixed", None, &mods).unwrap();
        // Only mod-a is "installed"
        let known = vec!["mod-a".to_string()];
        let preview = parse_import(&json, &known).unwrap();

        assert_eq!(preview.total, 3);
        assert_eq!(preview.found.len(), 1);
        assert_eq!(preview.found[0], "mod-a");
        assert_eq!(preview.missing.len(), 2);
        assert_eq!(preview.missing[0].id, "mod-b");
        assert_eq!(preview.missing[0].name, Some("Mod B".to_string()));
        assert_eq!(preview.missing[0].workshop_id, Some("222".to_string()));
        assert_eq!(preview.missing[1].id, "mod-c");
    }

    #[test]
    fn csv_escape_handles_special_chars() {
        assert_eq!(csv_escape("simple"), "simple");
        assert_eq!(csv_escape("has,comma"), "\"has,comma\"");
        assert_eq!(csv_escape("has\"quote"), "\"has\"\"quote\"");
    }

    #[test]
    fn parse_csv_line_handles_quoted_fields() {
        let fields = parse_csv_line("a,\"b,c\",d");
        assert_eq!(fields, vec!["a", "b,c", "d"]);

        let fields = parse_csv_line("a,\"b\"\"c\",d");
        assert_eq!(fields, vec!["a", "b\"c", "d"]);
    }

    #[test]
    fn text_import_skips_comments_and_empty_lines() {
        let content = "# Profile: Test\n# Exported: 2024-01-01\n\nmod-a\n\nmod-b\n";
        let known = vec!["mod-a".to_string()];
        let preview = parse_import(content, &known).unwrap();

        assert_eq!(preview.detected_format, "text");
        assert_eq!(preview.total, 2);
        assert_eq!(preview.found, vec!["mod-a"]);
        assert_eq!(preview.missing.len(), 1);
        assert_eq!(preview.missing[0].id, "mod-b");
    }
}
