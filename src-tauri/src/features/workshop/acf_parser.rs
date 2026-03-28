use std::path::Path;
use crate::app_core::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemInfo {
    pub workshop_id: String,
    pub size: u64,
    pub time_updated: u64,
}

/// Extract all quoted strings from a VDF line.
/// For `"key"  "value"` returns `["key", "value"]`.
/// For `"key"` returns `["key"]`.
fn extract_quoted_strings(line: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('"') {
            result.push(&rest[..end]);
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    result
}

/// Parse the appworkshop_108600.acf file to extract workshop item details.
pub fn parse_workshop_acf(path: &Path) -> Result<Vec<WorkshopItemInfo>, AppError> {
    let content = std::fs::read_to_string(path)?;
    let mut items = Vec::new();

    // State machine for parsing the VDF structure
    let mut in_items_block = false;
    let mut in_item = false;
    let mut current_id = String::new();
    let mut current_size: u64 = 0;
    let mut current_time: u64 = 0;
    let mut brace_depth: i32 = 0;
    let mut items_block_depth: i32 = 0;
    let mut item_depth: i32 = 0;
    // Track last seen key so we can associate the next '{' with it
    let mut pending_key = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            brace_depth += 1;
            if !pending_key.is_empty() {
                if pending_key == "WorkshopItemsInstalled" {
                    in_items_block = true;
                    items_block_depth = brace_depth;
                } else if in_items_block && !in_item
                    && pending_key.chars().all(|c| c.is_ascii_digit())
                {
                    in_item = true;
                    current_id = pending_key.clone();
                    item_depth = brace_depth;
                }
                pending_key.clear();
            }
            continue;
        }
        if trimmed == "}" {
            if in_item && brace_depth == item_depth {
                items.push(WorkshopItemInfo {
                    workshop_id: current_id.clone(),
                    size: current_size,
                    time_updated: current_time,
                });
                in_item = false;
                current_id.clear();
                current_size = 0;
                current_time = 0;
            } else if in_items_block && brace_depth == items_block_depth {
                in_items_block = false;
            }
            brace_depth -= 1;
            continue;
        }

        let parts = extract_quoted_strings(trimmed);

        if parts.len() == 1 {
            // Standalone key — next line should be '{'
            pending_key = parts[0].to_string();
        } else if parts.len() >= 2 && in_item {
            // Key-value pair inside an item block
            match parts[0] {
                "size" => current_size = parts[1].parse().unwrap_or(0),
                "timeupdated" => current_time = parts[1].parse().unwrap_or(0),
                _ => {}
            }
        } else if parts.len() == 1 {
            pending_key = parts[0].to_string();
        }
    }

    Ok(items)
}

/// Find the ACF manifest path from the steam path.
pub fn find_acf_path(steam_path: &Path) -> Option<std::path::PathBuf> {
    // Try common locations
    let candidates = [
        steam_path.join("steamapps/workshop/appworkshop_108600.acf"),
        steam_path.join("workshop/appworkshop_108600.acf"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

