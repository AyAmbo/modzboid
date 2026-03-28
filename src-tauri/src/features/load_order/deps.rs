use std::collections::{HashMap, HashSet, VecDeque};

use crate::app_core::types::DepResolution;
use super::topo_sort::topological_sort;

/// BFS from `mod_id` through the dependency graph.
/// Returns mods that need enabling (installed but not enabled) and mods not installed at all.
pub fn resolve_transitive_deps(
    mod_id: &str,
    dependencies: &HashMap<String, Vec<String>>,
    enabled_set: &HashSet<String>,
    all_known_ids: &HashSet<String>,
) -> DepResolution {
    let mut to_enable = Vec::new();
    let mut not_installed = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Mark the source mod as visited so it's never added to `to_enable`
    visited.insert(mod_id.to_string());

    if let Some(deps) = dependencies.get(mod_id) {
        for dep in deps {
            if !visited.contains(dep.as_str()) {
                visited.insert(dep.clone());
                queue.push_back(dep.clone());
            }
        }
    }

    while let Some(dep_id) = queue.pop_front() {
        if enabled_set.contains(&dep_id) {
            // Already enabled — skip but still explore its deps (transitive)
        } else if all_known_ids.contains(&dep_id) {
            to_enable.push(dep_id.clone());
        } else {
            not_installed.push(dep_id.clone());
            continue;
        }

        if let Some(sub_deps) = dependencies.get(&dep_id) {
            for sub_dep in sub_deps {
                if !visited.contains(sub_dep.as_str()) {
                    visited.insert(sub_dep.clone());
                    queue.push_back(sub_dep.clone());
                }
            }
        }
    }

    // Sort to_enable in topological order
    if to_enable.len() > 1 {
        if let Ok(sorted) = topological_sort(&to_enable, dependencies) {
            to_enable = sorted;
        }
    }

    DepResolution { to_enable, not_installed }
}

/// BFS through the reverse dependency graph.
/// Returns all enabled mods that transitively depend on `mod_id`.
pub fn find_reverse_deps(
    mod_id: &str,
    reverse_deps: &HashMap<String, Vec<String>>,
    enabled_set: &HashSet<String>,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(mod_id.to_string());

    if let Some(dependents) = reverse_deps.get(mod_id) {
        for dep in dependents {
            if enabled_set.contains(dep) && !visited.contains(dep.as_str()) {
                visited.insert(dep.clone());
                queue.push_back(dep.clone());
            }
        }
    }

    while let Some(dep_id) = queue.pop_front() {
        result.push(dep_id.clone());
        if let Some(dependents) = reverse_deps.get(&dep_id) {
            for dep in dependents {
                if enabled_set.contains(dep) && !visited.contains(dep.as_str()) {
                    visited.insert(dep.clone());
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    result
}

/// Build a reverse dependency map: for each mod, which mods depend on it.
pub fn build_reverse_dep_map(
    dependencies: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
    for (mod_id, deps) in dependencies {
        for dep in deps {
            reverse.entry(dep.clone()).or_default().push(mod_id.clone());
        }
    }
    reverse
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deps(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect())).collect()
    }

    fn make_set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_resolve_no_deps() {
        let deps = HashMap::new();
        let enabled = make_set(&[]);
        let known = make_set(&["A"]);
        let result = resolve_transitive_deps("A", &deps, &enabled, &known);
        assert!(result.to_enable.is_empty());
        assert!(result.not_installed.is_empty());
    }

    #[test]
    fn test_resolve_direct_dep() {
        let deps = make_deps(&[("B", &["A"])]);
        let enabled = make_set(&[]);
        let known = make_set(&["A", "B"]);
        let result = resolve_transitive_deps("B", &deps, &enabled, &known);
        assert_eq!(result.to_enable, vec!["A"]);
        assert!(result.not_installed.is_empty());
    }

    #[test]
    fn test_resolve_transitive() {
        let deps = make_deps(&[("C", &["B"]), ("B", &["A"])]);
        let enabled = make_set(&[]);
        let known = make_set(&["A", "B", "C"]);
        let result = resolve_transitive_deps("C", &deps, &enabled, &known);
        assert_eq!(result.to_enable, vec!["A", "B"]);
    }

    #[test]
    fn test_resolve_skips_already_enabled() {
        let deps = make_deps(&[("C", &["B"]), ("B", &["A"])]);
        let enabled = make_set(&["A"]);
        let known = make_set(&["A", "B", "C"]);
        let result = resolve_transitive_deps("C", &deps, &enabled, &known);
        assert_eq!(result.to_enable, vec!["B"]);
    }

    #[test]
    fn test_resolve_not_installed() {
        let deps = make_deps(&[("B", &["Unknown"])]);
        let enabled = make_set(&[]);
        let known = make_set(&["B"]);
        let result = resolve_transitive_deps("B", &deps, &enabled, &known);
        assert!(result.to_enable.is_empty());
        assert_eq!(result.not_installed, vec!["Unknown"]);
    }

    #[test]
    fn test_resolve_circular_deps() {
        let deps = make_deps(&[("A", &["B"]), ("B", &["A"])]);
        let enabled = make_set(&[]);
        let known = make_set(&["A", "B"]);
        let result = resolve_transitive_deps("A", &deps, &enabled, &known);
        assert_eq!(result.to_enable.len(), 1);
        assert!(result.to_enable.contains(&"B".to_string()));
    }

    #[test]
    fn test_reverse_deps_none() {
        let reverse = HashMap::new();
        let enabled = make_set(&["A", "B"]);
        let result = find_reverse_deps("A", &reverse, &enabled);
        assert!(result.is_empty());
    }

    #[test]
    fn test_reverse_deps_direct() {
        let deps = make_deps(&[("B", &["A"])]);
        let reverse = build_reverse_dep_map(&deps);
        let enabled = make_set(&["A", "B"]);
        let result = find_reverse_deps("A", &reverse, &enabled);
        assert_eq!(result, vec!["B"]);
    }

    #[test]
    fn test_reverse_deps_transitive() {
        let deps = make_deps(&[("B", &["A"]), ("C", &["B"])]);
        let reverse = build_reverse_dep_map(&deps);
        let enabled = make_set(&["A", "B", "C"]);
        let result = find_reverse_deps("A", &reverse, &enabled);
        assert!(result.contains(&"B".to_string()));
        assert!(result.contains(&"C".to_string()));
    }

    #[test]
    fn test_reverse_deps_only_enabled() {
        let deps = make_deps(&[("B", &["A"]), ("C", &["A"])]);
        let reverse = build_reverse_dep_map(&deps);
        let enabled = make_set(&["A", "B"]);
        let result = find_reverse_deps("A", &reverse, &enabled);
        assert_eq!(result, vec!["B"]);
    }

    #[test]
    fn test_build_reverse_dep_map() {
        let deps = make_deps(&[("B", &["A"]), ("C", &["A", "B"])]);
        let reverse = build_reverse_dep_map(&deps);
        assert_eq!(reverse.get("A").unwrap().len(), 2);
        assert_eq!(reverse.get("B").unwrap().len(), 1);
    }
}
