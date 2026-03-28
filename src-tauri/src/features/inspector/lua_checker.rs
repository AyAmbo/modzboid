use std::path::Path;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaIssue {
    pub file: String,
    pub line: Option<u32>,
    pub severity: String, // "error", "warning", "info"
    pub category: String, // "syntax", "encoding", "deprecated", "compat", "quality"
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaCheckReport {
    pub files_checked: u32,
    pub files_with_issues: u32,
    pub issues: Vec<LuaIssue>,
    pub summary: LuaCheckSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaCheckSummary {
    pub errors: u32,
    pub warnings: u32,
    pub info: u32,
}

/// A deprecation rule — either hardcoded or loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeprecationRule {
    pattern: String,
    message: String,
    suggestion: String,
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default)]
    source: String,
}

fn default_severity() -> String { "warning".to_string() }

/// Load deprecation rules from JSON file, falling back to built-in defaults.
fn load_deprecation_rules(app_data_dir: Option<&Path>) -> Vec<DeprecationRule> {
    // Try loading from app data directory
    if let Some(dir) = app_data_dir {
        let rules_path = dir.join("deprecation-rules.json");
        if rules_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&rules_path) {
                if let Ok(rules) = serde_json::from_str::<Vec<DeprecationRule>>(&content) {
                    if !rules.is_empty() {
                        return rules;
                    }
                }
            }
        }
    }

    // Fall back to built-in defaults
    default_deprecation_rules()
}

