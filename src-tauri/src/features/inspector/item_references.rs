//! Item Reference Checker — builds a dictionary of all defined items from the base game
//! and loaded mods, then scans mod scripts for references to non-existent items.
//!
//! Catches crashes like WorldDictionaryException caused by recipes referencing
//! items that don't exist (e.g. Base.223Bullets removed in B42).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A reference to an item that doesn't exist in any loaded mod or the base game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingItemRef {
    pub mod_id: String,
    pub mod_name: String,
    pub file: String,
    pub line: u32,
    pub context: String,       // "recipe", "item_property", "fixing", "item_mapper"
    pub block_name: String,    // e.g. "craftRecipe Make223Ammo" or "item MyGun"
    pub property: String,      // e.g. "AmmoType", "ReplaceOnUse", or "ingredient"
    pub referenced_item: String, // e.g. "Base.223Bullets"
    pub severity: String,      // "error" for recipes (crash), "warning" for properties
}

/// Report for all missing references across all mods.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRefReport {
    pub total_items_known: u32,
    pub total_references_checked: u32,
    pub total_missing: u32,
    pub missing_refs: Vec<MissingItemRef>,
}

/// Build a set of all known item IDs from the base game scripts directory.
/// Returns items as "Module.ItemName" (e.g. "Base.Axe").
pub fn build_base_game_dictionary(game_path: &Path) -> HashSet<String> {
    let mut items = HashSet::new();
    let scripts_dir = game_path.join("media").join("scripts");
    if !scripts_dir.exists() {
        return items;
    }
    collect_item_definitions(&scripts_dir, &mut items);
    items
}

/// Build item dictionary from a single mod's scripts.
pub fn build_mod_dictionary(
    source_path: &Path,
    active_version_folder: Option<&str>,
) -> HashSet<String> {
    let mut items = HashSet::new();

    let mut script_dirs = Vec::new();
    if let Some(vf) = active_version_folder {
        let d = source_path.join(vf).join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }
    let common = source_path.join("common").join("media").join("scripts");
    if common.exists() { script_dirs.push(common); }
    if script_dirs.is_empty() {
        let d = source_path.join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }

    for dir in &script_dirs {
        collect_item_definitions(dir, &mut items);
    }
    items
}

/// Parse all .txt script files in a directory tree and extract item + fluid definitions.
fn collect_item_definitions(dir: &Path, items: &mut HashSet<String>) {
    // Match "item ItemName" but NOT "item 1 Base.Something" (recipe ingredient lines)
    let item_def_re = Regex::new(r"(?i)^\s*item\s+(\w+)\s*$|^\s*item\s+(\w+)\s*\{").unwrap();
    // Match "fluid FluidName" definitions
    let fluid_def_re = Regex::new(r"(?i)^\s*fluid\s+(\w+)\s*$|^\s*fluid\s+(\w+)\s*\{").unwrap();
    // Match "module Name" on one line, OR just "module" alone (name on next line)
    let module_re = Regex::new(r"^\s*module\s+(\w+)").unwrap();
    let module_alone_re = Regex::new(r"^\s*module\s*$").unwrap();
    // Match a bare module name on its own line (e.g. "RPGSkillTree {" or "RPGSkillTree")
    let module_name_re = Regex::new(r"^\s*(\w+)\s*\{?\s*$").unwrap();

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "txt"))
    {
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut current_module = String::new();
        let mut expect_module_name = false;
        for line in content.lines() {
            let trimmed = line.trim();

            // Handle "module\nName {" split across two lines
            if expect_module_name {
                expect_module_name = false;
                if let Some(caps) = module_name_re.captures(trimmed) {
                    current_module = caps[1].to_string();
                    continue;
                }
            }
            if module_alone_re.is_match(trimmed) {
                expect_module_name = true;
                continue;
            }
            if let Some(caps) = module_re.captures(trimmed) {
                current_module = caps[1].to_string();
            }

            if let Some(caps) = item_def_re.captures(trimmed) {
                let item_name = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str());
                if let Some(name) = item_name {
                    if !current_module.is_empty() {
                        items.insert(format!("{}.{}", current_module, name));
                    }
                }
            }
            // Also collect fluid definitions (fluids are referenced by name without module prefix)
            if let Some(caps) = fluid_def_re.captures(trimmed) {
                let fluid_name = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str());
                if let Some(name) = fluid_name {
                    // Store fluids with a "fluid:" prefix to distinguish from items
                    items.insert(format!("fluid:{}", name));
                }
            }
        }
    }
}

