use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::app_core::types::{IncompatDb, ModInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub passed: bool,
    pub checks: Vec<PreflightCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightCheck {
    pub name: String,
    pub status: String, // "pass", "warn", "fail"
    pub message: String,
    pub details: Vec<String>,
}

/// Run pre-flight checks on a profile's load order.
///
/// Checks:
/// 1. Empty load order
/// 2. Missing mods (in load order but not in all_mods)
/// 3. Missing dependencies
/// 4. Known incompatibilities
/// 5. Game version compatibility
pub fn run_preflight(
    profile_load_order: &[String],
    all_mods: &[ModInfo],
    incompat_db: &IncompatDb,
    game_version: Option<&str>,
) -> PreflightResult {
    let mut checks = Vec::new();

    // Build lookup sets
    let all_mod_ids: HashSet<&str> = all_mods.iter().map(|m| m.id.as_str()).collect();
    let enabled_set: HashSet<&str> = profile_load_order.iter().map(|s| s.as_str()).collect();

    // 1. Empty check
    checks.push(check_empty(profile_load_order));

    // 2. Missing mods check
    checks.push(check_missing_mods(profile_load_order, &all_mod_ids));

    // 3. Dependency check
    checks.push(check_dependencies(
        profile_load_order,
        all_mods,
        &enabled_set,
    ));

    // 4. Incompatibility check
    checks.push(check_incompatibilities(
        profile_load_order,
        &enabled_set,
        incompat_db,
    ));

    // 5. Version check
    checks.push(check_versions(
        profile_load_order,
        all_mods,
        game_version,
    ));

    let passed = !checks.iter().any(|c| c.status == "fail");

    PreflightResult { passed, checks }
}

fn check_empty(load_order: &[String]) -> PreflightCheck {
    if load_order.is_empty() {
        PreflightCheck {
            name: "Mod Count".into(),
            status: "warn".into(),
            message: "No mods enabled".into(),
            details: vec!["The load order is empty. Add mods to your profile.".into()],
        }
    } else {
        PreflightCheck {
            name: "Mod Count".into(),
            status: "pass".into(),
            message: format!("{} mod(s) enabled", load_order.len()),
            details: vec![],
        }
    }
}

fn check_missing_mods(load_order: &[String], all_mod_ids: &HashSet<&str>) -> PreflightCheck {
    let missing: Vec<String> = load_order
        .iter()
        .filter(|id| !all_mod_ids.contains(id.as_str()))
        .cloned()
        .collect();

    if missing.is_empty() {
        PreflightCheck {
            name: "Installed Mods".into(),
            status: "pass".into(),
            message: "All enabled mods are installed".into(),
            details: vec![],
        }
    } else {
        PreflightCheck {
            name: "Installed Mods".into(),
            status: "fail".into(),
            message: format!("{} mod(s) not found on disk", missing.len()),
            details: missing
                .iter()
                .map(|id| format!("'{}' is in the load order but not installed", id))
                .collect(),
        }
    }
}

fn check_dependencies(
    load_order: &[String],
    all_mods: &[ModInfo],
    enabled_set: &HashSet<&str>,
) -> PreflightCheck {
    let mut missing_deps: Vec<String> = Vec::new();

    for mod_id in load_order {
        // Find the mod info
        if let Some(mod_info) = all_mods.iter().find(|m| m.id == *mod_id) {
            for req in &mod_info.requires {
                if !enabled_set.contains(req.as_str()) {
                    missing_deps.push(format!(
                        "'{}' requires '{}' which is not in the load order",
                        mod_id, req
                    ));
                }
            }
        }
    }

    if missing_deps.is_empty() {
        PreflightCheck {
            name: "Dependencies".into(),
            status: "pass".into(),
            message: "All dependencies satisfied".into(),
            details: vec![],
        }
    } else {
        PreflightCheck {
            name: "Dependencies".into(),
            status: "fail".into(),
            message: format!("{} missing dependency(ies)", missing_deps.len()),
            details: missing_deps,
        }
    }
}

fn check_incompatibilities(
    _load_order: &[String],
    enabled_set: &HashSet<&str>,
    incompat_db: &IncompatDb,
) -> PreflightCheck {
    let mut found: Vec<String> = Vec::new();

    for entry in &incompat_db.incompatibilities {
        if enabled_set.contains(entry.mod_a.as_str())
            && enabled_set.contains(entry.mod_b.as_str())
        {
            found.push(format!(
                "'{}' and '{}' are incompatible: {}",
                entry.mod_a, entry.mod_b, entry.reason
            ));
        }
    }

    if found.is_empty() {
        PreflightCheck {
            name: "Incompatibilities".into(),
            status: "pass".into(),
            message: "No known incompatibilities".into(),
            details: vec![],
        }
    } else {
        PreflightCheck {
            name: "Incompatibilities".into(),
            status: "warn".into(),
            message: format!("{} known incompatibility(ies)", found.len()),
            details: found,
        }
    }
}

fn check_versions(
    load_order: &[String],
    all_mods: &[ModInfo],
    game_version: Option<&str>,
) -> PreflightCheck {
    let game_ver = match game_version {
        Some(v) => v,
        None => {
            return PreflightCheck {
                name: "Version Compatibility".into(),
                status: "pass".into(),
                message: "Game version not configured, skipping version checks".into(),
                details: vec![],
            };
        }
    };

    let enabled_set: HashSet<&str> = load_order.iter().map(|s| s.as_str()).collect();
    let mut warnings: Vec<String> = Vec::new();

    for m in all_mods {
        if !enabled_set.contains(m.id.as_str()) {
            continue;
        }

        if let Some(ref min) = m.version_min {
            if version_greater_than(min, game_ver) {
                warnings.push(format!(
                    "'{}' requires game version {} or higher (current: {})",
                    m.id, min, game_ver
                ));
            }
        }

        if let Some(ref max) = m.version_max {
            if version_greater_than(game_ver, max) {
                warnings.push(format!(
                    "'{}' supports up to version {} (current: {})",
                    m.id, max, game_ver
                ));
            }
        }
    }

    if warnings.is_empty() {
        PreflightCheck {
            name: "Version Compatibility".into(),
            status: "pass".into(),
            message: "All mods compatible with current game version".into(),
            details: vec![],
        }
    } else {
        PreflightCheck {
            name: "Version Compatibility".into(),
            status: "warn".into(),
            message: format!("{} version warning(s)", warnings.len()),
            details: warnings,
        }
    }
}

/// Compare two version strings (major.minor format). Returns true if a > b.
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
    use crate::app_core::types::{IncompatEntry, ModSource};
    use std::path::PathBuf;

    fn make_mod(id: &str, requires: &[&str]) -> ModInfo {
        ModInfo {
            id: id.to_string(),
            raw_id: id.to_string(),
            workshop_id: None,
            name: id.to_string(),
            description: String::new(),
            authors: vec![],
            url: None,
            mod_version: None,
            poster_path: None,
            icon_path: None,
            version_min: None,
            version_max: None,
            version_folders: vec![],
            active_version_folder: None,
            requires: requires.iter().map(|s| s.to_string()).collect(),
            pack: None,
            tile_def: vec![],
            category: None,
            source: ModSource::Local,
            source_path: PathBuf::from("/mods"),
            mod_info_path: PathBuf::from("/mods/mod.info"),
            size_bytes: None,
            last_modified: String::new(),
            detected_category: None,
        }
    }

    fn make_mod_with_versions(id: &str, min: Option<&str>, max: Option<&str>) -> ModInfo {
        let mut m = make_mod(id, &[]);
        m.version_min = min.map(|s| s.to_string());
        m.version_max = max.map(|s| s.to_string());
        m
    }

    fn empty_incompat_db() -> IncompatDb {
        IncompatDb {
            version: 1,
            incompatibilities: vec![],
        }
    }

    #[test]
    fn test_empty_load_order() {
        let result = run_preflight(&[], &[], &empty_incompat_db(), None);
        // Not a fail, but a warn
        assert!(result.passed);
        let empty_check = result.checks.iter().find(|c| c.name == "Mod Count").unwrap();
        assert_eq!(empty_check.status, "warn");
    }

    #[test]
    fn test_all_pass() {
        let mods = vec![make_mod("ModA", &[]), make_mod("ModB", &["ModA"])];
        let order = vec!["ModA".into(), "ModB".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), None);
        assert!(result.passed);
        assert!(result.checks.iter().all(|c| c.status != "fail"));
    }

    #[test]
    fn test_missing_mod_fails() {
        let mods = vec![make_mod("ModA", &[])];
        let order = vec!["ModA".into(), "ModX".into()]; // ModX not installed
        let result = run_preflight(&order, &mods, &empty_incompat_db(), None);
        assert!(!result.passed);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Installed Mods")
            .unwrap();
        assert_eq!(check.status, "fail");
        assert!(check.details[0].contains("ModX"));
    }

    #[test]
    fn test_missing_dependency_fails() {
        let mods = vec![
            make_mod("ModA", &[]),
            make_mod("ModB", &["ModC"]), // ModC not in load order
        ];
        let order = vec!["ModA".into(), "ModB".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), None);
        assert!(!result.passed);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Dependencies")
            .unwrap();
        assert_eq!(check.status, "fail");
        assert!(check.details[0].contains("ModC"));
    }

    #[test]
    fn test_incompatibility_warns() {
        let mods = vec![make_mod("ModA", &[]), make_mod("ModB", &[])];
        let order = vec!["ModA".into(), "ModB".into()];
        let db = IncompatDb {
            version: 1,
            incompatibilities: vec![IncompatEntry {
                mod_a: "ModA".into(),
                mod_b: "ModB".into(),
                reason: "They conflict".into(),
                severity: "warning".into(),
            }],
        };
        let result = run_preflight(&order, &mods, &db, None);
        // Incompatibility is a warn, not fail → still passes
        assert!(result.passed);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Incompatibilities")
            .unwrap();
        assert_eq!(check.status, "warn");
        assert!(check.details[0].contains("They conflict"));
    }

    #[test]
    fn test_version_min_warns() {
        let mods = vec![make_mod_with_versions("ModA", Some("43.0"), None)];
        let order = vec!["ModA".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), Some("42.15"));
        assert!(result.passed); // version is a warn, not fail
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Version Compatibility")
            .unwrap();
        assert_eq!(check.status, "warn");
        assert!(check.details[0].contains("43.0"));
    }

    #[test]
    fn test_version_max_warns() {
        let mods = vec![make_mod_with_versions("ModA", None, Some("41.78"))];
        let order = vec!["ModA".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), Some("42.15"));
        assert!(result.passed);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Version Compatibility")
            .unwrap();
        assert_eq!(check.status, "warn");
        assert!(check.details[0].contains("41.78"));
    }

    #[test]
    fn test_version_within_range_passes() {
        let mods = vec![make_mod_with_versions("ModA", Some("42.0"), Some("43.0"))];
        let order = vec!["ModA".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), Some("42.15"));
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Version Compatibility")
            .unwrap();
        assert_eq!(check.status, "pass");
    }

    #[test]
    fn test_no_game_version_skips_version_check() {
        let mods = vec![make_mod_with_versions("ModA", Some("99.0"), None)];
        let order = vec!["ModA".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), None);
        let check = result
            .checks
            .iter()
            .find(|c| c.name == "Version Compatibility")
            .unwrap();
        assert_eq!(check.status, "pass");
        assert!(check.message.contains("not configured"));
    }

    #[test]
    fn test_multiple_failures() {
        let mods = vec![make_mod("ModA", &["MissingDep"])];
        let order = vec!["ModA".into(), "NotInstalled".into()];
        let result = run_preflight(&order, &mods, &empty_incompat_db(), None);
        assert!(!result.passed);
        let fail_count = result.checks.iter().filter(|c| c.status == "fail").count();
        assert_eq!(fail_count, 2); // missing mod + missing dep
    }

    #[test]
    fn test_version_greater_than() {
        assert!(version_greater_than("43.0", "42.15"));
        assert!(version_greater_than("42.16", "42.15"));
        assert!(!version_greater_than("42.15", "42.15"));
        assert!(!version_greater_than("42.14", "42.15"));
    }
}
