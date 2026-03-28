use tauri::State;
use crate::app_core::config::AppState;
use crate::app_core::error::AppError;
use crate::features::discovery::cache;
use crate::features::extensions::loader;
use super::validator::{self, InspectionReport};
use super::lua_checker::{self, LuaCheckReport};
use super::migration::{self, MigrationReport, DeprecationRule};
use super::script_checker::{self, ScriptPropertyRule, ScriptCheckReport};
use super::auto_fixer;
use super::item_references;

#[tauri::command]
pub async fn inspect_mod_cmd(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<InspectionReport, AppError> {
    let mod_info = cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))?;

    Ok(validator::inspect_mod(&mod_info))
}

/// Run Lua file checks on a mod — syntax, encoding, deprecated APIs, quality.
#[tauri::command]
pub async fn check_mod_lua_cmd(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<LuaCheckReport, AppError> {
    let mod_info = cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))?;

    Ok(lua_checker::check_lua_files_with_rules(
        &mod_info.source_path,
        Some(&state.app_data_dir),
    ))
}

/// Pick the best rules version based on a mod's versionMin.
/// Returns (from_version, to_version) for the rules to load.
fn pick_rules_version(version_min: Option<&str>, versions: &loader::MigrationVersionsData) -> Option<(String, String)> {
    if versions.versions.is_empty() {
        return None;
    }

    // Parse the mod's versionMin to find the best matching "from" version
    if let Some(vmin) = version_min {
        // Find the closest rules version >= mod's versionMin
        // e.g., mod versionMin=42.13.0 → rules from 42.13.0
        //       mod versionMin=42.14.2 → rules from 42.14.0
        //       mod versionMin=41.0    → rules from 41.78.16
        for v in versions.versions.iter().rev() {
            // Check if the mod's version is >= this rules "from" version
            if version_ge(vmin, &v.from) {
                return Some((v.from.clone(), v.to.clone()));
            }
        }
    }

    // Fallback: use the broadest rules (first entry, usually B41→latest)
    let first = &versions.versions[0];
    Some((first.from.clone(), first.to.clone()))
}

/// Simple version comparison: is `a` >= `b`?
fn version_ge(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let pa = va.get(i).copied().unwrap_or(0);
        let pb = vb.get(i).copied().unwrap_or(0);
        if pa > pb { return true; }
        if pa < pb { return false; }
    }
    true // equal
}

/// Scan a single mod for migration issues. Auto-detects the right rules from the extension.
/// If `force_full` is true, uses the broadest rules (B41→latest) regardless of mod version.
#[tauri::command]
pub async fn scan_mod_migration_cmd(
    mod_id: String,
    force_full: Option<bool>,
    state: State<'_, AppState>,
) -> Result<MigrationReport, AppError> {
    let mod_info = cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))?;

    // Try extension rules first
    let versions_data = loader::load_migration_versions(&state.app_data_dir);
    let version_min = if force_full.unwrap_or(false) {
        None // Force broadest rules
    } else {
        mod_info.version_min.as_deref()
    };
    let rules = if let Some((from, to)) = pick_rules_version(
        version_min,
        &versions_data,
    ) {
        let raw = loader::load_migration_rules(&state.app_data_dir, &from, &to)?;
        raw.iter()
            .filter_map(|v| serde_json::from_value::<DeprecationRule>(v.clone()).ok())
            .collect::<Vec<_>>()
    } else {
        // Fallback: try legacy rules file
        let rules_path = state.app_data_dir.join("deprecation-rules.json");
        migration::load_rules(&rules_path)
    };

    if rules.is_empty() {
        return Err(AppError::NotFound(
            "No migration rules found. Install the pz-migration-rules extension.".into()
        ));
    }

    Ok(migration::scan_mod_migration(
        &mod_info.source_path,
        &mod_info.id,
        &mod_info.name,
        &rules,
        mod_info.active_version_folder.as_deref(),
    ))
}

