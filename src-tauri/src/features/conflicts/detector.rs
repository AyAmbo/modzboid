use std::collections::{HashMap, HashSet};
use std::path::Path;

use sqlx::sqlite::SqlitePool;

use crate::app_core::error::AppError;
use crate::app_core::types::{
    ConflictType, IncompatDb, IssueSeverity, ModConflict, ModInfo,
};

/// Load incompatibility database from JSON file. Returns empty if missing/malformed.
pub fn load_incompat_db(app_data_dir: &Path) -> IncompatDb {
    let path = app_data_dir.join("incompatibilities.json");
    if !path.exists() {
        return IncompatDb { version: 1, incompatibilities: vec![] };
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(IncompatDb {
            version: 1, incompatibilities: vec![],
        }),
        Err(_) => IncompatDb { version: 1, incompatibilities: vec![] },
    }
}

/// Detect all conflicts among the given set of enabled mod IDs.
pub async fn detect_conflicts(
    pool: &SqlitePool,
    mod_ids: &[String],
    all_mods: &[ModInfo],
    game_version: Option<&str>,
    incompat_db: &IncompatDb,
) -> Result<Vec<ModConflict>, AppError> {
    let mut conflicts = Vec::new();

    if mod_ids.is_empty() {
        return Ok(conflicts);
    }

    let dep_map: HashMap<&str, HashSet<&str>> = all_mods
        .iter()
        .map(|m| {
            let deps: HashSet<&str> = m.requires.iter().map(|s| s.as_str()).collect();
            (m.id.as_str(), deps)
        })
        .collect();

    let mod_id_set: HashSet<&str> = mod_ids.iter().map(|s| s.as_str()).collect();

    // 1. File conflicts
    let file_conflicts = find_file_conflicts(pool, mod_ids).await?;
    for (path, file_type, involved_mods) in &file_conflicts {
        let is_intentional = check_intentional(involved_mods, &dep_map);
        let severity = match file_type.as_str() {
            "lua" | "script" => {
                if is_intentional { IssueSeverity::Info } else { IssueSeverity::Warning }
            }
            _ => IssueSeverity::Info,
        };

        conflicts.push(ModConflict {
            conflict_type: ConflictType::FileOverride,
            severity,
            mod_ids: involved_mods.clone(),
            file_path: Some(path.clone()),
            script_id: None,
            message: format!("File override: {} is present in {}", path, involved_mods.join(", ")),
            suggestion: if is_intentional {
                Some("Intentional override — one mod patches the other".into())
            } else {
                Some("The mod loaded later wins. Adjust load order if needed.".into())
            },
            is_intentional,
        });
    }

    // 2. Script ID conflicts
    let script_conflicts = find_script_conflicts(pool, mod_ids).await?;
    for (script_type, script_id, involved_mods) in &script_conflicts {
        let is_intentional = check_intentional(involved_mods, &dep_map);
        conflicts.push(ModConflict {
            conflict_type: ConflictType::ScriptIdClash,
            severity: if is_intentional { IssueSeverity::Info } else { IssueSeverity::Warning },
            mod_ids: involved_mods.clone(),
            file_path: None,
            script_id: Some(script_id.clone()),
            message: format!("Script ID clash: {} \"{}\" defined in {}", script_type, script_id, involved_mods.join(", ")),
            suggestion: Some("Only the last-loaded definition takes effect.".into()),
            is_intentional,
        });
    }

    // 3. Version mismatches
    if let Some(game_ver) = game_version {
        for m in all_mods {
            if !mod_id_set.contains(m.id.as_str()) { continue; }
            if let Some(ref min) = m.version_min {
                if version_greater_than(min, game_ver) {
                    conflicts.push(ModConflict {
                        conflict_type: ConflictType::VersionMismatch,
                        severity: IssueSeverity::Error,
                        mod_ids: vec![m.id.clone()],
                        file_path: None, script_id: None,
                        message: format!("{} requires game version {} or higher (current: {})", m.name, min, game_ver),
                        suggestion: Some("Update the game or disable this mod.".into()),
                        is_intentional: false,
                    });
                }
            }
            if let Some(ref max) = m.version_max {
                if version_greater_than(game_ver, max) {
                    conflicts.push(ModConflict {
                        conflict_type: ConflictType::VersionMismatch,
                        severity: IssueSeverity::Warning,
                        mod_ids: vec![m.id.clone()],
                        file_path: None, script_id: None,
                        message: format!("{} supports up to version {} (current: {})", m.name, max, game_ver),
                        suggestion: Some("This mod may not work with the current game version.".into()),
                        is_intentional: false,
                    });
                }
            }
        }
    }

    // 4. Lua function override conflicts
    let function_conflicts = find_function_conflicts(pool, mod_ids).await?;
    for (symbol_name, symbol_type, involved_mods) in &function_conflicts {
        let is_intentional = check_intentional(involved_mods, &dep_map);
        conflicts.push(ModConflict {
            conflict_type: ConflictType::FunctionOverride,
            severity: if is_intentional { IssueSeverity::Info } else { IssueSeverity::Warning },
            mod_ids: involved_mods.clone(),
            file_path: None,
            script_id: Some(symbol_name.clone()),
            message: format!("Lua {} \"{}\" defined by multiple mods: {}", symbol_type, symbol_name, involved_mods.join(", ")),
            suggestion: if is_intentional {
                Some("Intentional override — one mod patches the other.".into())
            } else {
                Some("Multiple mods define this symbol. The last-loaded wins, which may cause issues.".into())
            },
            is_intentional,
        });
    }

    // 5. Event hook collisions (info-level — multiple mods hooking same event)
    let event_collisions = find_event_collisions(pool, mod_ids).await?;
    for (event_name, involved_mods) in &event_collisions {
        conflicts.push(ModConflict {
            conflict_type: ConflictType::EventCollision,
            severity: IssueSeverity::Info,
            mod_ids: involved_mods.clone(),
            file_path: None,
            script_id: Some(event_name.clone()),
            message: format!("Event \"{}\" hooked by {} mods: {}", event_name, involved_mods.len(), involved_mods.join(", ")),
            suggestion: Some("Multiple mods hook this event. Usually harmless, but may cause ordering-dependent behavior.".into()),
            is_intentional: false,
        });
    }

    // 6. Known incompatibilities
    for entry in &incompat_db.incompatibilities {
        if mod_id_set.contains(entry.mod_a.as_str()) && mod_id_set.contains(entry.mod_b.as_str()) {
            let severity = if entry.severity == "error" { IssueSeverity::Error } else { IssueSeverity::Warning };
            conflicts.push(ModConflict {
                conflict_type: ConflictType::KnownIncompat,
                severity,
                mod_ids: vec![entry.mod_a.clone(), entry.mod_b.clone()],
                file_path: None, script_id: None,
                message: format!("Known incompatibility: {} and {} — {}", entry.mod_a, entry.mod_b, entry.reason),
                suggestion: Some("Disable one of these mods.".into()),
                is_intentional: false,
            });
        }
    }

    Ok(conflicts)
}

