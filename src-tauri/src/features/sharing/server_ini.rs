use crate::app_core::error::AppError;
use super::formats::{ImportPreview, MissingMod};

/// Parse a PZ server.ini file and extract mod IDs and workshop IDs.
///
/// Looks for lines starting with `Mods=` and `WorkshopItems=`,
/// where values are semicolon-separated.
pub fn parse_server_ini(content: &str) -> Result<ServerIniData, AppError> {
    let mut mod_ids = Vec::new();
    let mut workshop_ids = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(value) = trimmed.strip_prefix("Mods=") {
            mod_ids = value
                .split(';')
                .map(|s| {
                    let cleaned = s.trim().trim_start_matches('\\');
                    // Handle workshopId/modId format (e.g. "2788256295/ammomaker" → "ammomaker")
                    if let Some(slash_idx) = cleaned.find('/') {
                        let prefix = &cleaned[..slash_idx];
                        if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
                            return cleaned[slash_idx + 1..].to_string();
                        }
                    }
                    cleaned.to_string()
                })
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(value) = trimmed.strip_prefix("WorkshopItems=") {
            workshop_ids = value
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    Ok(ServerIniData {
        mod_ids,
        workshop_ids,
    })
}

/// Raw data extracted from a server.ini file.
#[derive(Debug, Clone)]
pub struct ServerIniData {
    pub mod_ids: Vec<String>,
    pub workshop_ids: Vec<String>,
}

/// Parse a server.ini file and produce an ImportPreview by cross-referencing
/// with known installed mods.
pub fn import_from_server_ini(
    content: &str,
    known_mod_ids: &[String],
    known_workshop_ids: &[(&str, &str)], // (mod_id, workshop_id) pairs
) -> Result<ImportPreview, AppError> {
    let data = parse_server_ini(content)?;

    let mut found = Vec::new();
    let mut missing = Vec::new();

    for (i, mod_id) in data.mod_ids.iter().enumerate() {
        if known_mod_ids.contains(mod_id) {
            found.push(mod_id.clone());
        } else {
            // Try to find the workshop ID for this position
            let workshop_id = data.workshop_ids.get(i).cloned();

            // Also try to find by workshop ID if the mod ID doesn't match
            let found_by_workshop = workshop_id.as_ref().and_then(|wid| {
                known_workshop_ids
                    .iter()
                    .find(|(_, w)| w == wid)
                    .map(|(mid, _)| mid.to_string())
            });

            if let Some(mid) = found_by_workshop {
                found.push(mid);
            } else {
                missing.push(MissingMod {
                    id: mod_id.clone(),
                    name: None,
                    workshop_id,
                });
            }
        }
    }

    Ok(ImportPreview {
        total: data.mod_ids.len(),
        found,
        missing,
        detected_format: "server-ini".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_server_ini_basic() {
        let content = r#"
PVP=false
PauseEmpty=true
Mods=mod1;mod2;mod3
WorkshopItems=111;222;333
MaxPlayers=32
"#;

        let data = parse_server_ini(content).unwrap();
        assert_eq!(data.mod_ids, vec!["mod1", "mod2", "mod3"]);
        assert_eq!(data.workshop_ids, vec!["111", "222", "333"]);
    }

    #[test]
    fn parse_server_ini_empty_values() {
        let content = "Mods=\nWorkshopItems=\n";

        let data = parse_server_ini(content).unwrap();
        assert!(data.mod_ids.is_empty());
        assert!(data.workshop_ids.is_empty());
    }

    #[test]
    fn parse_server_ini_trailing_semicolons() {
        let content = "Mods=mod1;mod2;\nWorkshopItems=111;222;\n";

        let data = parse_server_ini(content).unwrap();
        assert_eq!(data.mod_ids, vec!["mod1", "mod2"]);
        assert_eq!(data.workshop_ids, vec!["111", "222"]);
    }

    #[test]
    fn parse_server_ini_strips_backslashes() {
        let content = r#"Mods=\ModA;\ModB;ModC
WorkshopItems=111;222;333
"#;
        let data = parse_server_ini(content).unwrap();
        assert_eq!(data.mod_ids, vec!["ModA", "ModB", "ModC"]);
    }

    #[test]
    fn parse_server_ini_strips_workshop_id_prefix() {
        let content = "Mods=\\2788256295/ammomaker;\\1299328280/ToadTraits;\\NormalMod\n";
        let data = parse_server_ini(content).unwrap();
        assert_eq!(data.mod_ids, vec!["ammomaker", "ToadTraits", "NormalMod"]);
    }

    #[test]
    fn parse_server_ini_no_mod_lines() {
        let content = "PVP=false\nMaxPlayers=32\n";

        let data = parse_server_ini(content).unwrap();
        assert!(data.mod_ids.is_empty());
        assert!(data.workshop_ids.is_empty());
    }

    #[test]
    fn import_from_server_ini_with_known_mods() {
        let content = "Mods=mod1;mod2;mod3\nWorkshopItems=111;222;333\n";
        let known = vec!["mod1".to_string(), "mod3".to_string()];
        let known_workshop: Vec<(&str, &str)> = vec![("mod1", "111"), ("mod3", "333")];

        let preview = import_from_server_ini(content, &known, &known_workshop).unwrap();

        assert_eq!(preview.total, 3);
        assert_eq!(preview.found, vec!["mod1", "mod3"]);
        assert_eq!(preview.missing.len(), 1);
        assert_eq!(preview.missing[0].id, "mod2");
        assert_eq!(preview.missing[0].workshop_id, Some("222".to_string()));
        assert_eq!(preview.detected_format, "server-ini");
    }

    #[test]
    fn import_resolves_by_workshop_id() {
        // Mod has a different ID locally but same workshop ID
        let content = "Mods=SomeOtherId;mod2\nWorkshopItems=111;222\n";
        let known = vec!["mod1".to_string(), "mod2".to_string()];
        let known_workshop: Vec<(&str, &str)> = vec![("mod1", "111"), ("mod2", "222")];

        let preview = import_from_server_ini(content, &known, &known_workshop).unwrap();

        assert_eq!(preview.total, 2);
        // "SomeOtherId" isn't in known_mod_ids, but workshop ID 111 maps to mod1
        assert_eq!(preview.found, vec!["mod1", "mod2"]);
        assert!(preview.missing.is_empty());
    }

    #[test]
    fn import_with_spaces_around_values() {
        let content = "Mods= mod1 ; mod2 \nWorkshopItems= 111 ; 222 \n";
        let known = vec!["mod1".to_string(), "mod2".to_string()];
        let known_workshop: Vec<(&str, &str)> = vec![];

        let preview = import_from_server_ini(content, &known, &known_workshop).unwrap();

        assert_eq!(preview.total, 2);
        assert_eq!(preview.found.len(), 2);
    }
}
