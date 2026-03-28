use std::collections::{HashMap, HashSet, VecDeque};

/// Topologically sort mod IDs based on their dependencies using Kahn's algorithm.
/// Returns `Ok(sorted)` in dependency-first order, or `Err(cycle_participants)` if a cycle
/// is detected.
///
/// `mod_ids`: the mods to sort
/// `dependencies`: mod_id → list of mod_ids it depends on (from the `requires` field)
///
/// Mods not in `mod_ids` that appear in dependencies are ignored (they're not enabled).
/// Within the same topological level, mods are sorted alphabetically for deterministic output.
pub fn topological_sort(
    mod_ids: &[String],
    dependencies: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>, Vec<String>> {
    let mod_set: HashSet<&String> = mod_ids.iter().collect();

    // adjacency: dependency → dependents that need it (reverse edges for propagation)
    let mut dependents: HashMap<&String, Vec<&String>> = HashMap::new();
    // in_degree: how many (enabled) dependencies each mod still needs
    let mut in_degree: HashMap<&String, usize> = HashMap::new();

    // Initialise every known mod with in_degree 0
    for id in mod_ids {
        in_degree.entry(id).or_insert(0);
        dependents.entry(id).or_insert_with(Vec::new);
    }

    // Build edges (only between mods in mod_set)
    for id in mod_ids {
        if let Some(deps) = dependencies.get(id) {
            for dep in deps {
                if mod_set.contains(dep) {
                    // dep must load before id
                    dependents.entry(dep).or_insert_with(Vec::new).push(id);
                    *in_degree.entry(id).or_insert(0) += 1;
                }
            }
        }
    }

    // Seed the queue with all mods that have no remaining dependencies.
    // Sort alphabetically so the output is deterministic within each wave.
    let mut queue: VecDeque<&String> = {
        let mut zero: Vec<&String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        zero.sort();
        VecDeque::from(zero)
    };

    let mut sorted: Vec<String> = Vec::with_capacity(mod_ids.len());

    while let Some(node) = queue.pop_front() {
        sorted.push(node.clone());

        // Collect dependents whose in-degree drops to 0, then sort before enqueuing
        let mut newly_zero: Vec<&String> = Vec::new();
        if let Some(deps) = dependents.get(node) {
            for dep in deps {
                let deg = in_degree.get_mut(dep).expect("dep must be in in_degree");
                *deg -= 1;
                if *deg == 0 {
                    newly_zero.push(dep);
                }
            }
        }
        newly_zero.sort();
        for dep in newly_zero {
            queue.push_back(dep);
        }
    }

    if sorted.len() != mod_ids.len() {
        // Some nodes remain — they are part of a cycle
        let sorted_set: HashSet<&String> = sorted.iter().collect();
        let cycle_nodes: Vec<String> = mod_ids
            .iter()
            .filter(|id| !sorted_set.contains(id))
            .cloned()
            .collect();
        return Err(cycle_nodes);
    }

    Ok(sorted)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency_chain() {
        // A requires B, B requires C → sorted: [C, B, A]
        let ids = vec!["A".into(), "B".into(), "C".into()];
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["B".into()]);
        deps.insert("B".into(), vec!["C".into()]);
        let result = topological_sort(&ids, &deps).unwrap();
        assert_eq!(result, vec!["C", "B", "A"]);
    }

    #[test]
    fn test_independent_mods_alphabetical() {
        let ids = vec!["C".into(), "A".into(), "B".into()];
        let deps = HashMap::new();
        let result = topological_sort(&ids, &deps).unwrap();
        assert_eq!(result, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_circular_dependency() {
        // A → B → C → A
        let ids = vec!["A".into(), "B".into(), "C".into()];
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["B".into()]);
        deps.insert("B".into(), vec!["C".into()]);
        deps.insert("C".into(), vec!["A".into()]);
        let result = topological_sort(&ids, &deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_external_deps_ignored() {
        // A requires ExtMod (not in mod_ids) → A still sorted successfully
        let ids = vec!["A".into()];
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["ExtMod".into()]);
        let result = topological_sort(&ids, &deps).unwrap();
        assert_eq!(result, vec!["A"]);
    }

    #[test]
    fn test_diamond_dependency() {
        // A requires B and C, B requires D, C requires D → D first, A last
        let ids = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["B".into(), "C".into()]);
        deps.insert("B".into(), vec!["D".into()]);
        deps.insert("C".into(), vec!["D".into()]);
        let result = topological_sort(&ids, &deps).unwrap();
        assert_eq!(result[0], "D"); // D must be first
        assert_eq!(*result.last().unwrap(), "A"); // A must be last
    }

    #[test]
    fn test_empty_input() {
        let ids: Vec<String> = vec![];
        let deps = HashMap::new();
        let result = topological_sort(&ids, &deps).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_mod_no_deps() {
        let ids = vec!["OnlyMod".into()];
        let deps = HashMap::new();
        let result = topological_sort(&ids, &deps).unwrap();
        assert_eq!(result, vec!["OnlyMod"]);
    }

    #[test]
    fn test_partial_cycle_rest_is_sorted() {
        // D and E are independent; A, B, C form a cycle
        let ids = vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()];
        let mut deps = HashMap::new();
        deps.insert("A".into(), vec!["B".into()]);
        deps.insert("B".into(), vec!["C".into()]);
        deps.insert("C".into(), vec!["A".into()]);
        let result = topological_sort(&ids, &deps);
        assert!(result.is_err());
        let cycle = result.unwrap_err();
        assert_eq!(cycle.len(), 3);
        assert!(cycle.contains(&"A".to_string()));
        assert!(cycle.contains(&"B".to_string()));
        assert!(cycle.contains(&"C".to_string()));
    }
}
