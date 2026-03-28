use std::path::Path;
use crate::app_core::types::{Profile, ProfileType};
use crate::app_core::error::AppError;

pub fn ensure_profiles_dir(app_data_dir: &Path) -> Result<std::path::PathBuf, AppError> {
    let dir = app_data_dir.join("profiles");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn list_profiles(profiles_dir: &Path) -> Result<Vec<Profile>, AppError> {
    let mut profiles = Vec::new();

    let entries = std::fs::read_dir(profiles_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path)?;
            match serde_json::from_str::<Profile>(&content) {
                Ok(profile) => profiles.push(profile),
                Err(e) => {
                    log::warn!("Failed to parse profile at {:?}: {}", path, e);
                }
            }
        }
    }

    profiles.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(profiles)
}

fn validate_profile_id(id: &str) -> Result<(), AppError> {
    if id.is_empty() || id.contains("..") || id.contains('/') || id.contains('\\')
        || id.contains('\0') {
        return Err(AppError::Validation(format!("Invalid profile ID: {}", id)));
    }
    Ok(())
}

pub fn get_profile(profiles_dir: &Path, id: &str) -> Result<Profile, AppError> {
    validate_profile_id(id)?;
    let path = profiles_dir.join(format!("{}.json", id));
    if !path.exists() {
        return Err(AppError::NotFound(format!("Profile not found: {}", id)));
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_profile(profiles_dir: &Path, profile: &Profile) -> Result<(), AppError> {
    let path = profiles_dir.join(format!("{}.json", profile.id));
    let content = serde_json::to_string_pretty(profile)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn delete_profile(profiles_dir: &Path, id: &str) -> Result<(), AppError> {
    validate_profile_id(id)?;
    let profile = get_profile(profiles_dir, id)?;
    if profile.is_default {
        return Err(AppError::Validation("Cannot delete the default profile".into()));
    }
    let path = profiles_dir.join(format!("{}.json", id));
    std::fs::remove_file(&path)?;
    Ok(())
}

pub fn create_profile(profiles_dir: &Path, name: &str, profile_type: ProfileType) -> Result<Profile, AppError> {
    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        profile_type,
        load_order: vec![],
        server_config_path: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        is_default: false,
        version_overrides: std::collections::HashMap::new(),
        game_path: None,
        steam_path: None,
        workshop_path: None,
        local_mods_path: None,
        zomboid_user_dir: None,
        game_version: None,
    };
    save_profile(profiles_dir, &profile)?;
    Ok(profile)
}

pub fn duplicate_profile(profiles_dir: &Path, source_id: &str, new_name: &str) -> Result<Profile, AppError> {
    let source = get_profile(profiles_dir, source_id)?;
    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: new_name.into(),
        profile_type: source.profile_type,
        load_order: source.load_order,
        server_config_path: source.server_config_path,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        is_default: false,
        version_overrides: source.version_overrides,
        game_path: source.game_path,
        steam_path: source.steam_path,
        workshop_path: source.workshop_path,
        local_mods_path: source.local_mods_path,
        zomboid_user_dir: source.zomboid_user_dir,
        game_version: source.game_version,
    };
    save_profile(profiles_dir, &profile)?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a default-like profile for tests.
    fn create_test_default_profile(profiles_dir: &Path) -> Profile {
        let profile = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Default".into(),
            profile_type: ProfileType::Singleplayer,
            load_order: vec![],
            server_config_path: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            is_default: true,
            version_overrides: std::collections::HashMap::new(),
            game_path: None,
            steam_path: None,
            workshop_path: None,
            local_mods_path: None,
            zomboid_user_dir: None,
            game_version: None,
        };
        save_profile(profiles_dir, &profile).unwrap();
        profile
    }

    #[test]
    fn test_create_and_list_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let p = create_test_default_profile(dir.path());
        assert!(p.is_default);
        assert_eq!(p.name, "Default");
        let all = list_profiles(dir.path()).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_cannot_delete_default_profile() {
        let dir = tempfile::tempdir().unwrap();
        let p = create_test_default_profile(dir.path());
        let result = delete_profile(dir.path(), &p.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_json_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = create_test_default_profile(dir.path());
        p.load_order = vec!["ModA".into(), "ModB".into()];
        save_profile(dir.path(), &p).unwrap();
        let loaded = get_profile(dir.path(), &p.id).unwrap();
        assert_eq!(loaded.load_order, vec!["ModA", "ModB"]);
    }

    #[test]
    fn test_duplicate_profile() {
        let dir = tempfile::tempdir().unwrap();
        let p = create_test_default_profile(dir.path());
        let dup = duplicate_profile(dir.path(), &p.id, "Duplicate").unwrap();
        assert_ne!(dup.id, p.id);
        assert_eq!(dup.name, "Duplicate");
        assert!(!dup.is_default);
    }

    #[test]
    fn test_create_custom_profile() {
        let dir = tempfile::tempdir().unwrap();
        let p = create_profile(dir.path(), "My Modset", ProfileType::Singleplayer).unwrap();
        assert_eq!(p.name, "My Modset");
        assert!(!p.is_default);
    }

    #[test]
    fn test_rejects_path_traversal_id() {
        let dir = tempfile::tempdir().unwrap();
        let result = get_profile(dir.path(), "../../../etc/passwd");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid profile ID"), "Expected validation error, got: {}", err);

        let result2 = get_profile(dir.path(), "foo/bar");
        assert!(result2.is_err());
    }

    #[test]
    fn test_export_import_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = create_profile(dir.path(), "Export Test", ProfileType::Singleplayer).unwrap();
        p.load_order = vec!["Mod1".into(), "Mod2".into()];
        save_profile(dir.path(), &p).unwrap();
        let json = serde_json::to_string_pretty(&p).unwrap();
        // Simulate import
        let mut imported: Profile = serde_json::from_str(&json).unwrap();
        imported.id = uuid::Uuid::new_v4().to_string();
        imported.is_default = false;
        save_profile(dir.path(), &imported).unwrap();
        assert_ne!(imported.id, p.id);
        assert_eq!(imported.load_order, vec!["Mod1", "Mod2"]);
    }
}
