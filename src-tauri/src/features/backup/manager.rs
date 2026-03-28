use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use crate::app_core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub path: String,
    pub profile_count: usize,
    pub has_server_configs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    version: u32,
    created_at: String,
    profiles: Vec<String>,
    server_configs: Vec<String>,
}

fn backups_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("backups")
}

/// Create a backup zip containing profiles and optionally server configs.
pub fn create_backup(
    app_data_dir: &Path,
    zomboid_user_dir: Option<&Path>,
    name: &str,
) -> Result<BackupInfo, AppError> {
    let backup_dir = backups_dir(app_data_dir);
    std::fs::create_dir_all(&backup_dir)?;

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();
    let filename = format!("{}_{}.zip",
        name.replace(' ', "_").replace(|c: char| !c.is_alphanumeric() && c != '_', ""),
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    let zip_path = backup_dir.join(&filename);

    let file = std::fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut manifest = BackupManifest {
        version: 1,
        created_at: timestamp.clone(),
        profiles: vec![],
        server_configs: vec![],
    };

    // Add profiles
    let profiles_dir = app_data_dir.join("profiles");
    if profiles_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let filename = match path.file_name() {
                        Some(name) => name.to_string_lossy().into_owned(),
                        None => continue,
                    };
                    let mut content = Vec::new();
                    std::fs::File::open(&path)?.read_to_end(&mut content)?;
                    zip.start_file(format!("profiles/{}", filename), options)?;
                    zip.write_all(&content)?;
                    manifest.profiles.push(filename);
                }
            }
        }
    }

    // Add server configs
    if let Some(zomboid_dir) = zomboid_user_dir {
        let server_dir = zomboid_dir.join("Server");
        if server_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&server_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "ini" || path.to_string_lossy().contains("SandboxVars.lua") {
                        let filename = match path.file_name() {
                            Some(name) => name.to_string_lossy().into_owned(),
                            None => continue,
                        };
                        let mut content = Vec::new();
                        std::fs::File::open(&path)?.read_to_end(&mut content)?;
                        zip.start_file(format!("server/{}", filename), options)?;
                        zip.write_all(&content)?;
                        manifest.server_configs.push(filename);
                    }
                }
            }
        }
    }

    // Add manifest
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    zip.start_file("manifest.json", options)?;
    zip.write_all(manifest_json.as_bytes())?;

    zip.finish()?;

    let metadata = std::fs::metadata(&zip_path)?;

    Ok(BackupInfo {
        id,
        name: name.to_string(),
        created_at: timestamp,
        size_bytes: metadata.len(),
        path: zip_path.to_string_lossy().into_owned(),
        profile_count: manifest.profiles.len(),
        has_server_configs: !manifest.server_configs.is_empty(),
    })
}

/// List all backups in the backups directory.
pub fn list_backups(app_data_dir: &Path) -> Vec<BackupInfo> {
    let backup_dir = backups_dir(app_data_dir);
    if !backup_dir.exists() {
        return vec![];
    }

    let mut backups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "zip").unwrap_or(false) {
                if let Ok(info) = read_backup_info(&path) {
                    backups.push(info);
                }
            }
        }
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    backups
}

/// Read backup info from a zip file by reading its manifest.
fn read_backup_info(zip_path: &Path) -> Result<BackupInfo, AppError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Io(format!("Invalid backup: {}", e)))?;

    let manifest: BackupManifest = {
        let mut manifest_file = archive.by_name("manifest.json")
            .map_err(|_| AppError::Io("Backup missing manifest.json".into()))?;
        let mut content = String::new();
        manifest_file.read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };

    let metadata = std::fs::metadata(zip_path)?;
    let name = zip_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(BackupInfo {
        id: name.clone(),
        name,
        created_at: manifest.created_at,
        size_bytes: metadata.len(),
        path: zip_path.to_string_lossy().into_owned(),
        profile_count: manifest.profiles.len(),
        has_server_configs: !manifest.server_configs.is_empty(),
    })
}

