use std::collections::{HashMap, HashSet};

use tauri::State;

use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::app_core::types::{DepResolution, LoadOrderIssue, ModCategory};
use crate::features::discovery::cache;

use super::community_rules::{self, CommunityRulesDb};
use super::deps;
use super::rules;

// ─── Category helpers ──────────────────────────────────────────────────────────

/// Parse a raw category string into a `ModCategory`.
/// Matches the values stored/recognised by `cache.rs`.
fn parse_category_str(s: &str) -> Option<ModCategory> {
    match s.to_lowercase().as_str() {
        "framework" => Some(ModCategory::Framework),
        "map" => Some(ModCategory::Map),
        "content" => Some(ModCategory::Content),
        "overhaul" => Some(ModCategory::Overhaul),
        _ => None,
    }
}

// ─── Tauri commands ────────────────────────────────────────────────────────────

/// Sort a list of active mod IDs using tier-based topological ordering.
/// Reads dependency and category metadata from the SQLite cache.
#[tauri::command]
pub async fn sort_load_order_cmd(
    mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;

    let mut dependencies: HashMap<String, Vec<String>> = all_mods
        .iter()
        .map(|m| (m.id.clone(), m.requires.clone()))
        .collect();

    // Merge community rules into dependencies
    let community_db = community_rules::load_community_rules(&state.app_data_dir);
    let community_edges = community_db.to_dependency_edges();
    for (mod_id, extra_deps) in community_edges {
        dependencies
            .entry(mod_id)
            .or_default()
            .extend(extra_deps);
    }

    // Prefer detected_category (auto-classified) then fall back to the raw category string
    let categories: HashMap<String, Option<ModCategory>> = all_mods
        .iter()
        .map(|m| {
            let cat = m
                .detected_category
                .clone()
                .or_else(|| m.category.as_deref().and_then(parse_category_str));
            (m.id.clone(), cat)
        })
        .collect();

    rules::sort_with_tiers(&mod_ids, &categories, &dependencies)
}

/// Validate an existing load order and return any detected issues.
/// Reads dependency metadata from the SQLite cache.
#[tauri::command]
pub async fn validate_load_order_cmd(
    load_order: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LoadOrderIssue>, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;

    let mut dependencies: HashMap<String, Vec<String>> = all_mods
        .iter()
        .map(|m| (m.id.clone(), m.requires.clone()))
        .collect();

    // Merge community rules into dependencies
    let community_db = community_rules::load_community_rules(&state.app_data_dir);
    let community_edges = community_db.to_dependency_edges();
    for (mod_id, extra_deps) in community_edges {
        dependencies
            .entry(mod_id)
            .or_default()
            .extend(extra_deps);
    }

    let all_known: HashSet<String> = all_mods.iter().map(|m| m.id.clone()).collect();

    Ok(rules::validate_load_order(&load_order, &dependencies, &all_known))
}

/// Resolve all transitive dependencies for a mod that aren't yet enabled.
#[tauri::command]
pub async fn auto_resolve_deps_cmd(
    mod_id: String,
    enabled_mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<DepResolution, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;

    let dependencies: HashMap<String, Vec<String>> = all_mods
        .iter()
        .map(|m| (m.id.clone(), m.requires.clone()))
        .collect();

    let enabled_set: HashSet<String> = enabled_mod_ids.into_iter().collect();
    let all_known: HashSet<String> = all_mods.iter().map(|m| m.id.clone()).collect();

    Ok(deps::resolve_transitive_deps(
        &mod_id,
        &dependencies,
        &enabled_set,
        &all_known,
    ))
}

/// Find all enabled mods that transitively depend on the given mod.
#[tauri::command]
pub async fn reverse_deps_cmd(
    mod_id: String,
    enabled_mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let all_mods = cache::get_cached_mods(&state.db).await?;

    let dependencies: HashMap<String, Vec<String>> = all_mods
        .iter()
        .map(|m| (m.id.clone(), m.requires.clone()))
        .collect();

    let reverse_map = deps::build_reverse_dep_map(&dependencies);
    let enabled_set: HashSet<String> = enabled_mod_ids.into_iter().collect();

    Ok(deps::find_reverse_deps(&mod_id, &reverse_map, &enabled_set))
}

// ─── Community rules commands ─────────────────────────────────────────────────

/// Get the current community rules database.
#[tauri::command]
pub async fn get_community_rules_cmd(
    state: State<'_, AppState>,
) -> Result<CommunityRulesDb, AppError> {
    Ok(community_rules::load_community_rules(&state.app_data_dir))
}

/// Save the community rules database.
#[tauri::command]
pub async fn save_community_rules_cmd(
    rules: CommunityRulesDb,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    community_rules::save_community_rules(&state.app_data_dir, &rules)
}
