use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::app_core::error::AppError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(rename = "type")]
    pub extension_type: String, // "rule-pack" | "theme"
    #[serde(default)]
    pub provides: ExtensionProvides,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionProvides {
    pub community_rules: Option<String>,   // filename
    pub incompatibilities: Option<String>,  // filename
    pub replacements: Option<String>,       // filename
    pub theme: Option<String>,              // filename
    pub migration_versions: Option<String>,      // filename (versions.json)
    pub script_property_rules: Option<String>,   // filename (script-property-rules.json)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub extension_type: String,
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub outdated_mod_id: String,
    pub outdated_mod_name: Option<String>,
    pub replacement_mod_id: String,
    pub replacement_mod_name: Option<String>,
    pub replacement_workshop_id: Option<String>,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the extensions directory inside the app data folder.
pub fn extensions_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("extensions")
}

/// Validate an extension ID to prevent path traversal.
fn validate_extension_id(id: &str) -> Result<(), AppError> {
    if id.is_empty()
        || id.contains("..")
        || id.contains('/')
        || id.contains('\\')
        || id.contains('\0')
    {
        return Err(AppError::Validation(format!(
            "Invalid extension ID: {}",
            id
        )));
    }
    Ok(())
}

/// Parse the manifest from an extension directory.
fn parse_manifest(ext_dir: &Path) -> Result<ExtensionManifest, AppError> {
    let manifest_path = ext_dir.join("extension.json");
    if !manifest_path.exists() {
        return Err(AppError::NotFound(format!(
            "extension.json not found in {}",
            ext_dir.display()
        )));
    }
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: ExtensionManifest = serde_json::from_str(&content)?;
    Ok(manifest)
}