/// Restore profiles from a backup zip.
pub fn restore_backup(
    zip_path: &Path,
    app_data_dir: &Path,
    zomboid_user_dir: Option<&Path>,
) -> Result<(), AppError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Io(format!("Invalid backup: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| AppError::Io(format!("Failed to read backup entry: {}", e)))?;

        let name = file.name().to_string();

        if name == "manifest.json" {
            continue;
        }

        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        if let Some(profile_name) = name.strip_prefix("profiles/") {
            // Zip-slip protection: reject paths with traversal components
            if profile_name.contains("..") || profile_name.contains('/') || profile_name.contains('\\') {
                return Err(AppError::Validation(format!(
                    "Malicious zip entry rejected: {}", name
                )));
            }
            let profiles_dir = app_data_dir.join("profiles");
            std::fs::create_dir_all(&profiles_dir)?;
            std::fs::write(profiles_dir.join(profile_name), &content)?;
        } else if let Some(server_name) = name.strip_prefix("server/") {
            // Zip-slip protection
            if server_name.contains("..") || server_name.contains('/') || server_name.contains('\\') {
                return Err(AppError::Validation(format!(
                    "Malicious zip entry rejected: {}", name
                )));
            }
            if let Some(zomboid_dir) = zomboid_user_dir {
                let server_dir = zomboid_dir.join("Server");
                std::fs::create_dir_all(&server_dir)?;
                std::fs::write(server_dir.join(server_name), &content)?;
            }
        }
    }

    Ok(())
}

/// Delete a backup zip file.
pub fn delete_backup(zip_path: &Path) -> Result<(), AppError> {
    std::fs::remove_file(zip_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list_backup() {
        let app_dir = tempfile::tempdir().unwrap();
        // Create a test profile
        let profiles_dir = app_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        std::fs::write(profiles_dir.join("test.json"), r#"{"id":"test","name":"Test"}"#).unwrap();

        let info = create_backup(app_dir.path(), None, "Test Backup").unwrap();
        assert_eq!(info.name, "Test Backup");
        assert_eq!(info.profile_count, 1);
        assert!(!info.has_server_configs);

        let backups = list_backups(app_dir.path());
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn test_restore_backup() {
        let app_dir = tempfile::tempdir().unwrap();
        let restore_dir = tempfile::tempdir().unwrap();

        // Create profiles and backup
        let profiles_dir = app_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        std::fs::write(profiles_dir.join("p1.json"), r#"{"id":"p1"}"#).unwrap();

        let info = create_backup(app_dir.path(), None, "Restore Test").unwrap();

        // Restore to different location
        restore_backup(
            Path::new(&info.path),
            restore_dir.path(),
            None,
        ).unwrap();

        let restored = restore_dir.path().join("profiles/p1.json");
        assert!(restored.exists());
        let content = std::fs::read_to_string(restored).unwrap();
        assert!(content.contains("p1"));
    }

    #[test]
    fn test_delete_backup() {
        let app_dir = tempfile::tempdir().unwrap();
        let profiles_dir = app_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        std::fs::write(profiles_dir.join("x.json"), "{}").unwrap();

        let info = create_backup(app_dir.path(), None, "Delete Test").unwrap();
        assert!(Path::new(&info.path).exists());
        delete_backup(Path::new(&info.path)).unwrap();
        assert!(!Path::new(&info.path).exists());
    }

    #[test]
    fn test_list_backups_empty() {
        let app_dir = tempfile::tempdir().unwrap();
        let backups = list_backups(app_dir.path());
        assert!(backups.is_empty());
    }

    #[test]
    fn test_restore_rejects_zip_slip() {
        let app_dir = tempfile::tempdir().unwrap();
        let restore_dir = tempfile::tempdir().unwrap();

        // Create a malicious zip with path traversal
        let zip_path = app_dir.path().join("malicious.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.start_file("profiles/../../../evil.txt", options).unwrap();
        zip.write_all(b"pwned").unwrap();
        zip.finish().unwrap();

        let result = restore_backup(&zip_path, restore_dir.path(), None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Malicious zip entry"), "Expected zip-slip rejection, got: {}", err);
    }

    #[test]
    fn test_backup_with_server_configs() {
        let app_dir = tempfile::tempdir().unwrap();
        let zomboid_dir = tempfile::tempdir().unwrap();

        // Create profiles
        let profiles_dir = app_dir.path().join("profiles");
        std::fs::create_dir_all(&profiles_dir).unwrap();
        std::fs::write(profiles_dir.join("p.json"), "{}").unwrap();

        // Create server config
        let server_dir = zomboid_dir.path().join("Server");
        std::fs::create_dir_all(&server_dir).unwrap();
        std::fs::write(server_dir.join("test.ini"), "MaxPlayers=32").unwrap();

        let info = create_backup(app_dir.path(), Some(zomboid_dir.path()), "Full Backup").unwrap();
        assert_eq!(info.profile_count, 1);
        assert!(info.has_server_configs);
    }
}