fn default_deprecation_rules() -> Vec<DeprecationRule> {
    vec![
        DeprecationRule {
            pattern: r"getSpecificPlayer\s*\(\s*0\s*\)".into(),
            message: "getSpecificPlayer(0) is unreliable in multiplayer".into(),
            suggestion: "Use the player parameter passed to event callbacks instead".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"ISInventoryPane\s*:\s*new\s*\(".into(),
            message: "ISInventoryPane constructor changed in B42".into(),
            suggestion: "Check B42 API for updated ISInventoryPane parameters".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"getSandboxOptions\s*\(\s*\)\s*:\s*getOptionByName".into(),
            message: "SandboxOptions API changed in B42".into(),
            suggestion: "Use SandboxVars direct access instead".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r#"instanceof\s*\(\s*["']IsoGameCharacter["']\s*\)"#.into(),
            message: "instanceof check may fail in B42 due to class hierarchy changes".into(),
            suggestion: "Use duck typing or updated type checks".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"getCell\s*\(\s*\)\s*:\s*getGridSquare\s*\(".into(),
            message: "Cell:getGridSquare coordinates changed in B42".into(),
            suggestion: "Verify coordinate system matches B42 world grid".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"ISContextMenu\.addOption".into(),
            message: "ISContextMenu.addOption signature may differ in B42".into(),
            suggestion: "Verify parameter order matches B42 context menu API".into(),
            severity: "info".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"ReloadManager\s*[:\.]".into(),
            message: "ReloadManager was removed/reworked in B42".into(),
            suggestion: "Use B42 weapon reload system".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"setOutlineHighlight".into(),
            message: "setOutlineHighlight rendering changed in B42".into(),
            suggestion: "Check B42 rendering API for outline highlighting".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"getVehicleList\s*\(\s*\)".into(),
            message: "Vehicle list API changed in B42".into(),
            suggestion: "Use updated vehicle enumeration in B42".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
        DeprecationRule {
            pattern: r"doWalkToward".into(),
            message: "doWalkToward pathfinding changed significantly in B42".into(),
            suggestion: "Use B42 pathfinding API with updated parameters".into(),
            severity: "warning".into(), source: "builtin".into(),
        },
    ]
}

/// Check all Lua files in a mod's media directory.
pub fn check_lua_files(source_path: &Path) -> LuaCheckReport {
    check_lua_files_with_rules(source_path, None)
}

/// Check all Lua files with custom deprecation rules from app data.
pub fn check_lua_files_with_rules(source_path: &Path, app_data_dir: Option<&Path>) -> LuaCheckReport {
    let rules = load_deprecation_rules(app_data_dir);
    let media_path = source_path.join("media");
    let lua_dir = media_path.join("lua");

    let mut issues = Vec::new();
    let mut files_checked = 0u32;
    let mut files_with_issues = 0u32;

    // Also check media/scripts for .txt script files (not Lua, but PZ scripts)
    let dirs_to_scan: Vec<&Path> = if lua_dir.exists() {
        vec![&lua_dir]
    } else if media_path.exists() {
        vec![&media_path]
    } else {
        return empty_report();
    };

    for scan_dir in dirs_to_scan {
        for entry in walkdir::WalkDir::new(scan_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "lua" {
                continue;
            }

            files_checked += 1;
            let rel_path = path.strip_prefix(source_path).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy().to_string();

            let file_issues = check_single_lua_file(path, &rel_str, &rules);
            if !file_issues.is_empty() {
                files_with_issues += 1;
                issues.extend(file_issues);
            }
        }
    }

    let errors = issues.iter().filter(|i| i.severity == "error").count() as u32;
    let warnings = issues.iter().filter(|i| i.severity == "warning").count() as u32;
    let info = issues.iter().filter(|i| i.severity == "info").count() as u32;

    LuaCheckReport {
        files_checked,
        files_with_issues,
        issues,
        summary: LuaCheckSummary { errors, warnings, info },
    }
}

fn check_single_lua_file(path: &Path, rel_path: &str, rules: &[DeprecationRule]) -> Vec<LuaIssue> {
    let mut issues = Vec::new();

    // 1. Encoding check
    let raw_bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return issues,
    };

    let content = match std::str::from_utf8(&raw_bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            issues.push(LuaIssue {
                file: rel_path.to_string(),
                line: None,
                severity: "warning".into(),
                category: "encoding".into(),
                message: format!("File is not valid UTF-8 (error at byte {})", e.valid_up_to()),
                suggestion: Some("Re-save the file as UTF-8. Non-UTF8 can cause crashes on some systems.".into()),
            });
            // Try lossy decode for further analysis
            String::from_utf8_lossy(&raw_bytes).to_string()
        }
    };

    // 2. Syntax check with full_moon
    if let Err(parse_errors) = full_moon::parse(&content) {
        for err in parse_errors {
            let line = extract_line_from_error(&err, &content);
            issues.push(LuaIssue {
                file: rel_path.to_string(),
                line,
                severity: "error".into(),
                category: "syntax".into(),
                message: format!("Lua syntax error: {}", err),
                suggestion: Some("Fix the syntax error — this file will fail to load in PZ.".into()),
            });
        }
    }

    // 3. Deprecated API checks (from rules — either loaded from JSON or built-in)
    for rule in rules {
        if let Ok(re) = Regex::new(&rule.pattern) {
            for mat in re.find_iter(&content) {
                let line = content[..mat.start()].lines().count() as u32;
                issues.push(LuaIssue {
                    file: rel_path.to_string(),
                    line: Some(line),
                    severity: rule.severity.clone(),
                    category: "deprecated".into(),
                    message: rule.message.clone(),
                    suggestion: Some(rule.suggestion.clone()),
                });
            }
        }
    }

    // 4. Quality: global variable pollution (only top-level assignments, not inside functions)
    // Simple heuristic: lines starting with an identifier = value (not local, not inside a function/if block)
    let mut depth = 0i32;
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Track block depth (rough heuristic)
        if trimmed.starts_with("function ") || trimmed.starts_with("if ") || trimmed.starts_with("for ") || trimmed.starts_with("while ") {
            depth += 1;
        }
        if trimmed == "end" || trimmed.starts_with("end ") || trimmed.starts_with("end)") || trimmed.starts_with("end,") {
            depth -= 1;
            if depth < 0 { depth = 0; }
        }

        // Only flag top-level global assignments
        if depth == 0 && !trimmed.is_empty() && !trimmed.starts_with("--") && !trimmed.starts_with("local ") && !trimmed.starts_with("require") && !trimmed.starts_with("return") {
            // Check for pattern: identifier = something (not a function call or table access)
            if let Ok(re) = Regex::new(r"^[a-zA-Z_]\w*\s*=\s*[^=]") {
                if re.is_match(trimmed) {
                    // Skip common PZ patterns that are intentionally global
                    let var_name = trimmed.split('=').next().unwrap_or("").trim();
                    if !is_known_pz_global(var_name) {
                        issues.push(LuaIssue {
                            file: rel_path.to_string(),
                            line: Some((line_num + 1) as u32),
                            severity: "info".into(),
                            category: "quality".into(),
                            message: format!("Global variable '{}' — could conflict with other mods", var_name),
                            suggestion: Some("Consider using 'local' or wrapping in a mod-specific table".into()),
                        });
                    }
                }
            }
        }
    }

    issues
}

