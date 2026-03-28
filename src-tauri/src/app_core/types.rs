use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub id: String,
    pub raw_id: String,
    pub workshop_id: Option<String>,
    pub name: String,
    pub description: String,
    pub authors: Vec<String>,
    pub url: Option<String>,
    pub mod_version: Option<String>,
    pub poster_path: Option<String>,
    pub icon_path: Option<String>,
    pub version_min: Option<String>,
    pub version_max: Option<String>,
    pub version_folders: Vec<String>,
    pub active_version_folder: Option<String>,
    pub requires: Vec<String>,
    pub pack: Option<String>,
    pub tile_def: Vec<String>,
    pub category: Option<String>,
    pub source: ModSource,
    pub source_path: PathBuf,
    pub mod_info_path: PathBuf,
    pub size_bytes: Option<u64>,
    pub last_modified: String,
    pub detected_category: Option<ModCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Workshop,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModCategory {
    Framework,
    Map,
    Content,
    Overhaul,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: ProfileType,
    pub load_order: Vec<String>,
    pub server_config_path: Option<PathBuf>,
    pub created_at: String,
    pub updated_at: String,
    pub is_default: bool,
    /// Per-mod version folder overrides (mod_id → version folder name).
    /// When set, the scanner uses this folder instead of auto-resolving.
    #[serde(default)]
    pub version_overrides: std::collections::HashMap<String, String>,
    /// Per-profile path overrides. When set, these take precedence over global config.
    #[serde(default)]
    pub game_path: Option<PathBuf>,
    #[serde(default)]
    pub steam_path: Option<PathBuf>,
    #[serde(default)]
    pub workshop_path: Option<PathBuf>,
    #[serde(default)]
    pub local_mods_path: Option<PathBuf>,
    #[serde(default)]
    pub zomboid_user_dir: Option<PathBuf>,
    #[serde(default)]
    pub game_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    Singleplayer,
    Server,
}

fn default_ui_scale() -> u32 {
    100
}