/// Scan ALL enabled mods for migration issues (batch).
#[tauri::command]
pub async fn scan_all_mods_migration_cmd(
    mod_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<MigrationReport>, AppError> {
    let rules_path = state.app_data_dir.join("deprecation-rules.json");
    let rules = migration::load_rules(&rules_path);

    if rules.is_empty() {
        return Err(AppError::NotFound(
            "No deprecation rules found.".into()
        ));
    }

    let mut reports = Vec::new();
    for mod_id in &mod_ids {
        if let Ok(Some(mod_info)) = cache::get_mod_by_id(&state.db, mod_id).await {
            let report = migration::scan_mod_migration(
                &mod_info.source_path,
                &mod_info.id,
                &mod_info.name,
                &rules,
                mod_info.active_version_folder.as_deref(),
            );
            if report.total_issues > 0 {
                reports.push(report);
            }
        }
    }

    Ok(reports)
}

/// List available migration version transitions from the installed migration-rules extension.
#[tauri::command]
pub async fn list_migration_versions_cmd(
    state: State<'_, AppState>,
) -> Result<loader::MigrationVersionsData, AppError> {
    Ok(loader::load_migration_versions(&state.app_data_dir))
}

/// Scan all mods for compatibility issues. Auto-picks the right rules per mod
/// based on each mod's versionMin and active version folder.
/// When force_full is true, always uses the broadest ruleset (B41→latest).
#[tauri::command]
pub async fn scan_all_mods_compat_cmd(
    force_full: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<CompatReport>, AppError> {
    let force_full = force_full.unwrap_or(false);
    let versions_data = loader::load_migration_versions(&state.app_data_dir);
    if versions_data.versions.is_empty() {
        return Err(AppError::NotFound(
            "No migration rules extension installed.".into(),
        ));
    }

    // Load script property rules
    let script_rules = load_script_rules_from_extension(&state.app_data_dir);

    // Pre-load all rule sets we might need
    let mut rules_cache: std::collections::HashMap<String, Vec<DeprecationRule>> =
        std::collections::HashMap::new();

    let all_mods = cache::get_cached_mods(&state.db).await?;

    // Build item dictionary: base game + all loaded mods
    let config = state.config.read().await;
    let game_path = config.game_path.clone();
    drop(config);

    let mut known_items = if let Some(ref gp) = game_path {
        item_references::build_base_game_dictionary(gp)
    } else {
        std::collections::HashSet::new()
    };
    for mi in &all_mods {
        let mod_items = item_references::build_mod_dictionary(
            &mi.source_path,
            mi.active_version_folder.as_deref(),
        );
        known_items.extend(mod_items);
    }
    let mut reports = Vec::new();

    for mod_info in &all_mods {
        let version_min = if force_full {
            None
        } else {
            mod_info.version_min.as_deref()
        };
        let (from, to) = match pick_rules_version(version_min, &versions_data) {
            Some(v) => v,
            None => continue,
        };

        let cache_key = format!("{}->{}", from, to);
        if !rules_cache.contains_key(&cache_key) {
            if let Ok(raw) = loader::load_migration_rules(&state.app_data_dir, &from, &to) {
                let rules: Vec<DeprecationRule> = raw
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                rules_cache.insert(cache_key.clone(), rules);
            }
        }

        let rules = match rules_cache.get(&cache_key) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };

        let report = migration::scan_mod_migration(
            &mod_info.source_path,
            &mod_info.id,
            &mod_info.name,
            rules,
            mod_info.active_version_folder.as_deref(),
        );

        // Also check script properties
        let script_report = if !script_rules.is_empty() {
            let sr = script_checker::check_script_properties(
                &mod_info.source_path,
                &mod_info.id,
                &mod_info.name,
                &script_rules,
                mod_info.active_version_folder.as_deref(),
            );
            if sr.total_issues > 0 { Some(sr) } else { None }
        } else {
            None
        };
        let script_issues = script_report.as_ref().map_or(0, |r| r.total_issues);

        // Check item references
        let missing_refs = if !known_items.is_empty() {
            item_references::check_mod_references(
                &mod_info.source_path,
                &mod_info.id,
                &mod_info.name,
                mod_info.active_version_folder.as_deref(),
                &known_items,
            )
        } else {
            vec![]
        };
        let missing_ref_count = missing_refs.len() as u32;

        reports.push(CompatReport {
            report,
            detected_version: mod_info.version_min.clone().unwrap_or_else(|| "unknown".into()),
            rules_used: format!("{} → {}", from, to),
            active_folder: mod_info.active_version_folder.clone(),
            script_issues,
            script_report,
            missing_refs,
            missing_ref_count,
        });
    }

    // Sort: most issues first (missing refs are highest priority)
    reports.sort_by(|a, b| {
        let a_total = a.report.total_issues + a.script_issues + a.missing_ref_count;
        let b_total = b.report.total_issues + b.script_issues + b.missing_ref_count;
        b_total.cmp(&a_total)
    });

    Ok(reports)
}

/// Extended report with version metadata.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatReport {
    #[serde(flatten)]
    pub report: MigrationReport,
    pub detected_version: String,
    pub rules_used: String,
    pub active_folder: Option<String>,
    pub script_issues: u32,
    pub script_report: Option<ScriptCheckReport>,
    pub missing_refs: Vec<item_references::MissingItemRef>,
    pub missing_ref_count: u32,
}

/// Scan all mods for script property issues (deprecated .txt script properties).
#[tauri::command]
pub async fn scan_all_scripts_compat_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ScriptCheckReport>, AppError> {
    // Load script property rules from extension
    let rules = load_script_rules_from_extension(&state.app_data_dir);
    if rules.is_empty() {
        return Err(AppError::NotFound(
            "No script property rules found in migration extension.".into(),
        ));
    }

    let all_mods = cache::get_cached_mods(&state.db).await?;
    let mut reports = Vec::new();

    for mod_info in &all_mods {
        let report = script_checker::check_script_properties(
            &mod_info.source_path,
            &mod_info.id,
            &mod_info.name,
            &rules,
            mod_info.active_version_folder.as_deref(),
        );
        if report.total_issues > 0 {
            reports.push(report);
        }
    }

    reports.sort_by(|a, b| b.total_issues.cmp(&a.total_issues));
    Ok(reports)
}

