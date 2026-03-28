use serde::{Deserialize, Serialize};
use crate::app_core::types::ModInfo;

/// A single mismatch entry where a mod ID doesn't correspond to the expected workshop ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMismatch {
    pub index: usize,
    pub mod_id: String,
    pub workshop_id: String,
}

/// Result of checking Mods= and WorkshopItems= sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCheckResult {
    pub synced: bool,
    pub mod_ids: Vec<String>,
    pub workshop_ids: Vec<String>,
    pub mismatches: Vec<SyncMismatch>,
}

/// Check whether a load order's mod IDs and their workshop IDs are in matching order.
///
/// For each mod in `load_order`, looks up its `workshop_id` in `all_mods`.
/// Local mods (no workshop ID) are included with an empty string for their workshop ID.
/// Returns the parallel lists and any positional mismatches.
pub fn check_mod_workshop_sync(
    load_order: &[String],
    all_mods: &[ModInfo],
) -> SyncCheckResult {
    let mut mod_ids = Vec::with_capacity(load_order.len());
    let mut workshop_ids = Vec::with_capacity(load_order.len());
    let mismatches = Vec::new();

    for mod_id in load_order.iter() {
        let workshop_id = all_mods
            .iter()
            .find(|m| &m.id == mod_id)
            .and_then(|m| m.workshop_id.clone())
            .unwrap_or_default();

        mod_ids.push(mod_id.clone());
        workshop_ids.push(workshop_id.clone());

        // A mismatch is when a mod has a workshop ID but the parallel position
        // doesn't match what we'd expect from a sorted workshop ID order.
        // For the initial check, we verify that the workshop_ids list is consistent
        // with the mod_ids list - they should be in the same relative order.
    }

    // Check: are there any workshop mods whose ordering is inconsistent?
    // The Mods= and WorkshopItems= lines must be parallel, so we check
    // that if we sort by workshop_id, the mod_ids would be in the same order.
    // Actually, the real check is simpler: for any given mod, does it map to the
    // correct workshop ID at that position? Since we build both lists from the
    // same load_order, they are always in sync by construction.
    //
    // The real use case: the user provides EXISTING Mods= and WorkshopItems= lines
    // from a server.ini that may have drifted. Let's support that by checking the
    // built parallel lists against each other. If a mod has workshop_id X but
    // appears at a different position than X in the workshop_ids list, that's a mismatch.
    //
    // For now, since we derive both from load_order, check if any workshop mod
    // appears with mismatched workshop ID (e.g., mod moved but workshop ID didn't).
    // The lists we produce are always correct by construction, so synced=true
    // unless the caller wants to compare against existing server.ini lines.

    let synced = mismatches.is_empty();

    SyncCheckResult {
        synced,
        mod_ids,
        workshop_ids,
        mismatches,
    }
}

/// Check sync between existing Mods= and WorkshopItems= lines.
///
/// Takes the actual mod IDs and workshop IDs from a server.ini (or similar)
/// and validates them against the known mod database.
pub fn check_existing_sync(
    mod_ids: &[String],
    workshop_ids: &[String],
    all_mods: &[ModInfo],
) -> SyncCheckResult {
    let mut mismatches = Vec::new();

    // The lists must be the same length
    let len = mod_ids.len().min(workshop_ids.len());

    for i in 0..len {
        let mod_id = &mod_ids[i];
        let given_workshop_id = &workshop_ids[i];

        // Look up what the workshop ID should be for this mod
        let expected_workshop_id = all_mods
            .iter()
            .find(|m| &m.id == mod_id)
            .and_then(|m| m.workshop_id.clone())
            .unwrap_or_default();

        if *given_workshop_id != expected_workshop_id {
            mismatches.push(SyncMismatch {
                index: i,
                mod_id: mod_id.clone(),
                workshop_id: given_workshop_id.clone(),
            });
        }
    }

    // If lengths differ, the extra entries are all mismatches
    if mod_ids.len() > workshop_ids.len() {
        for i in len..mod_ids.len() {
            mismatches.push(SyncMismatch {
                index: i,
                mod_id: mod_ids[i].clone(),
                workshop_id: String::new(),
            });
        }
    } else if workshop_ids.len() > mod_ids.len() {
        for i in len..workshop_ids.len() {
            mismatches.push(SyncMismatch {
                index: i,
                mod_id: String::new(),
                workshop_id: workshop_ids[i].clone(),
            });
        }
    }

    let synced = mismatches.is_empty();

    SyncCheckResult {
        synced,
        mod_ids: mod_ids.to_vec(),
        workshop_ids: workshop_ids.to_vec(),
        mismatches,
    }
}