fn default_font_size() -> u32 {
    14
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub game_path: Option<PathBuf>,
    pub steam_path: Option<PathBuf>,
    pub workshop_path: Option<PathBuf>,
    pub local_mods_path: Option<PathBuf>,
    pub zomboid_user_dir: Option<PathBuf>,
    pub game_version: Option<String>,
    pub is_first_run: bool,
    pub theme: String,
    pub locale: String,
    pub check_updates: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: u32,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderIssue {
    pub severity: IssueSeverity,
    pub mod_id: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub related_mod_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConflictType {
    FileOverride,
    ScriptIdClash,
    VersionMismatch,
    KnownIncompat,
    FunctionOverride,
    EventCollision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModConflict {
    pub conflict_type: ConflictType,
    pub severity: IssueSeverity,
    pub mod_ids: Vec<String>,
    pub file_path: Option<String>,
    pub script_id: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
    pub is_intentional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepResolution {
    pub to_enable: Vec<String>,
    pub not_installed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompatEntry {
    pub mod_a: String,
    pub mod_b: String,
    pub reason: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompatDb {
    pub version: u32,
    pub incompatibilities: Vec<IncompatEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_info_serializes_to_camel_case() {
        let mod_info = ModInfo {
            id: "test-mod".to_string(),
            raw_id: "TestMod".to_string(),
            workshop_id: Some("12345".to_string()),
            name: "Test Mod".to_string(),
            description: "A test mod".to_string(),
            authors: vec!["Author1".to_string()],
            url: None,
            mod_version: Some("1.0".to_string()),
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
            source_path: PathBuf::from("/mods/test"),
            mod_info_path: PathBuf::from("/mods/test/mod.info"),
            size_bytes: Some(1024),
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            detected_category: Some(ModCategory::Content),
        };

        let json = serde_json::to_string(&mod_info).expect("serialization failed");
        assert!(json.contains("\"rawId\""), "expected camelCase 'rawId', got: {}", json);
        assert!(json.contains("\"workshopId\""), "expected camelCase 'workshopId'");
        assert!(json.contains("\"modVersion\""), "expected camelCase 'modVersion'");
        assert!(json.contains("\"sourcePath\""), "expected camelCase 'sourcePath'");
        assert!(json.contains("\"lastModified\""), "expected camelCase 'lastModified'");
        assert!(json.contains("\"detectedCategory\""), "expected camelCase 'detectedCategory'");
    }

    #[test]
    fn profile_type_serializes_as_string() {
        let profile_type = ProfileType::Singleplayer;
        let json = serde_json::to_string(&profile_type).expect("serialization failed");
        assert_eq!(json, "\"singleplayer\"");

        let profile_type = ProfileType::Server;
        let json = serde_json::to_string(&profile_type).expect("serialization failed");
        assert_eq!(json, "\"server\"");
    }

    #[test]
    fn mod_source_serializes_as_string() {
        let source = ModSource::Workshop;
        let json = serde_json::to_string(&source).expect("serialization failed");
        assert_eq!(json, "\"workshop\"");

        let source = ModSource::Local;
        let json = serde_json::to_string(&source).expect("serialization failed");
        assert_eq!(json, "\"local\"");
    }

    #[test]
    fn app_config_round_trip() {
        let config = AppConfig {
            game_path: Some(PathBuf::from("/usr/games/zomboid")),
            steam_path: Some(PathBuf::from("/home/user/.steam")),
            workshop_path: None,
            local_mods_path: Some(PathBuf::from("/home/user/mods")),
            zomboid_user_dir: Some(PathBuf::from("/home/user/.local/share/Zomboid")),
            game_version: Some("41.78".to_string()),
            is_first_run: false,
            theme: "dark".to_string(),
            locale: "en".to_string(),
            check_updates: true,
            ui_scale: 100,
            font_size: 14,
        };

        let json = serde_json::to_string(&config).expect("serialization failed");
        let restored: AppConfig = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(restored.game_path, config.game_path);
        assert_eq!(restored.steam_path, config.steam_path);
        assert_eq!(restored.workshop_path, config.workshop_path);
        assert_eq!(restored.local_mods_path, config.local_mods_path);
        assert_eq!(restored.zomboid_user_dir, config.zomboid_user_dir);
        assert_eq!(restored.game_version, config.game_version);
        assert_eq!(restored.is_first_run, config.is_first_run);
        assert_eq!(restored.theme, config.theme);
        assert_eq!(restored.locale, config.locale);
        assert_eq!(restored.check_updates, config.check_updates);
        assert_eq!(restored.ui_scale, 100);
        assert_eq!(restored.font_size, 14);
    }

    #[test]
    fn load_order_issue_serializes() {
        let issue = LoadOrderIssue {
            severity: IssueSeverity::Warning,
            mod_id: "mod-a".to_string(),
            message: "Mod A should load before Mod B".to_string(),
            suggestion: Some("Move Mod A above Mod B in the load order".to_string()),
            related_mod_id: Some("mod-b".to_string()),
        };

        let json = serde_json::to_string(&issue).expect("serialization failed");
        assert!(json.contains("\"severity\""), "missing 'severity' field");
        assert!(json.contains("\"modId\""), "missing camelCase 'modId' field");
        assert!(json.contains("\"message\""), "missing 'message' field");
        assert!(json.contains("\"suggestion\""), "missing 'suggestion' field");
        assert!(json.contains("\"relatedModId\""), "missing camelCase 'relatedModId' field");
        assert!(json.contains("\"warning\""), "severity should serialize as 'warning'");
    }

    #[test]
    fn conflict_type_serializes_camel_case() {
        let ct = ConflictType::FileOverride;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"fileOverride\"");
        let ct2 = ConflictType::ScriptIdClash;
        let json2 = serde_json::to_string(&ct2).unwrap();
        assert_eq!(json2, "\"scriptIdClash\"");
    }

    #[test]
    fn mod_conflict_serializes() {
        let conflict = ModConflict {
            conflict_type: ConflictType::FileOverride,
            severity: IssueSeverity::Warning,
            mod_ids: vec!["modA".into(), "modB".into()],
            file_path: Some("lua/client/Foo.lua".into()),
            script_id: None,
            message: "File conflict".into(),
            suggestion: None,
            is_intentional: false,
        };
        let json = serde_json::to_string(&conflict).unwrap();
        assert!(json.contains("\"conflictType\""));
        assert!(json.contains("\"fileOverride\""));
        assert!(json.contains("\"modIds\""));
        assert!(json.contains("\"isIntentional\""));
    }

    #[test]
    fn dep_resolution_serializes() {
        let dr = DepResolution {
            to_enable: vec!["ModA".into()],
            not_installed: vec!["ModX".into()],
        };
        let json = serde_json::to_string(&dr).unwrap();
        assert!(json.contains("\"toEnable\""));
        assert!(json.contains("\"notInstalled\""));
    }

    #[test]
    fn incompat_db_round_trip() {
        let db = IncompatDb {
            version: 1,
            incompatibilities: vec![IncompatEntry {
                mod_a: "A".into(),
                mod_b: "B".into(),
                reason: "conflict".into(),
                severity: "error".into(),
            }],
        };
        let json = serde_json::to_string(&db).unwrap();
        let restored: IncompatDb = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.incompatibilities.len(), 1);
    }
}
