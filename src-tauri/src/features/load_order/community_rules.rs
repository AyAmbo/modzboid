use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::app_core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityRule {
    pub mod_id: String,
    #[serde(default)]
    pub load_before: Vec<String>,   // this mod should load before these
    #[serde(default)]
    pub load_after: Vec<String>,    // this mod should load after these
    #[serde(default)]
    pub notes: String,              // human-readable explanation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityRulesDb {
    pub version: u32,
    pub rules: Vec<CommunityRule>,
}

impl CommunityRulesDb {
    pub fn empty() -> Self {
        CommunityRulesDb {
            version: 1,
            rules: vec![],
        }
    }

    /// Convert rules into additional dependency edges for the sort algorithm.
    /// Returns a map: mod_id -> Vec<mod_ids it should load after>
    /// (same format as the `requires` dependency map)
    pub fn to_dependency_edges(&self) -> HashMap<String, Vec<String>> {
        let mut edges: HashMap<String, Vec<String>> = HashMap::new();

        for rule in &self.rules {
            // load_after means this mod depends on those mods (should come after them)
            if !rule.load_after.is_empty() {
                edges
                    .entry(rule.mod_id.clone())
                    .or_default()
                    .extend(rule.load_after.clone());
            }
            // load_before means those mods depend on this mod (should come after it)
            for target in &rule.load_before {
                edges
                    .entry(target.clone())
                    .or_default()
                    .push(rule.mod_id.clone());
            }
        }

        edges
    }
}

/// Load community rules from a JSON file. Returns empty if missing/malformed.
pub fn load_community_rules(app_data_dir: &Path) -> CommunityRulesDb {
    let path = app_data_dir.join("community_rules.json");
    if !path.exists() {
        return CommunityRulesDb::empty();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or_else(|_| CommunityRulesDb::empty())
        }
        Err(_) => CommunityRulesDb::empty(),
    }
}

/// Save community rules to a JSON file.
pub fn save_community_rules(app_data_dir: &Path, db: &CommunityRulesDb) -> Result<(), AppError> {
    let path = app_data_dir.join("community_rules.json");
    let content = serde_json::to_string_pretty(db)?;
    std::fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_rules() {
        let db = CommunityRulesDb::empty();
        assert!(db.rules.is_empty());
        let edges = db.to_dependency_edges();
        assert!(edges.is_empty());
    }

    #[test]
    fn test_load_after_creates_dep_edge() {
        let db = CommunityRulesDb {
            version: 1,
            rules: vec![CommunityRule {
                mod_id: "ModB".into(),
                load_before: vec![],
                load_after: vec!["ModA".into()],
                notes: "ModB needs ModA loaded first".into(),
            }],
        };
        let edges = db.to_dependency_edges();
        assert_eq!(edges.get("ModB").unwrap(), &vec!["ModA".to_string()]);
    }

    #[test]
    fn test_load_before_creates_reverse_edge() {
        let db = CommunityRulesDb {
            version: 1,
            rules: vec![CommunityRule {
                mod_id: "Framework".into(),
                load_before: vec!["ModX".into(), "ModY".into()],
                load_after: vec![],
                notes: "Framework should load before content mods".into(),
            }],
        };
        let edges = db.to_dependency_edges();
        // ModX and ModY should have Framework as a dependency
        assert!(edges.get("ModX").unwrap().contains(&"Framework".to_string()));
        assert!(edges.get("ModY").unwrap().contains(&"Framework".to_string()));
    }

    #[test]
    fn test_load_missing_file() {
        let db = load_community_rules(Path::new("/nonexistent"));
        assert!(db.rules.is_empty());
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = CommunityRulesDb {
            version: 1,
            rules: vec![CommunityRule {
                mod_id: "TestMod".into(),
                load_before: vec!["Other".into()],
                load_after: vec!["Base".into()],
                notes: "test".into(),
            }],
        };
        save_community_rules(dir.path(), &db).unwrap();
        let loaded = load_community_rules(dir.path());
        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(loaded.rules[0].mod_id, "TestMod");
    }

    #[test]
    fn test_combined_edges() {
        let db = CommunityRulesDb {
            version: 1,
            rules: vec![
                CommunityRule {
                    mod_id: "A".into(),
                    load_before: vec!["C".into()],
                    load_after: vec!["B".into()],
                    notes: String::new(),
                },
            ],
        };
        let edges = db.to_dependency_edges();
        // A should load after B
        assert!(edges.get("A").unwrap().contains(&"B".to_string()));
        // C should load after A
        assert!(edges.get("C").unwrap().contains(&"A".to_string()));
    }
}