/// Convert a manifest + directory into an ExtensionInfo.
fn manifest_to_info(manifest: &ExtensionManifest, ext_dir: &Path, enabled: bool) -> ExtensionInfo {
    ExtensionInfo {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        author: manifest.author.clone(),
        description: manifest.description.clone(),
        extension_type: manifest.extension_type.clone(),
        enabled,
        path: ext_dir.to_string_lossy().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// List all installed extensions, checking each for a `.disabled` marker.
pub fn list_extensions(app_data_dir: &Path) -> Vec<ExtensionInfo> {
    let dir = extensions_dir(app_data_dir);
    if !dir.exists() {
        return vec![];
    }

    let mut results = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(manifest) = parse_manifest(&path) {
            let disabled_marker = path.join(".disabled");
            let enabled = !disabled_marker.exists();
            results.push(manifest_to_info(&manifest, &path, enabled));
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

/// Install an extension from a source path (zip file or directory).
pub fn install_extension(
    app_data_dir: &Path,
    source_path: &Path,
) -> Result<ExtensionInfo, AppError> {
    let ext_base = extensions_dir(app_data_dir);
    std::fs::create_dir_all(&ext_base)?;

    if source_path.is_dir() {
        install_from_directory(app_data_dir, source_path)
    } else {
        // Detect archive format by magic bytes, not file extension.
        match detect_archive_format(source_path)? {
            ArchiveFormat::Zip => install_from_zip(app_data_dir, source_path),
            ArchiveFormat::Gzip => install_from_tar_gz(app_data_dir, source_path),
            ArchiveFormat::Unknown => Err(AppError::Validation(
                "Unsupported archive format. Expected a .zip or .tar.gz file.".to_string(),
            )),
        }
    }
}

fn install_from_directory(
    app_data_dir: &Path,
    source: &Path,
) -> Result<ExtensionInfo, AppError> {
    let manifest = parse_manifest(source)?;
    validate_extension_id(&manifest.id)?;

    let dest = extensions_dir(app_data_dir).join(&manifest.id);
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    copy_dir_recursive(source, &dest)?;

    Ok(manifest_to_info(&manifest, &dest, true))
}

fn install_from_zip(
    app_data_dir: &Path,
    zip_path: &Path,
) -> Result<ExtensionInfo, AppError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Extract to a temporary directory first so we can validate the manifest
    let tmp = tempfile::tempdir().map_err(|e| AppError::Io(e.to_string()))?;
    archive.extract(tmp.path())?;

    // The zip might contain a single top-level directory or files directly.
    // Check for extension.json at root first, then in a single subdirectory.
    let manifest_direct = tmp.path().join("extension.json");
    let ext_source = if manifest_direct.exists() {
        tmp.path().to_path_buf()
    } else {
        // Look for a single subdirectory containing extension.json
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(tmp.path()) {
            let subdirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .collect();
            if subdirs.len() == 1 {
                let sub = subdirs[0].path();
                if sub.join("extension.json").exists() {
                    found = Some(sub);
                }
            }
        }
        found.ok_or_else(|| {
            AppError::Validation(
                "Zip does not contain extension.json at root or in a single subdirectory"
                    .to_string(),
            )
        })?
    };

    install_from_directory(app_data_dir, &ext_source)
}

enum ArchiveFormat {
    Zip,
    Gzip,
    Unknown,
}

/// Detect archive format by reading magic bytes from the file header.
fn detect_archive_format(path: &Path) -> Result<ArchiveFormat, AppError> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 4];
    use std::io::Read;
    let n = file.read(&mut magic).unwrap_or(0);
    if n < 2 {
        return Ok(ArchiveFormat::Unknown);
    }
    if magic[0] == 0x50 && magic[1] == 0x4B {
        // PK — ZIP archive
        Ok(ArchiveFormat::Zip)
    } else if magic[0] == 0x1F && magic[1] == 0x8B {
        // Gzip (tar.gz)
        Ok(ArchiveFormat::Gzip)
    } else {
        Ok(ArchiveFormat::Unknown)
    }
}

/// Install an extension from a .tar.gz archive.
fn install_from_tar_gz(
    app_data_dir: &Path,
    tar_gz_path: &Path,
) -> Result<ExtensionInfo, AppError> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let file = std::fs::File::open(tar_gz_path)?;
    let gz = GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let tmp = tempfile::tempdir().map_err(|e| AppError::Io(e.to_string()))?;
    archive.unpack(tmp.path()).map_err(|e| AppError::Io(e.to_string()))?;

    // Same logic as zip: check for extension.json at root or in a single subdirectory.
    let manifest_direct = tmp.path().join("extension.json");
    let ext_source = if manifest_direct.exists() {
        tmp.path().to_path_buf()
    } else {
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(tmp.path()) {
            let subdirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .collect();
            if subdirs.len() == 1 {
                let sub = subdirs[0].path();
                if sub.join("extension.json").exists() {
                    found = Some(sub);
                }
            }
        }
        found.ok_or_else(|| {
            AppError::Validation(
                "Archive does not contain extension.json at root or in a single subdirectory"
                    .to_string(),
            )
        })?
    };

    install_from_directory(app_data_dir, &ext_source)
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Toggle an extension's enabled state via a `.disabled` marker file.
pub fn toggle_extension(
    app_data_dir: &Path,
    id: &str,
    enabled: bool,
) -> Result<(), AppError> {
    validate_extension_id(id)?;

    let ext_dir = extensions_dir(app_data_dir).join(id);
    if !ext_dir.exists() {
        return Err(AppError::NotFound(format!("Extension not found: {}", id)));
    }

    let marker = ext_dir.join(".disabled");
    if enabled {
        // Remove the disabled marker if it exists
        if marker.exists() {
            std::fs::remove_file(&marker)?;
        }
    } else {
        // Create the disabled marker
        std::fs::write(&marker, "")?;
    }

    Ok(())
}

/// Uninstall an extension by removing its directory.
pub fn uninstall_extension(app_data_dir: &Path, id: &str) -> Result<(), AppError> {
    validate_extension_id(id)?;

    let ext_dir = extensions_dir(app_data_dir).join(id);
    if !ext_dir.exists() {
        return Err(AppError::NotFound(format!("Extension not found: {}", id)));
    }

    std::fs::remove_dir_all(&ext_dir)?;
    Ok(())
}

/// Load all rules, incompatibilities, and replacements from enabled rule-pack extensions.
pub fn load_extension_rules(
    app_data_dir: &Path,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>, Vec<Replacement>) {
    let mut all_rules = Vec::new();
    let mut all_incompat = Vec::new();
    let mut all_replacements = Vec::new();

    let dir = extensions_dir(app_data_dir);
    if !dir.exists() {
        return (all_rules, all_incompat, all_replacements);
    }

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return (all_rules, all_incompat, all_replacements),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip disabled extensions
        if path.join(".disabled").exists() {
            continue;
        }

        let manifest = match parse_manifest(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Only process rule-pack type extensions
        if manifest.extension_type != "rule-pack" {
            continue;
        }

        // Community rules
        if let Some(ref filename) = manifest.provides.community_rules {
            let file_path = path.join(filename);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(rules) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    all_rules.extend(rules);
                }
            }
        }

        // Incompatibilities
        if let Some(ref filename) = manifest.provides.incompatibilities {
            let file_path = path.join(filename);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(incompat) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    all_incompat.extend(incompat);
                }
            }
        }

        // Replacements
        if let Some(ref filename) = manifest.provides.replacements {
            let file_path = path.join(filename);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(replacements) = serde_json::from_str::<Vec<Replacement>>(&content) {
                    all_replacements.extend(replacements);
                }
            }
        }
    }

    (all_rules, all_incompat, all_replacements)
}