/// Check if a variable name is a known PZ global that mods are expected to set.
fn is_known_pz_global(name: &str) -> bool {
    // PZ event hooks, common mod registration patterns
    matches!(name,
        "Events" | "LuaEventManager" | "ISContextMenu" | "ISPanel" |
        "ISButton" | "ISLabel" | "ISTextEntryBox" | "ISRichTextPanel" |
        "ISScrollingListBox" | "ISTickBox" | "ISComboBox" | "ISImage" |
        "ISUIElement" | "ISCollapsableWindow" | "ISModalDialog" |
        "ISInventoryPane" | "ISInventoryPage" | "ISToolTip" |
        "VEHICLE_PART" | "SandboxVars" | "getText" | "getTexture" |
        "require" | "print" | "tostring" | "tonumber" | "type" |
        "pairs" | "ipairs" | "table" | "string" | "math" | "os" | "io"
    )
}

fn extract_line_from_error(err: &full_moon::Error, _content: &str) -> Option<u32> {
    // full_moon errors contain position info in their Display output
    let err_str = format!("{}", err);
    // Try to extract line number from error message (format varies)
    if let Some(pos) = err_str.find("line ") {
        let rest = &err_str[pos + 5..];
        if let Some(end) = rest.find(|c: char| !c.is_ascii_digit()) {
            if let Ok(line) = rest[..end].parse::<u32>() {
                return Some(line);
            }
        }
    }
    None
}

fn empty_report() -> LuaCheckReport {
    LuaCheckReport {
        files_checked: 0,
        files_with_issues: 0,
        issues: vec![],
        summary: LuaCheckSummary { errors: 0, warnings: 0, info: 0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_valid_lua() {
        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("media").join("lua").join("client");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let mut f = std::fs::File::create(lua_dir.join("test.lua")).unwrap();
        writeln!(f, "local function hello()").unwrap();
        writeln!(f, "  print('hello')").unwrap();
        writeln!(f, "end").unwrap();

        let report = check_lua_files(dir.path());
        assert_eq!(report.files_checked, 1);
        assert_eq!(report.summary.errors, 0);
    }

    #[test]
    fn test_syntax_error() {
        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("media").join("lua");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let mut f = std::fs::File::create(lua_dir.join("broken.lua")).unwrap();
        writeln!(f, "function broken(").unwrap();
        writeln!(f, "  -- missing closing paren and end").unwrap();

        let report = check_lua_files(dir.path());
        assert_eq!(report.files_checked, 1);
        assert!(report.summary.errors > 0);
        assert!(report.issues.iter().any(|i| i.category == "syntax"));
    }

    #[test]
    fn test_deprecated_api() {
        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("media").join("lua");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let mut f = std::fs::File::create(lua_dir.join("old_api.lua")).unwrap();
        writeln!(f, "local player = getSpecificPlayer(0)").unwrap();

        let report = check_lua_files(dir.path());
        assert!(report.issues.iter().any(|i| i.category == "deprecated"));
    }

    #[test]
    fn test_encoding_issue() {
        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("media").join("lua");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let path = lua_dir.join("bad_encoding.lua");
        // Write invalid UTF-8
        std::fs::write(&path, b"local x = 'hello \xff world'").unwrap();

        let report = check_lua_files(dir.path());
        assert!(report.issues.iter().any(|i| i.category == "encoding"));
    }

    #[test]
    fn test_no_lua_files() {
        let dir = tempfile::tempdir().unwrap();
        let report = check_lua_files(dir.path());
        assert_eq!(report.files_checked, 0);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_global_variable_detection() {
        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("media").join("lua");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let mut f = std::fs::File::create(lua_dir.join("globals.lua")).unwrap();
        writeln!(f, "MyMod = {{}}").unwrap();
        writeln!(f, "MyMod.value = 42").unwrap();

        let report = check_lua_files(dir.path());
        assert!(report.issues.iter().any(|i| i.category == "quality" && i.message.contains("MyMod")));
    }
}