/// Load script property rules from the migration-rules extension.
fn load_script_rules_from_extension(app_data_dir: &std::path::Path) -> Vec<ScriptPropertyRule> {
    let dir = loader::extensions_dir(app_data_dir);
    if !dir.exists() {
        return vec![];
    }

    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if !path.is_dir() || path.join(".disabled").exists() {
            continue;
        }
        let manifest_path = path.join("extension.json");
        if !manifest_path.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
                if manifest.get("type").and_then(|t| t.as_str()) == Some("migration-rules") {
                    if let Some(script_file) = manifest
                        .get("provides")
                        .and_then(|p| p.get("scriptPropertyRules"))
                        .and_then(|s| s.as_str())
                    {
                        let rules_path = path.join(script_file);
                        if let Ok(rules_content) = std::fs::read_to_string(&rules_path) {
                            if let Ok(rules) = serde_json::from_str::<Vec<ScriptPropertyRule>>(&rules_content) {
                                return rules;
                            }
                        }
                    }
                }
            }
        }
    }

    vec![]
}

/// Create a fixed local copy of a mod with B42 compatibility patches applied.
#[tauri::command]
pub async fn auto_fix_mod_cmd(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<auto_fixer::AutoFixReport, AppError> {
    let mod_info = cache::get_mod_by_id(&state.db, &mod_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Mod not found: {}", mod_id)))?;

    let config = state.config.read().await;
    let local_mods_path = config.local_mods_path.clone()
        .ok_or_else(|| AppError::Validation(
            "Local mods path is not configured. Set it in Settings first.".into()
        ))?;
    drop(config);

    auto_fixer::create_fixed_copy(
        &mod_info.source_path,
        &mod_info.id,
        &mod_info.name,
        &local_mods_path,
        mod_info.active_version_folder.as_deref(),
    )
}

/// Create a combined fix mod that patches all auto-fixable script issues across
/// all mods in the given load order. Only mods with fixable issues are included.
/// Manual review issues (Lua API) are listed in the description but not patched.
#[tauri::command]
pub async fn create_modpack_fixes_cmd(
    load_order: Vec<String>,
    pack_name: String,
    state: State<'_, AppState>,
) -> Result<auto_fixer::ModpackFixReport, AppError> {
    let config = state.config.read().await;
    let zomboid_dir = config.zomboid_user_dir.clone()
        .ok_or_else(|| AppError::Validation(
            "Zomboid user directory is not configured. Set it in Settings first.".into()
        ))?;
    drop(config);
    let workshop_path = zomboid_dir.join("Workshop");
    std::fs::create_dir_all(&workshop_path)?;

    // Resolve all mods in load order
    let all_mods = cache::get_cached_mods(&state.db).await?;
    let mod_map: std::collections::HashMap<&str, &crate::app_core::types::ModInfo> =
        all_mods.iter().map(|m| (m.id.as_str(), m)).collect();

    let mods_in_order: Vec<crate::app_core::types::ModInfo> = load_order.iter()
        .filter_map(|id| mod_map.get(id.as_str()).copied().cloned())
        .collect();

    // Collect manual review issues from compat scan (Lua API deprecations)
    // Load deprecation rules
    let versions_data = crate::features::extensions::loader::load_migration_versions(
        &state.app_data_dir
    );
    let mut manual_review: Vec<(String, String, u32, String, String)> = Vec::new();

    if !versions_data.versions.is_empty() {
        // Load the broadest ruleset (first from → last to)
        let from = &versions_data.versions[0].from;
        let to = &versions_data.versions.last().unwrap().to;
        if let Ok(raw) = crate::features::extensions::loader::load_migration_rules(
            &state.app_data_dir, from, to
        ) {
            let rules: Vec<migration::DeprecationRule> = raw.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();

            for mod_info in &mods_in_order {
                let report = migration::scan_mod_migration(
                    &mod_info.source_path,
                    &mod_info.id,
                    &mod_info.name,
                    &rules,
                    mod_info.active_version_folder.as_deref(),
                );
                for issue in &report.issues {
                    if !issue.auto_fixable {
                        manual_review.push((
                            mod_info.name.clone(),
                            issue.file.clone(),
                            issue.line,
                            issue.old_api.clone(),
                            issue.message.clone(),
                        ));
                    }
                }
            }
        }
    }

    // Build item dictionary for reference checking
    let config2 = state.config.read().await;
    let game_path = config2.game_path.clone();
    drop(config2);

    // Build item dictionary from ONLY active mods (mods_in_order), not all
    // discovered mods. Using all_mods would include inactive mods and old fix
    // packs, masking missing item references that need placeholders.
    let mut known_items = if let Some(ref gp) = game_path {
        item_references::build_base_game_dictionary(gp)
    } else {
        std::collections::HashSet::new()
    };
    for mi in &mods_in_order {
        let mod_items = item_references::build_mod_dictionary(
            &mi.source_path,
            mi.active_version_folder.as_deref(),
        );
        known_items.extend(mod_items);
    }

    auto_fixer::create_modpack_fixes(
        &mods_in_order,
        &pack_name,
        &workshop_path,
        &zomboid_dir,
        &known_items,
        &manual_review,
    )
}
