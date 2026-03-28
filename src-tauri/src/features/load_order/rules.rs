use std::collections::{HashMap, HashSet};

use crate::app_core::types::{IssueSeverity, LoadOrderIssue, ModCategory};

use super::topo_sort::topological_sort;

// ─── Tier helpers ──────────────────────────────────────────────────────────────

fn get_tier(category: &Option<ModCategory>) -> u8 {
    match category {
        Some(ModCategory::Framework) => 1,
        Some(ModCategory::Map) => 2,
        Some(ModCategory::Overhaul) => 4,
        _ => 3, // Content and None default to tier 3
    }
}

// ─── Tier-based sorting ────────────────────────────────────────────────────────

/// Sort mods by tier first, then topologically within each tier.
///
/// Tier 1: Framework
/// Tier 2: Map
/// Tier 3: Content (default for unknown/None)
/// Tier 4: Overhaul
///
/// Returns `Ok(sorted_ids)` or `Err(AppError::Validation(...))` if any tier contains a cycle.
pub fn sort_with_tiers(
    mod_ids: &[String],
    mod_categories: &HashMap<String, Option<ModCategory>>,
    dependencies: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>, crate::app_core::error::AppError> {
    // Group mods by tier
    let mut tier_groups: HashMap<u8, Vec<String>> = HashMap::new();
    for id in mod_ids {
        let cat = mod_categories.get(id);
        // cat is Option<&Option<ModCategory>>; flatten to Option<&ModCategory>
        let tier = get_tier(&cat.and_then(|c| c.clone()));
        tier_groups.entry(tier).or_insert_with(Vec::new).push(id.clone());
    }

    let mut result: Vec<String> = Vec::with_capacity(mod_ids.len());

    for tier in [1u8, 2, 3, 4] {
        if let Some(group) = tier_groups.get(&tier) {
            match topological_sort(group, dependencies) {
                Ok(sorted) => result.extend(sorted),
                Err(cycle_nodes) => {
                    return Err(crate::app_core::error::AppError::Validation(format!(
                        "Circular dependency detected in tier {} mods: {}",
                        tier,
                        cycle_nodes.join(", ")
                    )));
                }
            }
        }
    }

    Ok(result)
}

// ─── Validation ────────────────────────────────────────────────────────────────

/// Validate a load order for issues.
/// Returns a (possibly empty) list of issues.
pub fn validate_load_order(
    load_order: &[String],
    dependencies: &HashMap<String, Vec<String>>,
    all_known_mod_ids: &HashSet<String>,
) -> Vec<LoadOrderIssue> {
    let mut issues = Vec::new();

    let position: HashMap<&str, usize> = load_order
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    for (i, mod_id) in load_order.iter().enumerate() {
        if let Some(deps) = dependencies.get(mod_id) {
            for dep_id in deps {
                if !load_order.contains(dep_id) {
                    // Dependency is absent from the active load order
                    if all_known_mod_ids.contains(dep_id) {
                        issues.push(LoadOrderIssue {
                            severity: IssueSeverity::Error,
                            mod_id: mod_id.clone(),
                            message: format!(
                                "Missing dependency: {} requires {} but it is not enabled",
                                mod_id, dep_id
                            ),
                            suggestion: Some(format!("Enable {}", dep_id)),
                            related_mod_id: Some(dep_id.clone()),
                        });
                    } else {
                        issues.push(LoadOrderIssue {
                            severity: IssueSeverity::Warning,
                            mod_id: mod_id.clone(),
                            message: format!(
                                "Unknown dependency: {} requires {} which is not installed",
                                mod_id, dep_id
                            ),
                            suggestion: Some(format!(
                                "Install {} from Steam Workshop",
                                dep_id
                            )),
                            related_mod_id: Some(dep_id.clone()),
                        });
                    }
                } else if let Some(&dep_pos) = position.get(dep_id.as_str()) {
                    if dep_pos > i {
                        // Dependency loads after the mod that requires it
                        issues.push(LoadOrderIssue {
                            severity: IssueSeverity::Warning,
                            mod_id: mod_id.clone(),
                            message: format!(
                                "Load order issue: {} should load after {}",
                                mod_id, dep_id
                            ),
                            suggestion: Some(format!(
                                "Move {} below {}",
                                mod_id, dep_id
                            )),
                            related_mod_id: Some(dep_id.clone()),
                        });
                    }
                }
            }
        }
    }

    issues
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_load_order ────────────────────────────────────────────────────

    #[test]
    fn test_missing_dependency_flagged() {
        let order = vec!["ModA".into()];
        let mut deps = HashMap::new();
        deps.insert("ModA".into(), vec!["DepX".into()]);
        let known: HashSet<String> = vec!["ModA".into(), "DepX".into()].into_iter().collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn test_load_after_violation() {
        // ModA is before ModB in load order, but ModA requires ModB
        let order = vec!["ModA".into(), "ModB".into()];
        let mut deps = HashMap::new();
        deps.insert("ModA".into(), vec!["ModB".into()]);
        let known: HashSet<String> = vec!["ModA".into(), "ModB".into()].into_iter().collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn test_valid_order_no_issues() {
        // ModB loads before ModA; ModA requires ModB — correct order
        let order = vec!["ModB".into(), "ModA".into()];
        let mut deps = HashMap::new();
        deps.insert("ModA".into(), vec!["ModB".into()]);
        let known: HashSet<String> = vec!["ModA".into(), "ModB".into()].into_iter().collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_unknown_dependency_warning() {
        let order = vec!["ModA".into()];
        let mut deps = HashMap::new();
        deps.insert("ModA".into(), vec!["UnknownMod".into()]);
        let known: HashSet<String> = vec!["ModA".into()].into_iter().collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert!(issues[0].message.contains("not installed"));
    }

    #[test]
    fn test_no_issues_no_deps() {
        let order = vec!["ModA".into(), "ModB".into(), "ModC".into()];
        let deps: HashMap<String, Vec<String>> = HashMap::new();
        let known: HashSet<String> = order.iter().cloned().collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_multiple_issues() {
        // ModA missing DepX (known), ModB depends on UnknownMod (not known)
        let order = vec!["ModA".into(), "ModB".into()];
        let mut deps = HashMap::new();
        deps.insert("ModA".into(), vec!["DepX".into()]);
        deps.insert("ModB".into(), vec!["UnknownMod".into()]);
        let known: HashSet<String> = vec!["ModA".into(), "ModB".into(), "DepX".into()]
            .into_iter()
            .collect();
        let issues = validate_load_order(&order, &deps, &known);
        assert_eq!(issues.len(), 2);
        let severities: Vec<&IssueSeverity> = issues.iter().map(|i| &i.severity).collect();
        assert!(severities.contains(&&IssueSeverity::Error));
        assert!(severities.contains(&&IssueSeverity::Warning));
    }

    // ── sort_with_tiers ────────────────────────────────────────────────────────

    #[test]
    fn test_tier_ordering_framework_before_content() {
        let mod_ids = vec!["ContentMod".into(), "FrameworkMod".into()];
        let mut cats = HashMap::new();
        cats.insert("ContentMod".into(), Some(ModCategory::Content));
        cats.insert("FrameworkMod".into(), Some(ModCategory::Framework));
        let deps = HashMap::new();
        let result = sort_with_tiers(&mod_ids, &cats, &deps).unwrap();
        assert_eq!(result[0], "FrameworkMod");
        assert_eq!(result[1], "ContentMod");
    }

    #[test]
    fn test_tier_full_order() {
        let mod_ids = vec![
            "Overhaul".into(),
            "Content".into(),
            "Map".into(),
            "Framework".into(),
        ];
        let mut cats = HashMap::new();
        cats.insert("Overhaul".into(), Some(ModCategory::Overhaul));
        cats.insert("Content".into(), Some(ModCategory::Content));
        cats.insert("Map".into(), Some(ModCategory::Map));
        cats.insert("Framework".into(), Some(ModCategory::Framework));
        let deps = HashMap::new();
        let result = sort_with_tiers(&mod_ids, &cats, &deps).unwrap();
        assert_eq!(result, vec!["Framework", "Map", "Content", "Overhaul"]);
    }

    #[test]
    fn test_tier_cycle_returns_error() {
        let mod_ids = vec!["A".into(), "B".into()];
        let mut cats = HashMap::new();
        cats.insert("A".into(), Some(ModCategory::Content));
        cats.insert("B".into(), Some(ModCategory::Content));
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["B".into()]);
        deps.insert("B".into(), vec!["A".into()]);
        let result = sort_with_tiers(&mod_ids, &cats, &deps);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::app_core::error::AppError::Validation(_)));
    }

    #[test]
    fn test_none_category_defaults_to_content_tier() {
        // A mod with no category and a Framework mod; none-category mod should come after framework
        let mod_ids = vec!["NoCategory".into(), "FrameworkMod".into()];
        let mut cats = HashMap::new();
        cats.insert("NoCategory".into(), None);
        cats.insert("FrameworkMod".into(), Some(ModCategory::Framework));
        let deps = HashMap::new();
        let result = sort_with_tiers(&mod_ids, &cats, &deps).unwrap();
        assert_eq!(result[0], "FrameworkMod");
        assert_eq!(result[1], "NoCategory");
    }
}