async fn find_file_conflicts(
    pool: &SqlitePool,
    mod_ids: &[String],
) -> Result<Vec<(String, String, Vec<String>)>, AppError> {
    if mod_ids.is_empty() { return Ok(vec![]); }

    let placeholders: Vec<String> = (1..=mod_ids.len()).map(|i| format!("${}", i)).collect();
    let in_clause = placeholders.join(", ");

    let query = format!(
        "SELECT relative_path, file_type, GROUP_CONCAT(mod_id, '|') as mod_ids \
         FROM mod_files WHERE mod_id IN ({}) \
         GROUP BY relative_path HAVING COUNT(DISTINCT mod_id) >= 2",
        in_clause
    );

    let mut q = sqlx::query_as::<_, (String, String, String)>(&query);
    for id in mod_ids { q = q.bind(id); }

    let rows = q.fetch_all(pool).await
        .map_err(|e| AppError::Database(format!("Failed to find file conflicts: {}", e)))?;

    Ok(rows.into_iter().map(|(path, ft, mods)| {
        (path, ft, mods.split('|').map(String::from).collect())
    }).collect())
}

async fn find_script_conflicts(
    pool: &SqlitePool,
    mod_ids: &[String],
) -> Result<Vec<(String, String, Vec<String>)>, AppError> {
    if mod_ids.is_empty() { return Ok(vec![]); }

    let placeholders: Vec<String> = (1..=mod_ids.len()).map(|i| format!("${}", i)).collect();
    let in_clause = placeholders.join(", ");

    let query = format!(
        "SELECT script_type, script_id, GROUP_CONCAT(mod_id, '|') as mod_ids \
         FROM (SELECT DISTINCT mod_id, script_type, script_id FROM script_ids WHERE mod_id IN ({})) \
         GROUP BY script_type, script_id HAVING COUNT(mod_id) >= 2",
        in_clause
    );

    let mut q = sqlx::query_as::<_, (String, String, String)>(&query);
    for id in mod_ids { q = q.bind(id); }

    let rows = q.fetch_all(pool).await
        .map_err(|e| AppError::Database(format!("Failed to find script conflicts: {}", e)))?;

    Ok(rows.into_iter().map(|(st, sid, mods)| {
        (st, sid, mods.split('|').map(String::from).collect())
    }).collect())
}