/// Convenience function to get all replacements from enabled extensions.
pub fn get_all_replacements(app_data_dir: &Path) -> Vec<Replacement> {
    let (_, _, replacements) = load_extension_rules(app_data_dir);
    replacements
}

// ---------------------------------------------------------------------------
// Migration rules loading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationVersionInfo {
    pub from: String,
    pub to: String,
    pub rules_file: String,
    pub rule_count: u32,
    pub summary: MigrationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSummary {
    pub classes_added: u32,
    pub classes_removed: u32,
    pub classes_changed: u32,
    pub methods_added: u32,
    pub methods_removed: u32,
    pub events_added: u32,
    pub events_removed: u32,
    pub events_changed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationVersionsData {
    pub versions: Vec<MigrationVersionInfo>,
    pub latest_version: String,
}

/// Load available migration version transitions from installed migration-rules extensions.
pub fn load_migration_versions(app_data_dir: &Path) -> MigrationVersionsData {
    let dir = extensions_dir(app_data_dir);
    let empty = MigrationVersionsData {
        versions: vec![],
        latest_version: String::new(),
    };

    if !dir.exists() {
        return empty;
    }

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return empty,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || path.join(".disabled").exists() {
            continue;
        }

        let manifest = match parse_manifest(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if manifest.extension_type != "migration-rules" {
            continue;
        }

        if let Some(ref versions_file) = manifest.provides.migration_versions {
            let file_path = path.join(versions_file);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(data) = serde_json::from_str::<MigrationVersionsData>(&content) {
                    return data;
                }
            }
        }
    }

    empty
}