/// Reference properties in item definitions that point to other items.
const ITEM_REF_PROPERTIES: &[&str] = &[
    "AmmoType", "ReplaceOnUse", "ReplaceOnUseOn", "ReplaceOnDeplete",
    "ReplaceInPrimaryHand", "ReplaceInSecondHand",
    "MountOn", "ClothingItem", "ClothingItemExtra", "ClothingItemExtraOption",
    "Require", "Fixer",
];

/// Scan a mod's script files for references to items not in the dictionary.
pub fn check_mod_references(
    source_path: &Path,
    mod_id: &str,
    mod_name: &str,
    active_version_folder: Option<&str>,
    known_items: &HashSet<String>,
) -> Vec<MissingItemRef> {
    let mut missing = Vec::new();

    let mut script_dirs = Vec::new();
    if let Some(vf) = active_version_folder {
        let d = source_path.join(vf).join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }
    let common = source_path.join("common").join("media").join("scripts");
    if common.exists() { script_dirs.push(common); }
    if script_dirs.is_empty() {
        let d = source_path.join("media").join("scripts");
        if d.exists() { script_dirs.push(d); }
    }

    // Regex to match Module.ItemName references
    // Match Module.ItemName — item names can start with digits (e.g. Base.223Bullets)
    let item_ref_re = Regex::new(r"\b([A-Z]\w+)\.(\w+)\b").unwrap();
    let module_re = Regex::new(r"^\s*module\s+(\w+)").unwrap();
    let block_re = Regex::new(r"(?i)^\s*(item|craftRecipe|recipe|fixing|evolvedrecipe)\s+(.+?)(?:\s*\{)?$").unwrap();
    let fixing_block_re = Regex::new(r"(?i)^\s*fixing\s+").unwrap();
    // Detect vehicle/template blocks to skip (not item references)
    let vehicle_block_re = Regex::new(r"(?i)^\s*(vehicle|template|part|model|anim|skin|physics|wheel|area|passenger|sound|lightbar|entity|component)\s+").unwrap();
    // Match fluid references: "-fluid 0.1 [FluidName]" or "-fluid 0.1 [Fluid1;Fluid2]"
    let fluid_ref_re = Regex::new(r"-fluid\s+[\d.]+\s+\[([^\]]+)\]").unwrap();
    let prop_re = Regex::new(r"^\s*(\w+)\s*=\s*(.*)").unwrap();
    // Recipe ingredient line: "item N Base.X" or "item N [Base.X;Base.Y]"
    let ingredient_re = Regex::new(r"^\s*item\s+\d+").unwrap();

    // Modules that are NOT item namespaces (Lua function namespaces, engine refs, etc.)
    let non_item_modules: HashSet<&str> = [
        "mode", "flags", "mappers", "mapper", "tags", "base",
        "Vehicles", "BuildRecipeCode", "ISBuildMenu", "ISBuild",
        "ISInventoryPaneContextMenu", "ISToolTipInv",
        "SandboxVars", "Events", "luautils", "getText",
        "TimedActionOnIsValid", "OnIsValid", "OnCreate",
    ].iter().copied().collect();

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

            let rel_path = path.strip_prefix(source_path).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy().to_string();
            let mut current_module = String::new();
            let mut current_block = String::new();
            let mut brace_depth = 0i32;
            let mut in_recipe = false;     // craftRecipe/recipe — crashes on missing items
            let mut in_fixing = false;     // fixing — just hides repair option, no crash
            let mut in_vehicle_block = false;

            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                // Track module
                if let Some(caps) = module_re.captures(trimmed) {
                    current_module = caps[1].to_string();
                }

                // Track block (item, recipe, fixing)
                if let Some(caps) = block_re.captures(trimmed) {
                    let block_type = caps[1].to_string();
                    let block_name = caps[2].trim().to_string();
                    // "item 4 Base.223Bullets," is a recipe ingredient, NOT an item definition.
                    // Item definitions have names starting with a letter/underscore.
                    let is_ingredient_line = block_type.eq_ignore_ascii_case("item")
                        && block_name.chars().next().map_or(false, |c| c.is_ascii_digit());
                    if !is_ingredient_line {
                        current_block = format!("{} {}", block_type, block_name);
                        let bt = block_type.to_lowercase();
                        in_recipe = matches!(bt.as_str(), "craftrecipe" | "recipe" | "evolvedrecipe");
                        in_fixing = bt == "fixing";
                        in_vehicle_block = false;
                    }
                } else if vehicle_block_re.is_match(trimmed) {
                    in_vehicle_block = true;
                }

                brace_depth += trimmed.matches('{').count() as i32;
                brace_depth -= trimmed.matches('}').count() as i32;
                if brace_depth <= 0 {
                    current_block.clear();
                    in_recipe = false;
                    in_fixing = false;
                    in_vehicle_block = false;
                }

                // Skip lines that are just module/block declarations
                if trimmed.starts_with("module ") || trimmed == "{" || trimmed == "}" {
                    continue;
                }

                // Skip vehicle/template block internals — they use different reference systems
                if in_vehicle_block {
                    continue;
                }

                // Check recipe ingredient lines: "item N Base.X" or "item N [Base.X;Base.Y]"
                if brace_depth >= 2 && in_recipe && ingredient_re.is_match(trimmed) {
                    for caps in item_ref_re.captures_iter(trimmed) {
                        let module = &caps[1];
                        let item = &caps[2];
                        let full_ref = format!("{}.{}", module, item);

                        if non_item_modules.contains(module.as_ref() as &str) {
                            continue;
                        }

                        if !known_items.contains(&full_ref) {
                            missing.push(MissingItemRef {
                                mod_id: mod_id.to_string(),
                                mod_name: mod_name.to_string(),
                                file: rel_str.clone(),
                                line: (line_num + 1) as u32,
                                context: "recipe".into(),
                                block_name: current_block.clone(),
                                property: "ingredient".into(),
                                referenced_item: full_ref,
                                severity: "error".into(), // recipes cause crashes
                            });
                        }
                    }
                    continue;
                }

                // Check fluid references: "-fluid 0.1 [FluidName]"
                if brace_depth >= 2 && in_recipe {
                    if let Some(caps) = fluid_ref_re.captures(trimmed) {
                        let fluid_list = &caps[1];
                        for fluid_name in fluid_list.split(';') {
                            let fluid_name = fluid_name.trim();
                            if !fluid_name.is_empty() && !known_items.contains(&format!("fluid:{}", fluid_name)) {
                                missing.push(MissingItemRef {
                                    mod_id: mod_id.to_string(),
                                    mod_name: mod_name.to_string(),
                                    file: rel_str.clone(),
                                    line: (line_num + 1) as u32,
                                    context: "recipe_fluid".into(),
                                    block_name: current_block.clone(),
                                    property: "fluid".into(),
                                    referenced_item: format!("fluid:{}", fluid_name),
                                    severity: "error".into(),
                                });
                            }
                        }
                    }
                }

                // Check item/recipe/fixing properties that reference other items
                if brace_depth >= 2 && (in_recipe || in_fixing) {
                    if let Some(prop_caps) = prop_re.captures(line) {
                        let prop_name = prop_caps[1].to_string();
                        let value = &prop_caps[2];

                        let is_ref_property = ITEM_REF_PROPERTIES.iter()
                            .any(|&p| p.eq_ignore_ascii_case(&prop_name));

                        if is_ref_property {
                            for caps in item_ref_re.captures_iter(value) {
                                let module = &caps[1];
                                let item = &caps[2];
                                let full_ref = format!("{}.{}", module, item);

                                if non_item_modules.contains(module.as_ref() as &str) {
                                    continue;
                                }

                                if !known_items.contains(&full_ref) {
                                    // Recipes crash on missing items; fixing scripts just hide the repair option
                                    let (severity, context) = if in_recipe {
                                        ("error", "recipe")
                                    } else {
                                        ("warning", "fixing")
                                    };
                                    missing.push(MissingItemRef {
                                        mod_id: mod_id.to_string(),
                                        mod_name: mod_name.to_string(),
                                        file: rel_str.clone(),
                                        line: (line_num + 1) as u32,
                                        context: context.into(),
                                        block_name: current_block.clone(),
                                        property: prop_name.clone(),
                                        referenced_item: full_ref,
                                        severity: severity.into(),
                                    });
                                }
                            }
                        }
                    }
                }

                // Check itemMapper entries inside recipe blocks only: "Output.Item = Input.Item"
                if brace_depth >= 3 && in_recipe && trimmed.contains('=')
                    && !trimmed.starts_with("//") && !trimmed.starts_with("--")
                {
                    for caps in item_ref_re.captures_iter(trimmed) {
                        let module = &caps[1];
                        let item = &caps[2];
                        let full_ref = format!("{}.{}", module, item);

                        if non_item_modules.contains(module.as_ref() as &str) {
                            continue;
                        }

                        if !known_items.contains(&full_ref) {
                            missing.push(MissingItemRef {
                                mod_id: mod_id.to_string(),
                                mod_name: mod_name.to_string(),
                                file: rel_str.clone(),
                                line: (line_num + 1) as u32,
                                context: "item_mapper".into(),
                                block_name: current_block.clone(),
                                property: "mapper_entry".into(),
                                referenced_item: full_ref,
                                severity: "error".into(),
                            });
                        }
                    }
                }
            }
        }
    }

    missing
}