async fn find_function_conflicts(
    pool: &SqlitePool,
    mod_ids: &[String],
) -> Result<Vec<(String, String, Vec<String>)>, AppError> {
    if mod_ids.is_empty() { return Ok(vec![]); }

    let placeholders: Vec<String> = (1..=mod_ids.len()).map(|i| format!("${}", i)).collect();
    let in_clause = placeholders.join(", ");

    let query = format!(
        "SELECT symbol_name, symbol_type, GROUP_CONCAT(DISTINCT mod_id, '|') as mod_ids \
         FROM lua_globals WHERE mod_id IN ({}) \
         GROUP BY symbol_name HAVING COUNT(DISTINCT mod_id) >= 2",
        in_clause
    );

    let mut q = sqlx::query_as::<_, (String, String, String)>(&query);
    for id in mod_ids { q = q.bind(id); }

    let rows = q.fetch_all(pool).await
        .map_err(|e| AppError::Database(format!("Failed to find function conflicts: {}", e)))?;

    Ok(rows.into_iter().map(|(name, stype, mods)| {
        (name, stype, mods.split('|').map(String::from).collect())
    }).collect())
}

async fn find_event_collisions(
    pool: &SqlitePool,
    mod_ids: &[String],
) -> Result<Vec<(String, Vec<String>)>, AppError> {
    if mod_ids.is_empty() { return Ok(vec![]); }

    let placeholders: Vec<String> = (1..=mod_ids.len()).map(|i| format!("${}", i)).collect();
    let in_clause = placeholders.join(", ");

    // Only flag events hooked by 3+ mods to reduce noise
    let query = format!(
        "SELECT event_name, GROUP_CONCAT(DISTINCT mod_id, '|') as mod_ids \
         FROM event_hooks WHERE mod_id IN ({}) \
         GROUP BY event_name HAVING COUNT(DISTINCT mod_id) >= 3",
        in_clause
    );

    let mut q = sqlx::query_as::<_, (String, String)>(&query);
    for id in mod_ids { q = q.bind(id); }

    let rows = q.fetch_all(pool).await
        .map_err(|e| AppError::Database(format!("Failed to find event collisions: {}", e)))?;

    Ok(rows.into_iter().map(|(name, mods)| {
        (name, mods.split('|').map(String::from).collect())
    }).collect())
}

fn check_intentional(mod_ids: &[String], dep_map: &HashMap<&str, HashSet<&str>>) -> bool {
    for a in mod_ids {
        for b in mod_ids {
            if a == b { continue; }
            if let Some(deps) = dep_map.get(a.as_str()) {
                if deps.contains(b.as_str()) { return true; }
            }
        }
    }
    false
}

fn version_greater_than(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> (u64, u64) {
        let parts: Vec<&str> = v.splitn(3, '.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor)
    };
    parse(a) > parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_greater_than() {
        assert!(version_greater_than("42.15", "42.14"));
        assert!(version_greater_than("43.0", "42.15"));
        assert!(!version_greater_than("42.14", "42.15"));
        assert!(!version_greater_than("42.15", "42.15"));
    }

    #[test]
    fn test_version_with_patch() {
        assert!(version_greater_than("42.15.2", "42.14"));
        assert!(!version_greater_than("42.14", "42.15.2"));
    }

    #[test]
    fn test_check_intentional_with_dep() {
        let mut dep_map: HashMap<&str, HashSet<&str>> = HashMap::new();
        dep_map.insert("PatchMod", ["BaseMod"].iter().cloned().collect());
        dep_map.insert("BaseMod", HashSet::new());
        let mods = vec!["BaseMod".to_string(), "PatchMod".to_string()];
        assert!(check_intentional(&mods, &dep_map));
    }

    #[test]
    fn test_check_intentional_no_dep() {
        let mut dep_map: HashMap<&str, HashSet<&str>> = HashMap::new();
        dep_map.insert("ModA", HashSet::new());
        dep_map.insert("ModB", HashSet::new());
        let mods = vec!["ModA".to_string(), "ModB".to_string()];
        assert!(!check_intentional(&mods, &dep_map));
    }

    #[test]
    fn test_load_incompat_db_missing() {
        let db = load_incompat_db(Path::new("/nonexistent"));
        assert!(db.incompatibilities.is_empty());
        assert_eq!(db.version, 1);
    }
}
