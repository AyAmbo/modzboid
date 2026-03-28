//! Script property compatibility checker.
//! Scans mod .txt script files for deprecated B41 properties.

use std::path::Path;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptPropertyRule {
    pub block_type: String,
    pub property: String,
    pub message: String,
    pub suggestion: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCheckReport {
    pub mod_id: String,
    pub mod_name: String,
    pub files_scanned: u32,
    pub total_issues: u32,
    pub issues: Vec<ScriptIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptIssue {
    pub file: String,
    pub line: u32,
    pub block_type: String,
    pub property: String,
    pub value: String,
    pub message: String,
    pub suggestion: String,
    pub severity: String,
}

/// Scan a mod's script files for deprecated properties.
pub fn check_script_properties(
    source_path: &Path,
    mod_id: &str,
    mod_name: &str,
    rules: &[ScriptPropertyRule],
    active_version_folder: Option<&str>,
) -> ScriptCheckReport {
    // Build lookup: (block_type, property_name_EXACT) -> rule
    // We match the EXACT property name from the rule. The rule specifies the
    // OLD/deprecated form (e.g., "SwingTime", "ToolTip"). If the mod already
    // uses the correct B42 form (e.g., "Swingtime"), it should NOT be flagged.
    let rule_map: std::collections::HashMap<(String, String), &ScriptPropertyRule> = rules
        .iter()
        .map(|r| ((r.block_type.to_lowercase(), r.property.clone()), r))
        .collect();

    // Determine directories to scan (same logic as migration scanner)
    let mut scan_dirs = Vec::new();

    if let Some(vf) = active_version_folder {
        let ver_scripts = source_path.join(vf).join("media").join("scripts");
        if ver_scripts.exists() {
            scan_dirs.push(ver_scripts);
        }
    }

    let common_scripts = source_path.join("common").join("media").join("scripts");
    if common_scripts.exists() {
        scan_dirs.push(common_scripts);
    }

    if scan_dirs.is_empty() {
        let root_scripts = source_path.join("media").join("scripts");
        if root_scripts.exists() {
            scan_dirs.push(root_scripts);
        }
    }

    let block_type_re = Regex::new(
        r"(?i)^(item|recipe|vehicle|model|sound|fixing|template|evolvedrecipe|craftrecipe|entity|fluid)\s+\S+"
    ).unwrap();
    let prop_re = Regex::new(r"^([A-Za-z_]\w*)\s*=\s*(.*)").unwrap();

    let mut issues = Vec::new();
    let mut files_scanned = 0u32;

    for scan_dir in &scan_dirs {
        for entry in walkdir::WalkDir::new(scan_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "txt"))
        {
            let path = entry.path();
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            files_scanned += 1;
            let rel_path = path.strip_prefix(source_path).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy().to_string();

            let mut current_block_type: Option<String> = None;
            let mut waiting_for_brace = false;
            let mut brace_depth = 0i32;

            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("//") {
                    continue;
                }

                // Detect block type (item Name, recipe Name, etc.)
                // The { may be on the SAME line or the NEXT line
                if current_block_type.is_none() || brace_depth <= 1 {
                    if let Some(caps) = block_type_re.captures(trimmed) {
                        current_block_type = Some(caps[1].to_lowercase());
                        waiting_for_brace = true;
                    }
                }

                let opens = trimmed.matches('{').count() as i32;
                let closes = trimmed.matches('}').count() as i32;
                brace_depth += opens - closes;

                if waiting_for_brace && opens > 0 {
                    waiting_for_brace = false;
                }

                // Check properties inside blocks (depth >= 2: module { block { props } })
                if let Some(ref bt) = current_block_type {
                    if brace_depth >= 2 && !waiting_for_brace {
                        if let Some(caps) = prop_re.captures(trimmed) {
                            let prop_name = &caps[1];
                            let prop_value = caps[2].trim_end_matches(',').trim();

                            let key = (bt.clone(), prop_name.to_string());
                            if let Some(rule) = rule_map.get(&key) {
                                issues.push(ScriptIssue {
                                    file: rel_str.clone(),
                                    line: (line_num + 1) as u32,
                                    block_type: bt.clone(),
                                    property: prop_name.to_string(),
                                    value: prop_value.to_string(),
                                    message: rule.message.clone(),
                                    suggestion: rule.suggestion.clone(),
                                    severity: rule.severity.clone(),
                                });
                            }
                        }
                    }
                }

                // Reset block when we exit (but not if still waiting for opening brace)
                if brace_depth <= 1 && !waiting_for_brace {
                    current_block_type = None;
                }
            }
        }
    }

    ScriptCheckReport {
        mod_id: mod_id.to_string(),
        mod_name: mod_name.to_string(),
        files_scanned,
        total_issues: issues.len() as u32,
        issues,
    }
}