/// Fix the sync by producing correctly paired Mods= and WorkshopItems= lists
/// from a load order, using the mod database for workshop ID lookups.
///
/// Returns a SyncCheckResult with the corrected parallel lists and no mismatches.
pub fn fix_mod_workshop_sync(
    load_order: &[String],
    all_mods: &[ModInfo],
) -> SyncCheckResult {
    let mut mod_ids = Vec::with_capacity(load_order.len());
    let mut workshop_ids = Vec::with_capacity(load_order.len());

    for mod_id in load_order {
        let workshop_id = all_mods
            .iter()
            .find(|m| &m.id == mod_id)
            .and_then(|m| m.workshop_id.clone())
            .unwrap_or_default();

        mod_ids.push(mod_id.clone());
        workshop_ids.push(workshop_id);
    }

    SyncCheckResult {
        synced: true,
        mod_ids,
        workshop_ids,
        mismatches: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::app_core::types::ModSource;

    fn make_mod(id: &str, workshop_id: Option<&str>) -> ModInfo {
        ModInfo {
            id: id.to_string(),
            raw_id: id.to_string(),
            workshop_id: workshop_id.map(|s| s.to_string()),
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
            requires: vec![],
            pack: None,
            tile_def: vec![],
            category: None,
            source: ModSource::Workshop,
            source_path: PathBuf::from("/fake"),
            mod_info_path: PathBuf::from("/fake/mod.info"),
            size_bytes: None,
            last_modified: "2024-01-01T00:00:00+00:00".to_string(),
            detected_category: None,
        }
    }

    #[test]
    fn check_sync_produces_parallel_lists() {
        let all_mods = vec![
            make_mod("mod1", Some("111")),
            make_mod("mod2", Some("222")),
            make_mod("mod3", Some("333")),
        ];

        let load_order = vec!["mod1".into(), "mod2".into(), "mod3".into()];
        let result = check_mod_workshop_sync(&load_order, &all_mods);

        assert!(result.synced);
        assert_eq!(result.mod_ids, vec!["mod1", "mod2", "mod3"]);
        assert_eq!(result.workshop_ids, vec!["111", "222", "333"]);
        assert!(result.mismatches.is_empty());
    }

    #[test]
    fn check_sync_with_local_mod() {
        let all_mods = vec![
            make_mod("mod1", Some("111")),
            make_mod("local_mod", None),
            make_mod("mod3", Some("333")),
        ];

        let load_order = vec!["mod1".into(), "local_mod".into(), "mod3".into()];
        let result = check_mod_workshop_sync(&load_order, &all_mods);

        assert!(result.synced);
        assert_eq!(result.workshop_ids, vec!["111", "", "333"]);
    }

    #[test]
    fn check_existing_sync_detects_mismatch() {
        let all_mods = vec![
            make_mod("mod1", Some("111")),
            make_mod("mod2", Some("222")),
        ];

        // Workshop IDs are swapped
        let mod_ids = vec!["mod1".into(), "mod2".into()];
        let workshop_ids = vec!["222".into(), "111".into()];
        let result = check_existing_sync(&mod_ids, &workshop_ids, &all_mods);

        assert!(!result.synced);
        assert_eq!(result.mismatches.len(), 2);
        assert_eq!(result.mismatches[0].index, 0);
        assert_eq!(result.mismatches[0].mod_id, "mod1");
        assert_eq!(result.mismatches[1].index, 1);
        assert_eq!(result.mismatches[1].mod_id, "mod2");
    }

    #[test]
    fn check_existing_sync_length_mismatch() {
        let all_mods = vec![
            make_mod("mod1", Some("111")),
            make_mod("mod2", Some("222")),
        ];

        let mod_ids = vec!["mod1".into(), "mod2".into()];
        let workshop_ids = vec!["111".into()]; // Missing one
        let result = check_existing_sync(&mod_ids, &workshop_ids, &all_mods);

        assert!(!result.synced);
        assert_eq!(result.mismatches.len(), 1);
        assert_eq!(result.mismatches[0].index, 1);
    }

    #[test]
    fn fix_sync_produces_correct_pairs() {
        let all_mods = vec![
            make_mod("mod1", Some("111")),
            make_mod("mod2", Some("222")),
            make_mod("mod3", Some("333")),
        ];

        let load_order = vec!["mod3".into(), "mod1".into(), "mod2".into()];
        let result = fix_mod_workshop_sync(&load_order, &all_mods);

        assert!(result.synced);
        assert_eq!(result.mod_ids, vec!["mod3", "mod1", "mod2"]);
        assert_eq!(result.workshop_ids, vec!["333", "111", "222"]);
        assert!(result.mismatches.is_empty());
    }
}
