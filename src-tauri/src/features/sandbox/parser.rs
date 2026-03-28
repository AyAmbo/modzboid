use std::path::{Path, PathBuf};
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::app_core::error::AppError;

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxSettingType {
    Int,
    Float,
    Bool,
    String,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SandboxValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl std::fmt::Display for SandboxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxValue::Bool(b) => write!(f, "{}", b),
            SandboxValue::Int(i) => write!(f, "{}", i),
            SandboxValue::Float(v) => {
                // Preserve at least one decimal place for floats
                if *v == v.floor() {
                    write!(f, "{:.1}", v)
                } else {
                    // Remove trailing zeros but keep at least one decimal
                    let s = format!("{}", v);
                    if s.contains('.') {
                        write!(f, "{}", s)
                    } else {
                        write!(f, "{:.1}", v)
                    }
                }
            }
            SandboxValue::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumOption {
    pub value: i64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSetting {
    pub key: String,
    pub value: SandboxValue,
    pub setting_type: SandboxSettingType,
    pub description: Option<String>,
    pub enum_options: Vec<EnumOption>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCategory {
    pub name: String,
    pub settings: Vec<SandboxSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxVarsConfig {
    pub path: PathBuf,
    pub settings: Vec<SandboxSetting>,
    pub categories: Vec<SandboxCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSettingUpdate {
    pub key: String,
    pub category: Option<String>,
    pub value: SandboxValue,
}

// ── Comment metadata parsing ───────────────────────────────────────────────────

#[derive(Debug, Default)]
struct CommentMetadata {
    description: Option<String>,
    enum_options: Vec<EnumOption>,
    min: Option<f64>,
    max: Option<f64>,
    default_value: Option<String>,
}

fn parse_comment_metadata(comment_lines: &[String]) -> CommentMetadata {
    let mut meta = CommentMetadata::default();
    let mut description_parts: Vec<String> = Vec::new();

    let enum_re = Regex::new(r"^--\s*(-?\d+)\s*=\s*(.+)$").unwrap();
    let min_max_default_re = Regex::new(
        r"(?i)Min:\s*(-?[\d.]+)\s+Max:\s*(-?[\d.]+)\s+Default:\s*(.+?)$"
    ).unwrap();
    let default_only_re = Regex::new(r"(?i)Default\s*=\s*(.+?)$").unwrap();

    for line in comment_lines {
        let trimmed = line.trim();

        // Check for enum option: -- N = Label
        if let Some(caps) = enum_re.captures(trimmed) {
            let value: i64 = caps[1].parse().unwrap_or(0);
            let label = caps[2].trim().to_string();
            meta.enum_options.push(EnumOption { value, label });
            continue;
        }

        // Extract the comment text (strip the --)
        let comment_text = trimmed.strip_prefix("--").unwrap_or(trimmed).trim();

        // Check for Min/Max/Default pattern
        if let Some(caps) = min_max_default_re.captures(comment_text) {
            meta.min = caps[1].parse().ok();
            meta.max = caps[2].parse().ok();
            let default_str = caps[3].trim().to_string();
            meta.default_value = Some(default_str.clone());

            // The text before Min: is the description
            if let Some(idx) = comment_text.find("Min:") {
                let desc = comment_text[..idx].trim();
                if !desc.is_empty() {
                    description_parts.push(desc.to_string());
                }
            }
            continue;
        }

        // Check for Default = ... pattern (in description line)
        if let Some(caps) = default_only_re.captures(comment_text) {
            let default_str = caps[1].trim().to_string();
            meta.default_value = Some(default_str);

            // Text before "Default" is the description
            if let Some(idx) = comment_text.find("Default") {
                let desc = comment_text[..idx].trim().trim_end_matches('.');
                if !desc.is_empty() {
                    description_parts.push(desc.to_string());
                }
            }
            continue;
        }

        // Regular description line
        if !comment_text.is_empty() {
            description_parts.push(comment_text.to_string());
        }
    }

    if !description_parts.is_empty() {
        meta.description = Some(description_parts.join(" "));
    }

    meta
}

// ── Value parsing ──────────────────────────────────────────────────────────────

fn parse_value(raw: &str) -> SandboxValue {
    let val = raw.trim().trim_end_matches(',');

    if val == "true" {
        return SandboxValue::Bool(true);
    }
    if val == "false" {
        return SandboxValue::Bool(false);
    }

    // Quoted string
    if val.starts_with('"') && val.ends_with('"') {
        return SandboxValue::String(val[1..val.len() - 1].to_string());
    }

    // Float (contains a dot)
    if val.contains('.') {
        if let Ok(f) = val.parse::<f64>() {
            return SandboxValue::Float(f);
        }
    }

    // Integer
    if let Ok(i) = val.parse::<i64>() {
        return SandboxValue::Int(i);
    }

    // Fallback: treat as string
    SandboxValue::String(val.to_string())
}

fn determine_type(value: &SandboxValue, enum_options: &[EnumOption]) -> SandboxSettingType {
    if !enum_options.is_empty() {
        return SandboxSettingType::Enum;
    }
    match value {
        SandboxValue::Bool(_) => SandboxSettingType::Bool,
        SandboxValue::Int(_) => SandboxSettingType::Int,
        SandboxValue::Float(_) => SandboxSettingType::Float,
        SandboxValue::String(_) => SandboxSettingType::String,
    }
}

// ── Main parser ────────────────────────────────────────────────────────────────

pub fn parse_sandbox_vars(path: &Path) -> Result<SandboxVarsConfig, AppError> {
    let content = std::fs::read_to_string(path)?;
    parse_sandbox_vars_from_str(&content, path)
}

fn parse_sandbox_vars_from_str(content: &str, path: &Path) -> Result<SandboxVarsConfig, AppError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut settings: Vec<SandboxSetting> = Vec::new();
    let mut categories: Vec<SandboxCategory> = Vec::new();
    let mut accumulated_comments: Vec<String> = Vec::new();

    // Regex for key = value lines
    let kv_re = Regex::new(r#"^\s+(\w+)\s*=\s*(.+?)\s*,?\s*$"#).unwrap();
    // Regex for table open: TableName = {
    let table_open_re = Regex::new(r"^\s+(\w+)\s*=\s*\{").unwrap();
    // Regex for table close: },
    let table_close_re = Regex::new(r"^\s+\},?\s*$").unwrap();
    // Regex for comment lines
    let comment_re = Regex::new(r"^\s+--").unwrap();

    #[derive(PartialEq)]
    enum State {
        LookingForRoot,
        InRoot,
        InCategory(String),
    }

    let mut state = State::LookingForRoot;

    let mut category_settings: Vec<SandboxSetting> = Vec::new();

    for line in &lines {
        match &state {
            State::LookingForRoot => {
                if line.trim().starts_with("SandboxVars") && line.contains('{') {
                    state = State::InRoot;
                }
            }
            State::InRoot => {
                let trimmed = line.trim();

                // End of root table
                if trimmed == "}" {
                    break;
                }

                // Comment line
                if comment_re.is_match(line) {
                    // Extract just the comment part (handle inline comments on table-open lines, etc.)
                    accumulated_comments.push(trimmed.to_string());
                    continue;
                }

                // Table open: Category = {
                if let Some(caps) = table_open_re.captures(line) {
                    let name = caps[1].to_string();
                    // Any accumulated comments before the table open are category-level (we discard or associate)
                    accumulated_comments.clear();
                    category_settings.clear();
                    state = State::InCategory(name);
                    continue;
                }

                // Key = value (top-level setting)
                if let Some(caps) = kv_re.captures(line) {
                    let key = caps[1].to_string();
                    let raw_value = caps[2].to_string();
                    let value = parse_value(&raw_value);
                    let meta = parse_comment_metadata(&accumulated_comments);
                    let setting_type = determine_type(&value, &meta.enum_options);

                    settings.push(SandboxSetting {
                        key,
                        value,
                        setting_type,
                        description: meta.description,
                        enum_options: meta.enum_options,
                        min: meta.min,
                        max: meta.max,
                        default_value: meta.default_value,
                    });
                    accumulated_comments.clear();
                }
            }
            State::InCategory(cat_name) => {
                let trimmed = line.trim();

                // Table close
                if table_close_re.is_match(line) {
                    categories.push(SandboxCategory {
                        name: cat_name.clone(),
                        settings: std::mem::take(&mut category_settings),
                    });
                    accumulated_comments.clear();
                    state = State::InRoot;
                    continue;
                }

                // Comment line
                if comment_re.is_match(line) {
                    accumulated_comments.push(trimmed.to_string());
                    continue;
                }

                // Key = value (within category)
                if let Some(caps) = kv_re.captures(line) {
                    let key = caps[1].to_string();
                    let raw_value = caps[2].to_string();
                    let value = parse_value(&raw_value);
                    let meta = parse_comment_metadata(&accumulated_comments);
                    let setting_type = determine_type(&value, &meta.enum_options);

                    category_settings.push(SandboxSetting {
                        key,
                        value,
                        setting_type,
                        description: meta.description,
                        enum_options: meta.enum_options,
                        min: meta.min,
                        max: meta.max,
                        default_value: meta.default_value,
                    });
                    accumulated_comments.clear();
                }
            }
        }
    }

    Ok(SandboxVarsConfig {
        path: path.to_path_buf(),
        settings,
        categories,
    })
}

// ── Save logic ─────────────────────────────────────────────────────────────────

pub fn save_sandbox_vars(path: &Path, updates: &[SandboxSettingUpdate]) -> Result<(), AppError> {
    let content = std::fs::read_to_string(path)?;
    let result = apply_updates(&content, updates)?;
    std::fs::write(path, result)?;
    Ok(())
}

fn apply_updates(content: &str, updates: &[SandboxSettingUpdate]) -> Result<String, AppError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut output: Vec<String> = Vec::with_capacity(lines.len());

    let kv_re = Regex::new(r#"^(\s+)(\w+)(\s*=\s*).+?(,?\s*)$"#).unwrap();
    let table_open_re = Regex::new(r"^\s+(\w+)\s*=\s*\{").unwrap();
    let table_close_re = Regex::new(r"^\s+\},?\s*$").unwrap();

    let mut current_category: Option<String> = None;
    let mut in_root = false;

    for line in &lines {
        // Track state
        if line.contains("SandboxVars") && line.contains('{') {
            in_root = true;
            output.push(line.to_string());
            continue;
        }

        if !in_root {
            output.push(line.to_string());
            continue;
        }

        if line.trim() == "}" && current_category.is_none() {
            // End of root
            in_root = false;
            output.push(line.to_string());
            continue;
        }

        if table_close_re.is_match(line) && current_category.is_some() {
            current_category = None;
            output.push(line.to_string());
            continue;
        }

        if let Some(caps) = table_open_re.captures(line) {
            current_category = Some(caps[1].to_string());
            output.push(line.to_string());
            continue;
        }

        // Try to match key = value line and apply updates
        if let Some(caps) = kv_re.captures(line) {
            let indent = &caps[1];
            let key = &caps[2];
            let eq = &caps[3];
            let trailing = &caps[4];

            // Check if there's an update for this key in the current context
            if let Some(update) = updates.iter().find(|u| {
                u.key == key && u.category == current_category
            }) {
                let new_line = format!(
                    "{}{}{}{}{}",
                    indent,
                    key,
                    eq,
                    update.value,
                    trailing
                );
                output.push(new_line);
                continue;
            }
        }

        output.push(line.to_string());
    }

    // Preserve trailing newline if present
    let mut result = output.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

