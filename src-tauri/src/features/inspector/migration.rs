//! Mod migration scanner — checks a mod's Lua files against deprecation rules
//! and reports what needs updating for B42 compatibility.

use std::path::Path;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub mod_id: String,
    pub mod_name: String,
    pub files_scanned: u32,
    pub files_with_issues: u32,
    pub total_issues: u32,
    pub auto_fixable: u32,
    pub needs_review: u32,
    pub issues: Vec<MigrationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationIssue {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub old_api: String,
    pub replacement: Option<String>,
    pub auto_fixable: bool,
    pub severity: String,
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeprecationRule {
    pub pattern: String,
    pub message: String,
    pub suggestion: String,
    pub severity: String,
    pub source: String,
}

/// Scan a mod's Lua files against deprecation rules and report migration issues.
/// If `active_version_folder` is set, scans the version-specific code instead of root.
pub fn scan_mod_migration(
    source_path: &Path,
    mod_id: &str,
    mod_name: &str,
    rules: &[DeprecationRule],
    active_version_folder: Option<&str>,
) -> MigrationReport {
    // Build list of directories to scan, in priority order:
    // 1. Version-specific: {source_path}/{version}/media/lua/
    // 2. Common shared code: {source_path}/common/media/lua/
    // 3. Fallback to root: {source_path}/media/lua/
    let mut scan_dirs = Vec::new();

    if let Some(ver_folder) = active_version_folder {
        let ver_lua = source_path.join(ver_folder).join("media").join("lua");
        let ver_media = source_path.join(ver_folder).join("media");
        if ver_lua.exists() {
            scan_dirs.push(ver_lua);
        } else if ver_media.exists() {
            scan_dirs.push(ver_media);
        }
    }

    // Also scan common/ (shared across versions)
    let common_lua = source_path.join("common").join("media").join("lua");
    let common_media = source_path.join("common").join("media");
    if common_lua.exists() {
        scan_dirs.push(common_lua);
    } else if common_media.exists() {
        scan_dirs.push(common_media);
    }

    // Fallback: if no version folder found or specified, scan root
    if scan_dirs.is_empty() {
        let media_path = source_path.join("media");
        let lua_dir = media_path.join("lua");
        if lua_dir.exists() {
            scan_dirs.push(lua_dir);
        } else if media_path.exists() {
            scan_dirs.push(media_path);
        }
    }

    if scan_dirs.is_empty() {
        return MigrationReport {
            mod_id: mod_id.into(),
            mod_name: mod_name.into(),
            files_scanned: 0,
            files_with_issues: 0,
            total_issues: 0,
            auto_fixable: 0,
            needs_review: 0,
            issues: vec![],
        };
    }

    let mut issues = Vec::new();
    let mut files_scanned = 0u32;
    let mut files_with_issues = 0u32;

    // Compile all rules into regexes
    let compiled_rules: Vec<(Regex, &DeprecationRule)> = rules.iter()
        .filter_map(|r| Regex::new(&r.pattern).ok().map(|re| (re, r)))
        .collect();

    for scan_dir in &scan_dirs {
    for entry in walkdir::WalkDir::new(scan_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "lua"))
    {
        let path = entry.path();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        files_scanned += 1;
        let rel_path = path.strip_prefix(source_path).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().to_string();

        let mut file_has_issues = false;

        let mut in_block_comment = false;
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle block comments: --[[ ... --]]
            if in_block_comment {
                if trimmed.contains("--]]") || trimmed.contains("]]") {
                    in_block_comment = false;
                }
                continue;
            }
            if trimmed.starts_with("--[[") || trimmed.contains("--[[") {
                if !trimmed.contains("]]") {
                    in_block_comment = true;
                }
                continue;
            }
            // Skip single-line comments
            if trimmed.starts_with("--") {
                continue;
            }

            for (re, rule) in &compiled_rules {
                if let Some(mat) = re.find(trimmed) {
                    // Skip matches inside string literals
                    let before = &trimmed[..mat.start()];
                    let single_quotes = before.matches('\'').count();
                    let double_quotes = before.matches('"').count();
                    if single_quotes % 2 == 1 || double_quotes % 2 == 1 {
                        continue; // match is inside a string
                    }
                    file_has_issues = true;
                    // Only mark as auto-fixable when there's a concrete replacement
                    // (suggestion contains "→" with actual new API name, not just "Check the API docs")
                    let auto_fixable = rule.suggestion.contains("→")
                        && !rule.suggestion.contains("Check the API docs");

                    issues.push(MigrationIssue {
                        file: rel_str.clone(),
                        line: (line_num + 1) as u32,
                        column: mat.start() as u32,
                        old_api: mat.as_str().to_string(),
                        replacement: if rule.suggestion.contains("→") {
                            // Extract replacement from "Use X → Y" style suggestion
                            rule.suggestion.split('→').nth(1).map(|s| s.trim().to_string())
                        } else {
                            None
                        },
                        auto_fixable,
                        severity: rule.severity.clone(),
                        category: if rule.message.contains("removed") {
                            "removed".into()
                        } else if rule.message.contains("changed") {
                            "changed".into()
                        } else {
                            "deprecated".into()
                        },
                        message: rule.message.clone(),
                    });
                }
            }
        }

        if file_has_issues {
            files_with_issues += 1;
        }
    }
    } // end for scan_dir in &scan_dirs

    let auto_fixable = issues.iter().filter(|i| i.auto_fixable).count() as u32;
    let needs_review = issues.iter().filter(|i| !i.auto_fixable).count() as u32;

    MigrationReport {
        mod_id: mod_id.into(),
        mod_name: mod_name.into(),
        files_scanned,
        files_with_issues,
        total_issues: issues.len() as u32,
        auto_fixable,
        needs_review,
        issues,
    }
}

/// Load deprecation rules from a JSON file.
pub fn load_rules(path: &Path) -> Vec<DeprecationRule> {
    if !path.exists() {
        return vec![];
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => vec![],
    }
}