/// Load deprecation rules for a specific version transition from the migration-rules extension.
pub fn load_migration_rules(
    app_data_dir: &Path,
    from_version: &str,
    to_version: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let dir = extensions_dir(app_data_dir);
    if !dir.exists() {
        return Err(AppError::NotFound("No extensions installed".into()));
    }

    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if !path.is_dir() || path.join(".disabled").exists() {
            continue;
        }

        let manifest = match parse_manifest(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if manifest.extension_type != "migration-rules" {
            continue;
        }

        if let Some(ref versions_file) = manifest.provides.migration_versions {
            let vf_path = path.join(versions_file);
            if let Ok(content) = std::fs::read_to_string(&vf_path) {
                if let Ok(data) = serde_json::from_str::<MigrationVersionsData>(&content) {
                    // Find the matching version transition
                    for v in &data.versions {
                        if v.from == from_version && v.to == to_version {
                            let rules_path = path.join(&v.rules_file);
                            let rules_content = std::fs::read_to_string(&rules_path)?;
                            let rules: Vec<serde_json::Value> =
                                serde_json::from_str(&rules_content)?;
                            return Ok(rules);
                        }
                    }
                }
            }
        }
    }

    Err(AppError::NotFound(format!(
        "No migration rules found for {} → {}",
        from_version, to_version
    )))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_extension(base_dir: &Path, id: &str) -> PathBuf {
        let ext_dir = base_dir.join("extensions").join(id);
        std::fs::create_dir_all(&ext_dir).unwrap();

        let manifest = serde_json::json!({
            "id": id,
            "name": format!("Test Extension {}", id),
            "version": "1.0.0",
            "author": "TestAuthor",
            "description": "A test extension",
            "type": "rule-pack",
            "provides": {
                "communityRules": "rules.json",
                "incompatibilities": "incompat.json",
                "replacements": "replacements.json"
            }
        });

        std::fs::write(
            ext_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // Write sample rules
        let rules = serde_json::json!([
            { "modId": "ModA", "loadBefore": ["ModB"], "loadAfter": [], "notes": "test rule" }
        ]);
        std::fs::write(
            ext_dir.join("rules.json"),
            serde_json::to_string(&rules).unwrap(),
        )
        .unwrap();

        // Write sample incompatibilities
        let incompat = serde_json::json!([
            { "mod_a": "ModX", "mod_b": "ModY", "reason": "conflict", "severity": "error" }
        ]);
        std::fs::write(
            ext_dir.join("incompat.json"),
            serde_json::to_string(&incompat).unwrap(),
        )
        .unwrap();

        // Write sample replacements
        let replacements = serde_json::json!([
            {
                "outdatedModId": "OldMod",
                "outdatedModName": "Old Mod Name",
                "replacementModId": "NewMod",
                "replacementModName": "New Mod Name",
                "replacementWorkshopId": "99999",
                "reason": "OldMod is abandoned"
            }
        ]);
        std::fs::write(
            ext_dir.join("replacements.json"),
            serde_json::to_string(&replacements).unwrap(),
        )
        .unwrap();

        ext_dir
    }

    #[test]
    fn test_extensions_dir() {
        let dir = extensions_dir(Path::new("/app/data"));
        assert_eq!(dir, PathBuf::from("/app/data/extensions"));
    }

    #[test]
    fn test_validate_extension_id_accepts_valid() {
        assert!(validate_extension_id("my-extension").is_ok());
        assert!(validate_extension_id("ext_v2").is_ok());
        assert!(validate_extension_id("community-rules-b42").is_ok());
    }

    #[test]
    fn test_validate_extension_id_rejects_traversal() {
        assert!(validate_extension_id("..").is_err());
        assert!(validate_extension_id("../evil").is_err());
        assert!(validate_extension_id("foo/bar").is_err());
        assert!(validate_extension_id("foo\\bar").is_err());
        assert!(validate_extension_id("").is_err());
        assert!(validate_extension_id("foo\0bar").is_err());
    }

    #[test]
    fn test_manifest_parsing() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = tmp.path().join("test-ext");
        std::fs::create_dir_all(&ext_dir).unwrap();

        let manifest_json = serde_json::json!({
            "id": "test-ext",
            "name": "Test Extension",
            "version": "1.0.0",
            "author": "Author",
            "description": "Desc",
            "type": "rule-pack",
            "provides": {
                "communityRules": "rules.json"
            }
        });
        std::fs::write(
            ext_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest_json).unwrap(),
        )
        .unwrap();

        let manifest = parse_manifest(&ext_dir).unwrap();
        assert_eq!(manifest.id, "test-ext");
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.extension_type, "rule-pack");
        assert_eq!(
            manifest.provides.community_rules,
            Some("rules.json".to_string())
        );
        assert!(manifest.provides.incompatibilities.is_none());
    }

    #[test]
    fn test_manifest_parsing_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = parse_manifest(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_extensions_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let extensions = list_extensions(tmp.path());
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_list_extensions_finds_installed() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "ext-one");
        create_test_extension(tmp.path(), "ext-two");

        let extensions = list_extensions(tmp.path());
        assert_eq!(extensions.len(), 2);
        assert!(extensions.iter().any(|e| e.id == "ext-one"));
        assert!(extensions.iter().any(|e| e.id == "ext-two"));
        assert!(extensions.iter().all(|e| e.enabled));
    }

    #[test]
    fn test_toggle_extension() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "toggle-test");

        // Initially enabled (no .disabled marker)
        let extensions = list_extensions(tmp.path());
        assert!(extensions[0].enabled);

        // Disable
        toggle_extension(tmp.path(), "toggle-test", false).unwrap();
        let extensions = list_extensions(tmp.path());
        assert!(!extensions[0].enabled);

        // Re-enable
        toggle_extension(tmp.path(), "toggle-test", true).unwrap();
        let extensions = list_extensions(tmp.path());
        assert!(extensions[0].enabled);
    }

    #[test]
    fn test_toggle_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("extensions")).unwrap();
        let result = toggle_extension(tmp.path(), "nonexistent", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_uninstall_extension() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "uninstall-me");

        assert_eq!(list_extensions(tmp.path()).len(), 1);

        uninstall_extension(tmp.path(), "uninstall-me").unwrap();
        assert_eq!(list_extensions(tmp.path()).len(), 0);
    }

    #[test]
    fn test_uninstall_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("extensions")).unwrap();
        let result = uninstall_extension(tmp.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_uninstall_rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let result = uninstall_extension(tmp.path(), "../etc");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid extension ID"),
            "Expected validation error, got: {}",
            err
        );
    }

    #[test]
    fn test_install_from_directory() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a source extension directory
        let source_dir = tmp.path().join("source-ext");
        std::fs::create_dir_all(&source_dir).unwrap();
        let manifest = serde_json::json!({
            "id": "installed-ext",
            "name": "Installed Extension",
            "version": "2.0.0",
            "author": "Author",
            "description": "An installed extension",
            "type": "rule-pack"
        });
        std::fs::write(
            source_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let app_data = tmp.path().join("app_data");
        std::fs::create_dir_all(&app_data).unwrap();

        let info = install_extension(&app_data, &source_dir).unwrap();
        assert_eq!(info.id, "installed-ext");
        assert_eq!(info.version, "2.0.0");
        assert!(info.enabled);

        // Verify it shows up in the list
        let extensions = list_extensions(&app_data);
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0].id, "installed-ext");
    }

    #[test]
    fn test_install_from_zip() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a zip containing an extension
        let zip_path = tmp.path().join("test-ext.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);

        let manifest = serde_json::json!({
            "id": "zip-ext",
            "name": "Zip Extension",
            "version": "1.0.0",
            "author": "ZipAuthor",
            "description": "From zip",
            "type": "theme"
        });

        let options = zip::write::SimpleFileOptions::default();
        zip_writer
            .start_file("extension.json", options)
            .unwrap();
        use std::io::Write;
        zip_writer
            .write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();
        zip_writer.finish().unwrap();

        let app_data = tmp.path().join("app_data");
        std::fs::create_dir_all(&app_data).unwrap();

        let info = install_extension(&app_data, &zip_path).unwrap();
        assert_eq!(info.id, "zip-ext");
        assert_eq!(info.extension_type, "theme");
        assert!(info.enabled);
    }

    #[test]
    fn test_install_from_tar_gz() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a tar.gz containing an extension in a subdirectory
        let tar_gz_path = tmp.path().join("test-ext.tar.gz");
        let manifest = serde_json::json!({
            "id": "tar-gz-ext",
            "name": "Tar GZ Extension",
            "version": "2.0.0",
            "author": "TarAuthor",
            "description": "From tar.gz",
            "type": "rule-pack"
        });

        // Build tar.gz in memory
        let file = std::fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(gz);

        let manifest_bytes = serde_json::to_string_pretty(&manifest).unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append_data(
            &mut header,
            "tar-gz-ext/extension.json",
            manifest_bytes.as_bytes(),
        ).unwrap();
        tar_builder.into_inner().unwrap().finish().unwrap();

        let app_data = tmp.path().join("app_data");
        std::fs::create_dir_all(&app_data).unwrap();

        let info = install_extension(&app_data, &tar_gz_path).unwrap();
        assert_eq!(info.id, "tar-gz-ext");
        assert_eq!(info.extension_type, "rule-pack");
        assert!(info.enabled);

        // Verify the extension was installed
        let ext_dir = app_data.join("extensions").join("tar-gz-ext");
        assert!(ext_dir.join("extension.json").exists());
    }

    #[test]
    fn test_detect_archive_format() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a fake zip (PK magic bytes)
        let zip_path = tmp.path().join("test.zip");
        std::fs::write(&zip_path, b"PK\x03\x04fake zip content").unwrap();
        assert!(matches!(detect_archive_format(&zip_path).unwrap(), ArchiveFormat::Zip));

        // Create a fake gzip (1F 8B magic bytes)
        let gz_path = tmp.path().join("test.tar.gz");
        std::fs::write(&gz_path, b"\x1f\x8bfake gzip content").unwrap();
        assert!(matches!(detect_archive_format(&gz_path).unwrap(), ArchiveFormat::Gzip));

        // Create a file with unknown magic bytes
        let txt_path = tmp.path().join("test.txt");
        std::fs::write(&txt_path, b"hello world").unwrap();
        assert!(matches!(detect_archive_format(&txt_path).unwrap(), ArchiveFormat::Unknown));

        // Extension doesn't matter — a .zip file with gzip content is detected as gzip
        let misnamed = tmp.path().join("actually-gzip.zip");
        std::fs::write(&misnamed, b"\x1f\x8bfake gzip").unwrap();
        assert!(matches!(detect_archive_format(&misnamed).unwrap(), ArchiveFormat::Gzip));
    }

    #[test]
    fn test_load_extension_rules() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "rules-ext");

        let (rules, incompat, replacements) = load_extension_rules(tmp.path());

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["modId"], "ModA");

        assert_eq!(incompat.len(), 1);
        assert_eq!(incompat[0]["mod_a"], "ModX");

        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].outdated_mod_id, "OldMod");
        assert_eq!(replacements[0].replacement_mod_id, "NewMod");
        assert_eq!(replacements[0].reason, "OldMod is abandoned");
    }

    #[test]
    fn test_load_extension_rules_skips_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "disabled-ext");

        // Disable it
        toggle_extension(tmp.path(), "disabled-ext", false).unwrap();

        let (rules, incompat, replacements) = load_extension_rules(tmp.path());
        assert!(rules.is_empty());
        assert!(incompat.is_empty());
        assert!(replacements.is_empty());
    }

    #[test]
    fn test_load_extension_rules_skips_themes() {
        let tmp = tempfile::tempdir().unwrap();
        let ext_dir = tmp.path().join("extensions").join("theme-ext");
        std::fs::create_dir_all(&ext_dir).unwrap();

        let manifest = serde_json::json!({
            "id": "theme-ext",
            "name": "Theme Extension",
            "version": "1.0.0",
            "author": "Author",
            "description": "A theme",
            "type": "theme",
            "provides": {
                "theme": "theme.css"
            }
        });
        std::fs::write(
            ext_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let (rules, incompat, replacements) = load_extension_rules(tmp.path());
        assert!(rules.is_empty());
        assert!(incompat.is_empty());
        assert!(replacements.is_empty());
    }

    #[test]
    fn test_get_all_replacements() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "repl-ext");

        let replacements = get_all_replacements(tmp.path());
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].outdated_mod_id, "OldMod");
    }

    #[test]
    fn test_replacement_round_trip() {
        let replacement = Replacement {
            outdated_mod_id: "OldMod".to_string(),
            outdated_mod_name: Some("Old Mod".to_string()),
            replacement_mod_id: "NewMod".to_string(),
            replacement_mod_name: Some("New Mod".to_string()),
            replacement_workshop_id: Some("12345".to_string()),
            reason: "Abandoned".to_string(),
        };
        let json = serde_json::to_string(&replacement).unwrap();
        assert!(json.contains("\"outdatedModId\""));
        assert!(json.contains("\"replacementModId\""));
        assert!(json.contains("\"replacementWorkshopId\""));

        let restored: Replacement = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.outdated_mod_id, "OldMod");
        assert_eq!(restored.replacement_mod_id, "NewMod");
    }

    #[test]
    fn test_list_extensions_sorted_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        create_test_extension(tmp.path(), "zzz-ext");
        create_test_extension(tmp.path(), "aaa-ext");

        let extensions = list_extensions(tmp.path());
        assert_eq!(extensions.len(), 2);
        // The create_test_extension names format as "Test Extension {id}"
        assert!(extensions[0].name < extensions[1].name);
    }

    #[test]
    fn test_install_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let app_data = tmp.path().join("app_data");
        std::fs::create_dir_all(&app_data).unwrap();

        // Create a source extension
        let source_dir = tmp.path().join("source-ext");
        std::fs::create_dir_all(&source_dir).unwrap();
        let manifest = serde_json::json!({
            "id": "overwrite-ext",
            "name": "Version 1",
            "version": "1.0.0",
            "author": "Author",
            "description": "First version",
            "type": "rule-pack"
        });
        std::fs::write(
            source_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        install_extension(&app_data, &source_dir).unwrap();

        // Update the source and reinstall
        let manifest_v2 = serde_json::json!({
            "id": "overwrite-ext",
            "name": "Version 2",
            "version": "2.0.0",
            "author": "Author",
            "description": "Second version",
            "type": "rule-pack"
        });
        std::fs::write(
            source_dir.join("extension.json"),
            serde_json::to_string_pretty(&manifest_v2).unwrap(),
        )
        .unwrap();

        let info = install_extension(&app_data, &source_dir).unwrap();
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.name, "Version 2");

        // Only one extension should exist
        assert_eq!(list_extensions(&app_data).len(), 1);
    }
}
