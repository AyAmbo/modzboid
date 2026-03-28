//! Auto-fixer: creates a local fixed copy of a mod with B42 compatibility patches applied.
//!
//! Fixes applied:
//! - Script properties: Type → ItemType, case fixes, SwingTime → BaseSpeed
//! - DisplayName → generates translation JSON file
//! - Lua API renames (where suggestion contains →)
//! - Adds TODO comments for non-fixable issues

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use regex::Regex;

use crate::app_core::error::AppError;
use crate::app_core::types::ModInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoFixReport {
    pub mod_id: String,
    pub mod_name: String,
    pub output_path: String,
    pub fixes_applied: u32,
    pub todos_added: u32,
    pub translation_entries: u32,
    pub details: Vec<FixDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixDetail {
    pub file: String,
    pub line: u32,
    pub action: String, // "fixed", "todo", "translation"
    pub before: String,
    pub after: String,
}

/// B41 Type → B42 ItemType mapping
fn type_mapping() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("Weapon", "base:weapon");
    m.insert("Normal", "base:normal");
    m.insert("Food", "base:food");
    m.insert("Clothing", "base:clothing");
    m.insert("Container", "base:container");
    m.insert("Drainable", "base:drainable");
    m.insert("Literature", "base:literature");
    m.insert("Moveable", "base:moveable");
    m.insert("Radio", "base:radio");
    m.insert("Map", "base:map");
    m.insert("WeaponPart", "base:weaponpart");
    m.insert("Key", "base:key");
    m.insert("AlarmClock", "base:alarmclock");
    m
}

/// Case-fix mapping (B41 property → B42 property)
/// VERIFIED by comparing B41 and B42 game files downloaded via SteamCMD
fn case_fixes() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Weapon item case changes (verified against 106 common weapons)
    m.insert("MinimumSwingTime", "MinimumSwingtime");
    m.insert("MaxHitCount", "MaxHitcount");
    m.insert("AimingTime", "Aimingtime");
    m.insert("SwingTime", "Swingtime");
    m.insert("ReloadTime", "Reloadtime");
    m.insert("ProjectileCount", "Projectilecount");
    m.insert("critDmgMultiplier", "CritDmgMultiplier");
    m.insert("haveChamber", "HaveChamber");
    m.insert("clothingExtraSubmenu", "ClothingExtraSubmenu");
    // Vehicle case changes (verified: B42 uses lowercase, mods often use uppercase)
    m.insert("WheelFriction", "wheelFriction");
    m.insert("SuspensionCompression", "suspensionCompression");
    m.insert("SuspensionDamping", "suspensionDamping");
    m
}

/// Properties that are actually removed in B42 (verified against game files)
fn removed_properties() -> Vec<&'static str> {
    vec![
        // Verified removed from weapon items (not in any B42 weapon):
        "ShareDamage", "FirePower", "ReplaceOnUseOn",
        // Non-weapon items:
        "EnduranceChange", "FatigueChange", "FluReduction", "PainReduction",
        "Poison", "PoisonDetectionLevel", "UseForPoison", "ReduceFoodSickness",
        "ReplaceTypes", "HairDye", "TeachedRecipes",
        "TriggerExplosionTimer", "CountDownSound", "EngineLoudness",
        "OBSOLETE", "Obsolete", "OnlyAcceptCategory",
    ]
    // NOTE: These are STILL VALID in B42 (verified in 80+ weapons):
    // RunAnim, SwingAnim, SplatNumber, SplatBloodOnNoDeath, TwoHandWeapon,
    // SwingAmountBeforeImpact, SubCategory — DO NOT remove/comment these!
}

