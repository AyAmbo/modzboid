use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::app_core::error::AppError;
use super::categories;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SettingType {
    Bool,
    Int,
    Float,
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSetting {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub setting_type: SettingType,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default_value: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub name: String,
    pub path: String,
    pub settings: Vec<ServerSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfigInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettingUpdate {
    pub key: String,
    pub value: String,
}

/// Parse comment text to extract description, min, max, default.
fn parse_comment_metadata(comments: &[String]) -> (Option<String>, Option<f64>, Option<f64>, Option<String>) {
    let full_text = comments.iter()
        .map(|c| c.trim_start_matches('#').trim())
        .collect::<Vec<_>>()
        .join(" ");

    if full_text.is_empty() {
        return (None, None, None, None);
    }

    let mut description = full_text.clone();
    let mut min = None;
    let mut max = None;
    let mut default = None;

    // Extract "Min: N"
    if let Some(min_idx) = full_text.find("Min:") {
        let after = &full_text[min_idx + 4..].trim_start();
        if let Some(val) = after.split_whitespace().next() {
            min = val.parse::<f64>().ok();
        }
    }
    // Extract "Max: N"
    if let Some(max_idx) = full_text.find("Max:") {
        let after = &full_text[max_idx + 4..].trim_start();
        if let Some(val) = after.split_whitespace().next() {
            max = val.parse::<f64>().ok();
        }
    }
    // Extract "Default: N"
    if let Some(def_idx) = full_text.find("Default:") {
        let after = &full_text[def_idx + 8..].trim_start();
        if let Some(val) = after.split_whitespace().next() {
            default = Some(val.to_string());
        }
    }

    // Clean description: remove Min/Max/Default parts
    if let Some(min_idx) = description.find("Min:") {
        description = description[..min_idx].trim().to_string();
    }

    if description.is_empty() {
        return (None, min, max, default);
    }

    (Some(description), min, max, default)
}

/// Infer setting type from value and metadata.
fn infer_type(value: &str, min: Option<f64>, max: Option<f64>, default: &Option<String>) -> SettingType {
    if value == "true" || value == "false" {
        return SettingType::Bool;
    }
    if let Some(ref def) = default {
        if def == "true" || def == "false" {
            return SettingType::Bool;
        }
    }
    if min.is_some() || max.is_some() {
        // Check if value contains a decimal point
        if value.contains('.') {
            return SettingType::Float;
        }
        if let Some(ref def) = default {
            if def.contains('.') {
                return SettingType::Float;
            }
        }
        return SettingType::Int;
    }
    if value.parse::<i64>().is_ok() {
        return SettingType::Int;
    }
    SettingType::String
}

/// Parse a server.ini file into a ServerConfig.
pub fn parse_server_ini(path: &Path) -> Result<ServerConfig, AppError> {
    let content = std::fs::read_to_string(path)?;
    let name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut settings = Vec::new();
    let mut pending_comments: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !pending_comments.is_empty() {
                // Keep accumulating — empty line between comments and setting
            }
            continue;
        }
        if trimmed.starts_with('#') {
            pending_comments.push(trimmed.to_string());
            continue;
        }

        // Parse key=value
        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim().to_string();
            let value = trimmed[eq_idx + 1..].to_string();

            let (description, min, max, default) = parse_comment_metadata(&pending_comments);
            let setting_type = infer_type(&value, min, max, &default);
            let category = categories::get_category(&key).to_string();

            settings.push(ServerSetting {
                key,
                value,
                description,
                setting_type,
                min,
                max,
                default_value: default,
                category,
            });

            pending_comments.clear();
        }
    }

    Ok(ServerConfig {
        name,
        path: path.to_string_lossy().into_owned(),
        settings,
    })
}

/// Save updated settings to a server.ini file, preserving comments and format.
pub fn save_server_ini(path: &Path, updates: &[ServerSettingUpdate]) -> Result<(), AppError> {
    let content = std::fs::read_to_string(path)?;
    let update_map: std::collections::HashMap<&str, &str> = updates
        .iter()
        .map(|u| (u.key.as_str(), u.value.as_str()))
        .collect();

    let mut output = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(eq_idx) = trimmed.find('=') {
            if !trimmed.starts_with('#') {
                let key = trimmed[..eq_idx].trim();
                if let Some(&new_value) = update_map.get(key) {
                    output.push_str(&format!("{}={}\n", key, new_value));
                    continue;
                }
            }
        }
        output.push_str(line);
        output.push('\n');
    }

    std::fs::write(path, output)?;
    Ok(())
}

/// List all server.ini files in the Zomboid/Server/ directory.
pub fn list_server_configs(zomboid_user_dir: &Path) -> Vec<ServerConfigInfo> {
    let server_dir = zomboid_user_dir.join("Server");
    if !server_dir.exists() {
        return vec![];
    }

    let mut configs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&server_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "ini").unwrap_or(false) {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                configs.push(ServerConfigInfo {
                    name,
                    path: path.to_string_lossy().into_owned(),
                });
            }
        }
    }
    configs
}