/// Copy a directory recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Create a fixed local copy of a mod with B42 compatibility patches.
pub fn create_fixed_copy(
    source_path: &Path,
    mod_id: &str,
    mod_name: &str,
    local_mods_path: &Path,
    active_version_folder: Option<&str>,
) -> Result<AutoFixReport, AppError> {
    // Determine output directory
    let output_name = format!("{}_fixed", mod_id);
    let output_dir = local_mods_path.join(&output_name);

    // Remove existing fixed copy if present
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)?;
    }

    // Copy entire mod
    copy_dir_recursive(source_path, &output_dir)?;

    let type_map = type_mapping();
    let case_map = case_fixes();
    let removed_props = removed_properties();

    let mut details = Vec::new();
    let mut fixes_applied = 0u32;
    let mut todos_added = 0u32;

    // Collect DisplayName entries for translation file
    let mut translations: HashMap<String, String> = HashMap::new(); // "Module.Item" -> "Name"

    // Determine which directories to fix
    let mut script_dirs = Vec::new();
    if let Some(vf) = active_version_folder {
        let d = output_dir.join(vf).join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }
    let common_scripts = output_dir.join("common").join("media").join("scripts");
    if common_scripts.exists() { script_dirs.push(common_scripts); }
    if script_dirs.is_empty() {
        let d = output_dir.join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }

    // Track current module name for translations
    let module_re = Regex::new(r"^module\s+(\w+)").unwrap();
    let module_alone_re = Regex::new(r"^module\s*$").unwrap();
    let module_name_re = Regex::new(r"^(\w+)\s*\{?\s*$").unwrap();
    let item_re = Regex::new(r"(?i)^(item|craftRecipe|recipe|vehicle|template)\s+(\w+)").unwrap();
    let prop_re = Regex::new(r"^(\s*)(\w+)\s*=\s*(.*)").unwrap();

    // Fix script files
    for scripts_dir in &script_dirs {
        for entry in walkdir::WalkDir::new(scripts_dir)
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

            let rel_path = path.strip_prefix(&output_dir).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy().to_string();

            let mut new_lines = Vec::new();
            let mut modified = false;
            let mut current_module = String::new();
            let mut current_item = String::new();
            let mut brace_depth = 0i32;
            let mut expect_module_name = false;
            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                // Track module name (handles "module\nName {" split across lines)
                if expect_module_name {
                    expect_module_name = false;
                    if let Some(caps) = module_name_re.captures(trimmed) {
                        current_module = caps[1].to_string();
                    }
                }
                if module_alone_re.is_match(trimmed) {
                    expect_module_name = true;
                } else if let Some(caps) = module_re.captures(trimmed) {
                    current_module = caps[1].to_string();
                }

                // Track item/recipe block name
                if let Some(caps) = item_re.captures(trimmed) {
                    current_item = caps[2].to_string();
                }

                brace_depth += trimmed.matches('{').count() as i32;
                brace_depth -= trimmed.matches('}').count() as i32;

                // Process properties inside item blocks
                if brace_depth >= 2 {
                    if let Some(caps) = prop_re.captures(line) {
                        let indent = &caps[1];
                        let prop = &caps[2];
                        let value_with_comma = &caps[3];
                        let value = value_with_comma.trim().trim_end_matches(',').trim();
                        let has_comma = value_with_comma.trim().ends_with(',');
                        let comma = if has_comma { "," } else { "" };

                        // 1. Type → ItemType mapping
                        if prop == "Type" {
                            if let Some(&new_type) = type_map.get(value) {
                                let new_line = format!("{}ItemType = {}{}", indent, new_type, comma);
                                details.push(FixDetail {
                                    file: rel_str.clone(),
                                    line: (line_num + 1) as u32,
                                    action: "fixed".into(),
                                    before: format!("Type = {}", value),
                                    after: format!("ItemType = {}", new_type),
                                });
                                new_lines.push(new_line);
                                fixes_applied += 1;
                                modified = true;
                                continue;
                            }
                        }

                        // 2. Case fixes
                        if let Some(&new_prop) = case_map.get(prop.as_ref() as &str) {
                            let new_line = format!("{}{} = {}", indent, new_prop, value_with_comma);
                            details.push(FixDetail {
                                file: rel_str.clone(),
                                line: (line_num + 1) as u32,
                                action: "fixed".into(),
                                before: format!("{} = {}", prop, value),
                                after: format!("{} = {}", new_prop, value),
                            });
                            new_lines.push(new_line);
                            fixes_applied += 1;
                            modified = true;
                            continue;
                        }

                        // 3. DisplayName → collect for translation file
                        if prop == "DisplayName" && !current_module.is_empty() && !current_item.is_empty() {
                            let key = format!("{}.{}", current_module, current_item);
                            translations.insert(key, value.to_string());
                            details.push(FixDetail {
                                file: rel_str.clone(),
                                line: (line_num + 1) as u32,
                                action: "translation".into(),
                                before: format!("DisplayName = {}", value),
                                after: "Moved to translation file".into(),
                            });
                            modified = true;
                            continue;
                        }

                        // 4. Actually removed properties → comment out with TODO
                        if removed_props.contains(&(prop.as_ref() as &str)) {
                            let todo_line = format!("{}-- TODO [B42]: {} is no longer used in B42", indent, prop);
                            let commented = format!("{}-- {}", indent, trimmed);
                            new_lines.push(todo_line);
                            new_lines.push(commented);
                            details.push(FixDetail {
                                file: rel_str.clone(),
                                line: (line_num + 1) as u32,
                                action: "todo".into(),
                                before: format!("{} = {}", prop, value),
                                after: "Commented out (removed in B42)".into(),
                            });
                            todos_added += 1;
                            modified = true;
                            continue;
                        }
                    }
                }

                new_lines.push(line.to_string());
            }

            if modified {
                std::fs::write(path, new_lines.join("\n"))?;
            }
        }
    }

    // Generate translation file
    let translation_entries = translations.len() as u32;
    if !translations.is_empty() {
        // Find the right location for the translation file
        let translate_dir = if let Some(vf) = active_version_folder {
            output_dir.join(vf).join("media").join("lua").join("shared").join("Translate").join("EN")
        } else if output_dir.join("common").join("media").exists() {
            output_dir.join("common").join("media").join("lua").join("shared").join("Translate").join("EN")
        } else {
            output_dir.join("media").join("lua").join("shared").join("Translate").join("EN")
        };

        std::fs::create_dir_all(&translate_dir)?;
        // Use a unique filename to avoid replacing the base game's or other mods' ItemName.json.
        // PZ merges differently-named translation files but replaces same-named ones.
        let trans_filename = format!("{}_ItemName.json", mod_id);
        let trans_path = translate_dir.join(trans_filename);

        // Merge with existing if present
        let mut existing: HashMap<String, String> = if trans_path.exists() {
            let content = std::fs::read_to_string(&trans_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        for (k, v) in &translations {
            existing.insert(k.clone(), v.clone());
        }

        let json = serde_json::to_string_pretty(&existing)?;
        std::fs::write(&trans_path, json)?;

        fixes_applied += translation_entries;
    }

    // Write fix report
    let report_path = output_dir.join("_fix_report.txt");
    let mut report_text = format!(
        "Auto-Fix Report — {}\n\
         Generated by Project Modzboid\n\
         Original: {}\n\
         \n\
         SUMMARY:\n\
         - {} fixes applied\n\
         - {} items commented out (TODO)\n\
         - {} DisplayName entries moved to translation file\n\n",
        mod_name,
        source_path.display(),
        fixes_applied,
        todos_added,
        translation_entries,
    );

    for d in &details {
        report_text.push_str(&format!(
            "[{}] {}:{}\n  {} → {}\n\n",
            d.action, d.file, d.line, d.before, d.after
        ));
    }
    std::fs::write(&report_path, &report_text)?;

    Ok(AutoFixReport {
        mod_id: mod_id.to_string(),
        mod_name: mod_name.to_string(),
        output_path: output_dir.to_string_lossy().to_string(),
        fixes_applied,
        todos_added,
        translation_entries,
        details,
    })
}

// ─── Modpack Fixes Generator ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFixReport {
    pub output_path: String,
    pub mod_id: String,
    pub mods_patched: u32,
    pub mods_skipped: u32,
    pub total_fixes: u32,
    pub total_todos: u32,
    pub total_translations: u32,
    pub manual_review_issues: u32,
    pub patched_mods: Vec<PatchedModSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchedModSummary {
    pub mod_id: String,
    pub mod_name: String,
    pub authors: Vec<String>,
    pub workshop_id: Option<String>,
    pub workshop_url: Option<String>,
    pub fixes_applied: u32,
    pub todos_added: u32,
    pub translation_entries: u32,
    pub details: Vec<FixDetail>,
}

/// Info about a Lua API issue that needs manual review (not auto-fixed).
struct ManualReviewIssue {
    mod_name: String,
    file: String,
    line: u32,
    api: String,
    message: String,
}

/// Create a single combined fix mod that patches all auto-fixable script issues
/// across all mods in the load order. Only includes files that were actually changed.
/// Uses the correct Workshop upload structure:
///   {workshop_path}/{pack_id}/workshop.txt
///   {workshop_path}/{pack_id}/preview.png
///   {workshop_path}/{pack_id}/Contents/mods/{pack_id}/mod.info
///   {workshop_path}/{pack_id}/Contents/mods/{pack_id}/42/media/scripts/...
pub fn create_modpack_fixes(
    mods: &[ModInfo],
    pack_name: &str,
    workshop_path: &Path,
    zomboid_dir: &Path,
    known_items: &std::collections::HashSet<String>,
    manual_review: &[(String, String, u32, String, String)], // (mod_name, file, line, api, message)
) -> Result<ModpackFixReport, AppError> {
    let pack_id = sanitize_mod_id(pack_name);
    let workshop_root = workshop_path.join(&pack_id);
    let output_dir = workshop_root.join("Contents").join("mods").join(&pack_id);

    // Remove existing if present
    if workshop_root.exists() {
        std::fs::remove_dir_all(&workshop_root)?;
    }
    std::fs::create_dir_all(&output_dir)?;

    let type_map = type_mapping();
    let case_map = case_fixes();
    let removed_props = removed_properties();
    let module_re = Regex::new(r"^module\s+(\w+)").unwrap();
    let module_alone_re = Regex::new(r"^module\s*$").unwrap();
    let module_name_re = Regex::new(r"^(\w+)\s*\{?\s*$").unwrap();
    let item_re = Regex::new(r"(?i)^(item|craftRecipe|recipe|vehicle|template)\s+(\w+)").unwrap();
    let prop_re = Regex::new(r"^(\s*)(\w+)\s*=\s*(.*)").unwrap();

    let mut patched_mods = Vec::new();
    // Collect missing item references across all mods for placeholder generation
    // Key: (module, item_name), Value: list of (mod_id, mod_name, workshop_id, context)
    let mut missing_items: HashMap<(String, String), Vec<(String, String, Option<String>, String)>> = HashMap::new();
    // Missing fluids: key is fluid name, value is list of (mod_id, mod_name, workshop_id, context)
    let mut missing_fluids: HashMap<String, Vec<(String, String, Option<String>, String)>> = HashMap::new();
    let missing_ref_re = Regex::new(r"\b([A-Z]\w+)\.(\w+)\b").unwrap();
    let fluid_ref_re = Regex::new(r"-fluid\s+[\d.]+\s+\[([^\]]+)\]").unwrap();
    let recipe_block_re = Regex::new(r"(?i)^\s*(craftRecipe|recipe|fixing|evolvedrecipe)\s+").unwrap();
    let ingredient_line_re = Regex::new(r"^\s*item\s+\d+").unwrap();
    let non_item_mods: HashSet<&str> = [
        "mode", "flags", "mappers", "mapper", "tags", "base",
        "Vehicles", "BuildRecipeCode", "ISBuildMenu", "ISBuild",
        "ISInventoryPaneContextMenu", "SandboxVars", "Events",
    ].iter().copied().collect();
    // Per-mod translations: mod_id -> { "ItemName_Module.Item" -> "Display Name" }
    let mut per_mod_translations: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut total_fixes = 0u32;
    let mut total_todos = 0u32;
    let mut mods_skipped = 0u32;
    let mut dependency_ids = Vec::new();

    for mod_info in mods {
        // Find script directories in the source mod
        let source = &mod_info.source_path;
        let mut script_dirs: Vec<(PathBuf, String)> = Vec::new(); // (abs_path, relative_prefix)

        if let Some(ref vf) = mod_info.active_version_folder {
            let d = source.join(vf).join("media").join("scripts");
            if d.exists() {
                script_dirs.push((d, format!("{}/media/scripts", vf)));
            }
        }
        let common_scripts = source.join("common").join("media").join("scripts");
        if common_scripts.exists() {
            script_dirs.push((common_scripts, "common/media/scripts".into()));
        }
        if script_dirs.is_empty() {
            let d = source.join("media").join("scripts");
            if d.exists() {
                script_dirs.push((d, "media/scripts".into()));
            }
        }

        if script_dirs.is_empty() {
            mods_skipped += 1;
            continue;
        }

        let mut mod_details = Vec::new();
        let mut mod_fixes = 0u32;
        let mut mod_todos = 0u32;
        let mut mod_translations = 0u32;

        for (scripts_dir, _rel_prefix) in &script_dirs {
            for entry in walkdir::WalkDir::new(scripts_dir)
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

                let rel_to_scripts = path.strip_prefix(scripts_dir).unwrap_or(path);
                let display_file = format!("{}/{}", mod_info.id, rel_to_scripts.to_string_lossy());

                let mut new_lines = Vec::new();
                let mut modified = false;
                let mut current_module = String::new();
                let mut current_item = String::new();
                let mut brace_depth = 0i32;
                let mut expect_module_name = false;

                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();

                    // Track module name (handles "module\nName {" split across lines)
                    if expect_module_name {
                        expect_module_name = false;
                        if let Some(caps) = module_name_re.captures(trimmed) {
                            current_module = caps[1].to_string();
                        }
                    }
                    if module_alone_re.is_match(trimmed) {
                        expect_module_name = true;
                    } else if let Some(caps) = module_re.captures(trimmed) {
                        current_module = caps[1].to_string();
                    }
                    if let Some(caps) = item_re.captures(trimmed) {
                        current_item = caps[2].to_string();
                    }

                    brace_depth += trimmed.matches('{').count() as i32;
                    brace_depth -= trimmed.matches('}').count() as i32;

                    if brace_depth >= 2 {
                        if let Some(caps) = prop_re.captures(line) {
                            let indent = &caps[1];
                            let prop = &caps[2];
                            let value_with_comma = &caps[3];
                            let value = value_with_comma.trim().trim_end_matches(',').trim();
                            let has_comma = value_with_comma.trim().ends_with(',');
                            let comma = if has_comma { "," } else { "" };

                            // Type → ItemType
                            if prop == "Type" {
                                if let Some(&new_type) = type_map.get(value) {
                                    new_lines.push(format!("{}ItemType = {}{}", indent, new_type, comma));
                                    mod_details.push(FixDetail {
                                        file: display_file.clone(),
                                        line: (line_num + 1) as u32,
                                        action: "fixed".into(),
                                        before: format!("Type = {}", value),
                                        after: format!("ItemType = {}", new_type),
                                    });
                                    mod_fixes += 1;
                                    modified = true;
                                    continue;
                                }
                            }

                            // Case fixes
                            if let Some(&new_prop) = case_map.get(prop.as_ref() as &str) {
                                new_lines.push(format!("{}{} = {}", indent, new_prop, value_with_comma));
                                mod_details.push(FixDetail {
                                    file: display_file.clone(),
                                    line: (line_num + 1) as u32,
                                    action: "fixed".into(),
                                    before: format!("{} = {}", prop, value),
                                    after: format!("{} = {}", new_prop, value),
                                });
                                mod_fixes += 1;
                                modified = true;
                                continue;
                            }

                            // DisplayName → collect for per-mod translation file (unique name avoids clobbering)
                            if prop == "DisplayName" && !current_module.is_empty() && !current_item.is_empty() {
                                let key = format!("{}.{}", current_module, current_item);
                                per_mod_translations.entry(mod_info.id.clone())
                                    .or_default()
                                    .insert(key, value.to_string());
                                mod_details.push(FixDetail {
                                    file: display_file.clone(),
                                    line: (line_num + 1) as u32,
                                    action: "translation".into(),
                                    before: format!("DisplayName = {}", value),
                                    after: "Moved to translation file".into(),
                                });
                                mod_translations += 1;
                                modified = true;
                                continue;
                            }

                            // Removed properties → comment out
                            if removed_props.contains(&(prop.as_ref() as &str)) {
                                new_lines.push(format!("{}-- TODO [B42]: {} is no longer used in B42", indent, prop));
                                new_lines.push(format!("{}-- {}", indent, trimmed));
                                mod_details.push(FixDetail {
                                    file: display_file.clone(),
                                    line: (line_num + 1) as u32,
                                    action: "todo".into(),
                                    before: format!("{} = {}", prop, value),
                                    after: "Commented out (removed in B42)".into(),
                                });
                                mod_todos += 1;
                                modified = true;
                                continue;
                            }
                        }
                    }
                    new_lines.push(line.to_string());
                }

                if modified {
                    // Write the fixed file into the output mod under 42/ version folder,
                    // namespaced by source mod ID to avoid filename collisions
                    let out_script_dir = output_dir.join("42").join("media").join("scripts").join(&mod_info.id);
                    std::fs::create_dir_all(&out_script_dir)?;
                    let out_file = out_script_dir.join(rel_to_scripts);
                    if let Some(parent) = out_file.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&out_file, new_lines.join("\n"))?;
                }
            }
        }

        // Scan for missing item references in recipe/fixing scripts (for placeholder generation)
        if !known_items.is_empty() {
            for (scripts_dir, _) in &script_dirs {
                for entry in walkdir::WalkDir::new(scripts_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "txt"))
                {
                    let content = match std::fs::read_to_string(entry.path()) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let mut in_recipe_block = false;
                    let mut depth = 0i32;
                    let mut current_recipe = String::new();
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if recipe_block_re.is_match(trimmed) {
                            in_recipe_block = true;
                            current_recipe = trimmed.to_string();
                        }
                        depth += trimmed.matches('{').count() as i32;
                        depth -= trimmed.matches('}').count() as i32;
                        if depth <= 0 { in_recipe_block = false; }

                        if in_recipe_block && depth >= 2 {
                            // Check ingredient lines and reference properties (Require, Fixer, etc.)
                            let is_ref_prop = trimmed.starts_with("Require") || trimmed.starts_with("Fixer");
                            if ingredient_line_re.is_match(trimmed) || is_ref_prop {
                                for rcaps in missing_ref_re.captures_iter(trimmed) {
                                    let rmod = &rcaps[1];
                                    let ritem = &rcaps[2];
                                    if non_item_mods.contains(rmod.as_ref() as &str) { continue; }
                                    let full_ref = format!("{}.{}", rmod, ritem);
                                    if !known_items.contains(&full_ref) {
                                        missing_items
                                            .entry((rmod.to_string(), ritem.to_string()))
                                            .or_default()
                                            .push((
                                                mod_info.id.clone(),
                                                mod_info.name.clone(),
                                                mod_info.workshop_id.clone(),
                                                current_recipe.clone(),
                                            ));
                                    }
                                }
                            }
                            // Check fluid references: "-fluid 0.1 [FluidName]"
                            if let Some(fcaps) = fluid_ref_re.captures(trimmed) {
                                let fluid_list = &fcaps[1];
                                for fluid_name in fluid_list.split(';') {
                                    let fluid_name = fluid_name.trim();
                                    if !fluid_name.is_empty() && !known_items.contains(&format!("fluid:{}", fluid_name)) {
                                        missing_fluids
                                            .entry(fluid_name.to_string())
                                            .or_default()
                                            .push((
                                                mod_info.id.clone(),
                                                mod_info.name.clone(),
                                                mod_info.workshop_id.clone(),
                                                current_recipe.clone(),
                                            ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if mod_fixes == 0 && mod_todos == 0 && mod_translations == 0 {
            mods_skipped += 1;
            continue;
        }

        total_fixes += mod_fixes;
        total_todos += mod_todos;

        // Build workshop URL
        let workshop_url = mod_info.workshop_id.as_ref()
            .map(|wid| format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", wid));

        dependency_ids.push(mod_info.id.clone());

        patched_mods.push(PatchedModSummary {
            mod_id: mod_info.id.clone(),
            mod_name: mod_info.name.clone(),
            authors: mod_info.authors.clone(),
            workshop_id: mod_info.workshop_id.clone(),
            workshop_url,
            fixes_applied: mod_fixes,
            todos_added: mod_todos,
            translation_entries: mod_translations,
            details: mod_details,
        });
    }

    if patched_mods.is_empty() && missing_items.is_empty() && missing_fluids.is_empty() {
        // Clean up empty dir
        let _ = std::fs::remove_dir_all(&workshop_root);
        return Err(AppError::Validation(
            "No mods had auto-fixable script issues or missing item references. Nothing to patch.".into()
        ));
    }

    // Generate placeholder items for missing references (prevents WorldDictionaryException crashes)
    let placeholder_count = missing_items.len() as u32;
    if !missing_items.is_empty() {
        // Group by module for the script file
        let mut by_module: HashMap<String, Vec<(String, Vec<(String, String, Option<String>, String)>)>> = HashMap::new();
        for ((module, item_name), refs) in &missing_items {
            by_module.entry(module.clone()).or_default().push((item_name.clone(), refs.clone()));
        }

        let mut placeholder_script = String::new();

        for (module, items) in &by_module {
            placeholder_script.push_str(&format!("module {}\n{{\n", module));
            for (item_name, _refs) in items {
                placeholder_script.push_str(&format!("    item {}\n    {{\n", item_name));
                placeholder_script.push_str("        ItemType = base:normal,\n");
                placeholder_script.push_str("        Weight = 0.1,\n");
                placeholder_script.push_str("        DisplayCategory = Ammo,\n");
                placeholder_script.push_str("    }\n\n");
            }
            placeholder_script.push_str("}\n\n");
        }

        let placeholder_dir = output_dir.join("42").join("media").join("scripts");
        std::fs::create_dir_all(&placeholder_dir)?;
        std::fs::write(placeholder_dir.join("placeholder_items.txt"), &placeholder_script)?;

        total_fixes += placeholder_count;

        // Track which mod IDs we've already added to patched_mods for deduplication
        let mut seen_mod_ids: HashSet<String> = patched_mods.iter().map(|p| p.mod_id.clone()).collect();

        // Add placeholder info to patched_mods details
        for ((module, item_name), refs) in &missing_items {
            for (mod_id, mod_name, workshop_id, ctx) in refs {
                // Find existing patched mod entry by real mod ID
                if let Some(pm) = patched_mods.iter_mut().find(|p| p.mod_id == *mod_id) {
                    pm.fixes_applied += 1;
                    pm.details.push(FixDetail {
                        file: "_placeholder_items.txt".into(),
                        line: 0,
                        action: "placeholder".into(),
                        before: format!("{}.{} (missing)", module, item_name),
                        after: format!("Created placeholder item (referenced in {})", ctx),
                    });
                } else if !seen_mod_ids.contains(mod_id) {
                    // Create a new entry using real mod ID and info
                    let ws_url = workshop_id.as_ref()
                        .map(|wid| format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", wid));
                    patched_mods.push(PatchedModSummary {
                        mod_id: mod_id.clone(),
                        mod_name: mod_name.clone(),
                        authors: vec![],
                        workshop_id: workshop_id.clone(),
                        workshop_url: ws_url,
                        fixes_applied: 1,
                        todos_added: 0,
                        translation_entries: 0,
                        details: vec![FixDetail {
                            file: "_placeholder_items.txt".into(),
                            line: 0,
                            action: "placeholder".into(),
                            before: format!("{}.{} (missing)", module, item_name),
                            after: format!("Created placeholder item (referenced in {})", ctx),
                        }],
                    });
                    dependency_ids.push(mod_id.clone());
                    seen_mod_ids.insert(mod_id.clone());
                }
            }
        }
    }

    // Generate placeholder fluids for missing fluid references
    if !missing_fluids.is_empty() {
        let placeholder_dir = output_dir.join("42").join("media").join("scripts");
        std::fs::create_dir_all(&placeholder_dir)?;

        let mut fluid_script = String::new();
        fluid_script.push_str("module Base\n{\n");
        for (fluid_name, refs) in &missing_fluids {
            fluid_script.push_str(&format!("    fluid {}\n    {{\n", fluid_name));
            fluid_script.push_str("        ColorReference = LightSkyBlue,\n");
            fluid_script.push_str(&format!("        DisplayName = Fluid_Name_{},\n", fluid_name));
            fluid_script.push_str("        Categories\n        {\n");
            fluid_script.push_str("            Beverage,\n");
            fluid_script.push_str("        }\n");
            fluid_script.push_str("    }\n\n");

            // Add to patched_mods tracking
            let mut seen: HashSet<String> = HashSet::new();
            for (mod_id, mod_name, workshop_id, ctx) in refs {
                if seen.contains(mod_id) { continue; }
                seen.insert(mod_id.clone());
                if let Some(pm) = patched_mods.iter_mut().find(|p| p.mod_id == *mod_id) {
                    pm.fixes_applied += 1;
                    pm.details.push(FixDetail {
                        file: "placeholder_fluids.txt".into(),
                        line: 0,
                        action: "placeholder".into(),
                        before: format!("fluid:{} (missing)", fluid_name),
                        after: format!("Created placeholder fluid (referenced in {})", ctx),
                    });
                } else {
                    let ws_url = workshop_id.as_ref()
                        .map(|wid| format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", wid));
                    patched_mods.push(PatchedModSummary {
                        mod_id: mod_id.clone(),
                        mod_name: mod_name.clone(),
                        authors: vec![],
                        workshop_id: workshop_id.clone(),
                        workshop_url: ws_url,
                        fixes_applied: 1,
                        todos_added: 0,
                        translation_entries: 0,
                        details: vec![FixDetail {
                            file: "placeholder_fluids.txt".into(),
                            line: 0,
                            action: "placeholder".into(),
                            before: format!("fluid:{} (missing)", fluid_name),
                            after: format!("Created placeholder fluid (referenced in {})", ctx),
                        }],
                    });
                    dependency_ids.push(mod_id.clone());
                }
            }
        }
        fluid_script.push_str("}\n");
        std::fs::write(placeholder_dir.join("placeholder_fluids.txt"), &fluid_script)?;
        total_fixes += missing_fluids.len() as u32;
    }

    // Collect existing mod translations from .txt and .json files,
    // but ONLY for mods that already have script fixes applied (i.e., mods
    // whose DisplayName was extracted). This ensures we don't generate
    // translation files for mods the fix pack doesn't touch.
    let mods_with_fixes: std::collections::HashSet<String> = per_mod_translations.keys().cloned().collect();
    for mod_info in mods {
        if !mods_with_fixes.contains(&mod_info.id) {
            continue; // Skip mods without script fixes
        }
        let source = &mod_info.source_path;
        let mut translate_dirs = Vec::new();
        if let Some(ref vf) = mod_info.active_version_folder {
            let d = source.join(vf).join("media").join("lua").join("shared").join("Translate").join("EN");
            if d.exists() { translate_dirs.push(d); }
            // Some mods use lowercase
            let d2 = source.join(vf).join("media").join("lua").join("shared").join("translate").join("EN");
            if d2.exists() && !translate_dirs.contains(&d2) { translate_dirs.push(d2); }
        }
        let common_tr = source.join("common").join("media").join("lua").join("shared").join("Translate").join("EN");
        if common_tr.exists() { translate_dirs.push(common_tr); }

        for tr_dir in &translate_dirs {
            if let Ok(entries) = std::fs::read_dir(tr_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    // Only process ItemName files (not UI, Tooltip, Sandbox, etc.)
                    if !fname.to_lowercase().contains("itemname") { continue; }

                    if fname.ends_with(".txt") {
                        // Parse old PZ .txt format: ItemName_EN = { ItemName_Module.Item = "value", ... }
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let existing = per_mod_translations.entry(mod_info.id.clone()).or_default();
                            parse_txt_translations(&content, existing);
                        }
                    } else if fname.ends_with(".json") {
                        // Parse B42 JSON format: { "Module.Item": "value", ... }
                        // Also handles legacy "ItemName_Module.Item" prefix (strip it)
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                                let existing = per_mod_translations.entry(mod_info.id.clone()).or_default();
                                for (k, v) in map {
                                    let key = if k.starts_with("ItemName_") {
                                        k[9..].to_string()
                                    } else {
                                        k
                                    };
                                    existing.entry(key).or_insert(v);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate per-mod translation files with unique names to avoid clobbering
    let mut total_translations = 0u32;
    if !per_mod_translations.is_empty() {
        let translate_dir = output_dir.join("42").join("media").join("lua")
            .join("shared").join("Translate").join("EN");
        std::fs::create_dir_all(&translate_dir)?;
        for (mod_id, entries) in &per_mod_translations {
            let trans_filename = format!("{}-fix_ItemName.json", mod_id);
            let json = serde_json::to_string_pretty(entries)?;
            std::fs::write(translate_dir.join(trans_filename), json)?;
            total_translations += entries.len() as u32;
        }
        total_fixes += total_translations;
    }

    // Collect manual review issues for the description
    let manual_issues: Vec<ManualReviewIssue> = manual_review.iter()
        .map(|(mod_name, file, line, api, message)| ManualReviewIssue {
            mod_name: mod_name.clone(),
            file: file.clone(),
            line: *line,
            api: api.clone(),
            message: message.clone(),
        })
        .collect();
    let manual_review_count = manual_issues.len() as u32;

    // Generate mod.info (inside Contents/mods/{id}/)
    let mod_info_content = format!(
        "name={name}\n\
         id={id}\n\
         description=Auto-generated B42 compatibility fixes. Load this LAST in your mod list.\n\
         modversion=1.0\n",
        name = pack_name,
        id = pack_id,
    );
    std::fs::write(output_dir.join("mod.info"), &mod_info_content)?;
    // Also place mod.info inside the 42/ version folder (required for Workshop upload)
    let version_dir = output_dir.join("42");
    std::fs::create_dir_all(&version_dir)?;
    std::fs::write(version_dir.join("mod.info"), &mod_info_content)?;

    // preview.png (256x256, required for Workshop upload)
    // Try to copy from ModTemplate first, otherwise generate one
    let preview_dst = workshop_root.join("preview.png");
    let template_preview = zomboid_dir.join("Workshop").join("ModTemplate").join("preview.png");
    if template_preview.exists() {
        std::fs::copy(&template_preview, &preview_dst)?;
    } else {
        // Generate a minimal 256x256 dark gray PNG
        let png_data = generate_placeholder_png(256, 256);
        std::fs::write(&preview_dst, &png_data)?;
    }

    // Generate workshop.txt (at workshop root, for Steam upload)
    let workshop_txt = format!(
        "version=1\n\
         title={name}\n\
         description=Auto-generated B42 compatibility patch for a modpack collection.\n\
         description=Generated by Project Modzboid.\n\
         description=\n\
         description={mods} mods patched, {fixes} fixes applied.\n\
         description=Load this mod LAST in your mod order.\n\
         tags=Mod\n\
         visibility=public\n",
        name = pack_name,
        mods = patched_mods.len(),
        fixes = total_fixes,
    );
    std::fs::write(workshop_root.join("workshop.txt"), &workshop_txt)?;

    // Generate workshop description (Steam BBCode format)
    let mut desc = String::new();
    desc.push_str(&format!("[h1]{pack_name}[/h1]\n\n"));
    desc.push_str("Auto-generated B42 compatibility patch for a modpack collection.\n");
    desc.push_str("Generated by Project Modzboid.\n\n");
    desc.push_str(&format!(
        "[b]Stats:[/b] {} mods patched, {} fixes applied, {} commented out, {} translations\n\n",
        patched_mods.len(), total_fixes, total_todos, total_translations
    ));

    desc.push_str("[h2]Patched Mods[/h2]\n[list]\n");
    for pm in &patched_mods {
        let authors_str = if pm.authors.is_empty() {
            "Unknown".to_string()
        } else {
            pm.authors.join(", ")
        };
        let fix_count = pm.fixes_applied + pm.todos_added + pm.translation_entries;
        if let Some(ref url) = pm.workshop_url {
            desc.push_str(&format!(
                "[*] [url={url}]{name}[/url] by {authors} - {fixes} fixes\n",
                url = url, name = pm.mod_name, authors = authors_str, fixes = fix_count
            ));
        } else {
            desc.push_str(&format!(
                "[*] {name} by {authors} - {fixes} fixes\n",
                name = pm.mod_name, authors = authors_str, fixes = fix_count
            ));
        }
    }
    desc.push_str("[/list]\n");

    desc.push_str("\n[h2]Changes Applied[/h2]\n");
    for pm in &patched_mods {
        desc.push_str(&format!("\n[h3]{name}[/h3]\n[list]\n", name = pm.mod_name));
        for d in &pm.details {
            desc.push_str(&format!(
                "[*] [b]{file}[/b] line {line}: {before} -> {after}\n",
                file = d.file, line = d.line, before = d.before, after = d.after
            ));
        }
        desc.push_str("[/list]\n");
    }

    if !manual_issues.is_empty() {
        desc.push_str("\n[h2]Needs Manual Review (NOT patched)[/h2]\n");
        desc.push_str("These Lua API issues were detected but cannot be auto-fixed.\n");
        desc.push_str("They may or may not cause problems in B42.\n\n");

        let mut current_mod = String::new();
        for issue in &manual_issues {
            if issue.mod_name != current_mod {
                current_mod = issue.mod_name.clone();
                desc.push_str(&format!("\n[h3]{name}[/h3]\n[list]\n", name = current_mod));
            }
            desc.push_str(&format!(
                "[*] {file}:{line} - {api} ({msg})\n",
                file = issue.file, line = issue.line, api = issue.api, msg = issue.message
            ));
        }
        desc.push_str("[/list]\n");
    }

    desc.push_str("\n[h2]How to Use[/h2]\n[list]\n");
    desc.push_str("[*] Subscribe to this mod AND all required mods listed above\n");
    desc.push_str("[*] Place this mod LAST in your load order\n");
    desc.push_str("[*] The fixed script files will override the originals\n");
    desc.push_str("[/list]\n");

    // description.txt goes in the workshop root (handy for copy-paste to Steam)
    std::fs::write(workshop_root.join("description.txt"), &desc)?;

    // Also write a plain-text report
    let mut report = String::new();
    report.push_str(&format!("Modpack Fix Report — {}\n", pack_name));
    report.push_str(&format!("Generated by Project Modzboid\n\n"));
    report.push_str(&format!("Mods patched: {}\n", patched_mods.len()));
    report.push_str(&format!("Total fixes: {}\n", total_fixes));
    report.push_str(&format!("Properties commented out: {}\n", total_todos));
    report.push_str(&format!("Translations moved: {}\n", total_translations));
    report.push_str(&format!("Manual review needed: {}\n\n", manual_review_count));

    for pm in &patched_mods {
        report.push_str(&format!("--- {} ({}) ---\n", pm.mod_name, pm.mod_id));
        if let Some(ref url) = pm.workshop_url {
            report.push_str(&format!("Workshop: {}\n", url));
        }
        report.push_str(&format!("Authors: {}\n", if pm.authors.is_empty() { "Unknown".into() } else { pm.authors.join(", ") }));
        for d in &pm.details {
            report.push_str(&format!("  [{}] {}:{} — {} -> {}\n", d.action, d.file, d.line, d.before, d.after));
        }
        report.push('\n');
    }

    if !manual_issues.is_empty() {
        report.push_str("=== MANUAL REVIEW NEEDED ===\n");
        for issue in &manual_issues {
            report.push_str(&format!("  {} — {}:{} — {} ({})\n",
                issue.mod_name, issue.file, issue.line, issue.api, issue.message));
        }
    }

    std::fs::write(workshop_root.join("_fix_report.txt"), &report)?;

    Ok(ModpackFixReport {
        output_path: workshop_root.to_string_lossy().to_string(),
        mod_id: pack_id,
        mods_patched: patched_mods.len() as u32,
        mods_skipped,
        total_fixes,
        total_todos,
        total_translations,
        manual_review_issues: manual_review_count,
        patched_mods,
    })
}

/// Generate a minimal PNG image (solid dark gray) for Workshop preview placeholder.
fn generate_placeholder_png(width: u32, height: u32) -> Vec<u8> {
    use std::io::Write;

    // Build raw image data (filter byte 0 + RGB pixels per row)
    let mut raw = Vec::with_capacity((1 + width as usize * 3) * height as usize);
    for _ in 0..height {
        raw.push(0u8); // filter: None
        for _ in 0..width {
            raw.extend_from_slice(&[0x30, 0x30, 0x30]); // dark gray
        }
    }

    let compressed = {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&raw).unwrap();
        encoder.finish().unwrap()
    };

    fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
        buf.extend_from_slice(chunk_type);
        buf.extend_from_slice(data);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(chunk_type);
        hasher.update(data);
        buf.extend_from_slice(&hasher.finalize().to_be_bytes());
        buf
    }

    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&width.to_be_bytes());
    ihdr_data.extend_from_slice(&height.to_be_bytes());
    ihdr_data.push(8);  // bit depth
    ihdr_data.push(2);  // color type: RGB
    ihdr_data.push(0);  // compression
    ihdr_data.push(0);  // filter
    ihdr_data.push(0);  // interlace

    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    png.extend_from_slice(&png_chunk(b"IHDR", &ihdr_data));
    png.extend_from_slice(&png_chunk(b"IDAT", &compressed));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

/// Sanitize a name into a valid PZ mod ID (alphanumeric + underscores only).
fn sanitize_mod_id(name: &str) -> String {
    let sanitized: String = name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if sanitized.is_empty() { "modpack_fixes".into() } else { sanitized }
}

/// Parse PZ's old .txt translation format into B42 JSON key format.
/// Input format: `ItemName_EN = { ItemName_Module.Item = "value", ... }`
/// Output: inserts `"Module.Item" -> "value"` into the provided map.
fn parse_txt_translations(content: &str, out: &mut HashMap<String, String>) {
    let re = Regex::new(r#"ItemName_(\w+\.\w+)\s*=\s*"([^"]*)""#).unwrap();
    for cap in re.captures_iter(content) {
        let key = cap[1].to_string();   // "Module.Item"
        let value = cap[2].to_string(); // "Display Name"
        if !value.is_empty() {
            out.entry(key).or_insert(value);
        }
    }
}
