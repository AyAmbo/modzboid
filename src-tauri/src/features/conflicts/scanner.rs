use std::path::Path;

use regex::Regex;
use sqlx::sqlite::SqlitePool;
use walkdir::WalkDir;

use crate::app_core::error::AppError;

/// File type classification for conflict detection.
/// Returns None for translation files (excluded from caching).
pub fn classify_file(relative_path: &str) -> Option<&'static str> {
    let lower = relative_path.to_lowercase();

    // Exclude translation files entirely
    if lower.starts_with("translate/") || lower.starts_with("translate\\") {
        return None;
    }

    if (lower.starts_with("lua/") || lower.starts_with("lua\\")) && lower.ends_with(".lua") {
        Some("lua")
    } else if (lower.starts_with("scripts/") || lower.starts_with("scripts\\"))
        && lower.ends_with(".txt")
    {
        Some("script")
    } else if lower.starts_with("textures/")
        || lower.starts_with("textures\\")
        || lower.ends_with(".png")
        || lower.ends_with(".dds")
    {
        Some("texture")
    } else if lower.starts_with("models/")
        || lower.starts_with("models\\")
        || lower.ends_with(".fbx")
        || lower.ends_with(".x")
    {
        Some("model")
    } else if lower.starts_with("sound/")
        || lower.starts_with("sound\\")
        || lower.ends_with(".ogg")
        || lower.ends_with(".wav")
    {
        Some("sound")
    } else {
        Some("other")
    }
}

pub struct ModFileEntry {
    pub relative_path: String,
    pub file_type: String,
}

/// Walk a mod's media/ directory and return all file entries (excluding translations).
pub fn collect_mod_files(media_path: &Path) -> Vec<ModFileEntry> {
    if !media_path.exists() {
        return vec![];
    }

    let mut entries = Vec::new();
    for entry in WalkDir::new(media_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(media_path) {
            let rel_str = relative.to_string_lossy().replace('\\', "/");
            if let Some(file_type) = classify_file(&rel_str) {
                entries.push(ModFileEntry {
                    relative_path: rel_str,
                    file_type: file_type.to_string(),
                });
            }
        }
    }
    entries
}

/// Batch insert mod file entries into the mod_files table.
pub async fn cache_mod_files(
    pool: &SqlitePool,
    mod_id: &str,
    entries: &[ModFileEntry],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO mod_files (mod_id, relative_path, file_type) VALUES (?, ?, ?)",
        )
        .bind(mod_id)
        .bind(&entry.relative_path)
        .bind(&entry.file_type)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub struct ScriptIdEntry {
    pub script_type: String,
    pub script_id: String,
    pub file_path: String,
}

/// Extract module-qualified script IDs from a PZ script file content.
/// Format: `module MyMod { item MyItem { ... } }` -> ("item", "MyMod.MyItem")
pub fn extract_script_ids(content: &str, file_path: &str) -> Vec<ScriptIdEntry> {
    let mut results = Vec::new();

    let module_re = Regex::new(r"module\s+(\w+)\s*\{").unwrap();
    let block_re = Regex::new(r"(item|recipe|vehicle|fixing|model)\s+(\w+)\s*\{").unwrap();

    let module_matches: Vec<(usize, String)> = module_re
        .captures_iter(content)
        .filter_map(|cap| {
            let m = cap.get(0)?;
            Some((m.start(), cap[1].to_string()))
        })
        .collect();

    for cap in block_re.captures_iter(content) {
        let block_start = cap.get(0).unwrap().start();
        let script_type = &cap[1];
        let name = &cap[2];

        let module_name = module_matches
            .iter()
            .filter(|(pos, _)| *pos < block_start)
            .last()
            .map(|(_, name)| name.as_str())
            .unwrap_or("Base");

        results.push(ScriptIdEntry {
            script_type: script_type.to_string(),
            script_id: format!("{}.{}", module_name, name),
            file_path: file_path.to_string(),
        });
    }

    results
}

/// Batch insert script ID entries into the script_ids table.
pub async fn cache_script_ids(
    pool: &SqlitePool,
    mod_id: &str,
    entries: &[ScriptIdEntry],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO script_ids (mod_id, script_type, script_id, file_path) VALUES (?, ?, ?, ?)",
        )
        .bind(mod_id)
        .bind(&entry.script_type)
        .bind(&entry.script_id)
        .bind(&entry.file_path)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// --- Lua-level analysis for enhanced conflict detection ---

pub struct LuaGlobalEntry {
    pub symbol_name: String,
    pub symbol_type: String, // "function", "table", "variable"
    pub file_path: String,
    pub line: Option<u32>,
}

pub struct EventHookEntry {
    pub event_name: String,
    pub callback_name: String,
    pub file_path: String,
    pub line: Option<u32>,
}

/// Extract global function/table definitions and event hooks from a Lua file.
pub fn extract_lua_globals_and_hooks(content: &str, file_path: &str) -> (Vec<LuaGlobalEntry>, Vec<EventHookEntry>) {
    let mut globals = Vec::new();
    let mut hooks = Vec::new();

    let re_func_global = Regex::new(r#"^function\s+(\w+)\s*\("#).unwrap();
    let re_func_method = Regex::new(r#"^function\s+(\w+)[:.]\w+\s*\("#).unwrap();
    let re_table_init = Regex::new(r#"^(\w+)\s*=\s*(\w+\s*and\s*\w+\s*or\s*)?\s*\{\s*\}"#).unwrap();
    let re_derive = Regex::new(r#"^(\w+)\s*=\s*\w+:derive\s*\("#).unwrap();
    let re_event_add = Regex::new(r#"Events\.(\w+)\.Add\s*\(\s*(\w+)"#).unwrap();
    let re_global_assign = Regex::new(r#"^(\w+)\s*=\s*function\s*\("#).unwrap();

    // Known PZ built-ins that mods are expected to use (not define)
    let builtins: std::collections::HashSet<&str> = [
        "Events", "LuaEventManager", "require", "print", "tostring", "tonumber",
        "type", "pairs", "ipairs", "table", "string", "math", "os", "io",
        "pcall", "xpcall", "error", "assert", "select", "unpack", "rawget",
        "rawset", "setmetatable", "getmetatable", "next",
    ].iter().copied().collect();

    let mut depth = 0i32;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("--") {
            continue;
        }

        // Save depth before updates for top-level checks
        let is_top_level = depth == 0;

        // Track block depth (rough heuristic)
        if trimmed == "end" || trimmed.starts_with("end ") || trimmed.starts_with("end)") || trimmed.starts_with("end,") || trimmed.starts_with("end;") {
            depth -= 1;
            if depth < 0 { depth = 0; }
        }
        // Block openers
        if trimmed.starts_with("function ") || trimmed.starts_with("if ") || trimmed.starts_with("for ") || trimmed.starts_with("while ") || trimmed == "do" {
            depth += 1;
        }

        // Skip local declarations
        if trimmed.starts_with("local ") {
            continue;
        }

        // Event hooks — detect at any depth
        for caps in re_event_add.captures_iter(trimmed) {
            hooks.push(EventHookEntry {
                event_name: caps[1].to_string(),
                callback_name: caps[2].to_string(),
                file_path: file_path.to_string(),
                line: Some((line_num + 1) as u32),
            });
        }

        // Only track top-level globals
        if !is_top_level {
            continue;
        }

        // Global function: function MyFunc(...)
        if let Some(caps) = re_func_global.captures(trimmed) {
            let name = &caps[1];
            if !builtins.contains(name) && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                globals.push(LuaGlobalEntry {
                    symbol_name: name.to_string(),
                    symbol_type: "function".into(),
                    file_path: file_path.to_string(),
                    line: Some((line_num + 1) as u32),
                });
            }
            continue;
        }

        // Method definition implies class: function ClassName:method(...)
        // Only record the class name if it's a table being used, not the method
        if let Some(caps) = re_func_method.captures(trimmed) {
            let class_name = &caps[1];
            if !builtins.contains(class_name) && class_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                // Don't duplicate — only add if not already tracked
                if !globals.iter().any(|g| g.symbol_name == class_name) {
                    globals.push(LuaGlobalEntry {
                        symbol_name: class_name.to_string(),
                        symbol_type: "table".into(),
                        file_path: file_path.to_string(),
                        line: Some((line_num + 1) as u32),
                    });
                }
            }
            continue;
        }

        // Class derivation: MyClass = ParentClass:derive(...)
        if let Some(caps) = re_derive.captures(trimmed) {
            let name = &caps[1];
            globals.push(LuaGlobalEntry {
                symbol_name: name.to_string(),
                symbol_type: "table".into(),
                file_path: file_path.to_string(),
                line: Some((line_num + 1) as u32),
            });
            continue;
        }

        // Global table init: MyTable = {} or MyTable = MyTable or {}
        if let Some(caps) = re_table_init.captures(trimmed) {
            let name = &caps[1];
            if !builtins.contains(name) && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                if !globals.iter().any(|g| g.symbol_name == name) {
                    globals.push(LuaGlobalEntry {
                        symbol_name: name.to_string(),
                        symbol_type: "table".into(),
                        file_path: file_path.to_string(),
                        line: Some((line_num + 1) as u32),
                    });
                }
            }
            continue;
        }

        // Global function assignment: MyFunc = function(...)
        if let Some(caps) = re_global_assign.captures(trimmed) {
            let name = &caps[1];
            if !builtins.contains(name) && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                globals.push(LuaGlobalEntry {
                    symbol_name: name.to_string(),
                    symbol_type: "function".into(),
                    file_path: file_path.to_string(),
                    line: Some((line_num + 1) as u32),
                });
            }
        }
    }

    (globals, hooks)
}

/// Batch insert Lua globals into the lua_globals table.
pub async fn cache_lua_globals(
    pool: &SqlitePool,
    mod_id: &str,
    entries: &[LuaGlobalEntry],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO lua_globals (mod_id, symbol_name, symbol_type, file_path, line) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(mod_id)
        .bind(&entry.symbol_name)
        .bind(&entry.symbol_type)
        .bind(&entry.file_path)
        .bind(entry.line.map(|l| l as i64))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Batch insert event hooks into the event_hooks table.
pub async fn cache_event_hooks(
    pool: &SqlitePool,
    mod_id: &str,
    entries: &[EventHookEntry],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO event_hooks (mod_id, event_name, callback_name, file_path, line) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(mod_id)
        .bind(&entry.event_name)
        .bind(&entry.callback_name)
        .bind(&entry.file_path)
        .bind(entry.line.map(|l| l as i64))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_lua_file() {
        assert_eq!(classify_file("lua/client/Foo.lua"), Some("lua"));
        assert_eq!(classify_file("lua/shared/Bar.lua"), Some("lua"));
    }

    #[test]
    fn test_classify_script_file() {
        assert_eq!(classify_file("scripts/items.txt"), Some("script"));
    }

    #[test]
    fn test_classify_texture() {
        assert_eq!(classify_file("textures/Item_Gun.png"), Some("texture"));
    }

    #[test]
    fn test_classify_translation_excluded() {
        assert_eq!(classify_file("Translate/EN/items.txt"), None);
        assert_eq!(classify_file("Translate/RU/UI.txt"), None);
    }

    #[test]
    fn test_classify_model() {
        assert_eq!(classify_file("models/vehicles/car.fbx"), Some("model"));
    }

    #[test]
    fn test_classify_sound() {
        assert_eq!(classify_file("sound/gunshot.ogg"), Some("sound"));
    }

    #[test]
    fn test_classify_other() {
        assert_eq!(classify_file("clothing/hat.xml"), Some("other"));
    }

    #[test]
    fn test_extract_script_ids_basic() {
        let content = r#"
module TestMod {
    item MyGun {
        Weight = 1.5,
    }
    recipe CraftAmmo {
        Result: Ammo,
    }
}
"#;
        let ids = extract_script_ids(content, "scripts/items.txt");
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].script_type, "item");
        assert_eq!(ids[0].script_id, "TestMod.MyGun");
        assert_eq!(ids[1].script_type, "recipe");
        assert_eq!(ids[1].script_id, "TestMod.CraftAmmo");
    }

    #[test]
    fn test_extract_script_ids_multiple_modules() {
        let content = r#"
module Alpha {
    item Sword { }
}
module Beta {
    item Sword { }
}
"#;
        let ids = extract_script_ids(content, "scripts/weapons.txt");
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].script_id, "Alpha.Sword");
        assert_eq!(ids[1].script_id, "Beta.Sword");
    }

    #[test]
    fn test_extract_script_ids_no_module() {
        let content = "item OrphanItem { }";
        let ids = extract_script_ids(content, "scripts/misc.txt");
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].script_id, "Base.OrphanItem");
    }

    #[test]
    fn test_collect_mod_files_nonexistent() {
        let entries = collect_mod_files(Path::new("/nonexistent/path"));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_lua_globals() {
        let content = r#"
MyMod = {}

function MyMod:doStuff(x)
  print(x)
end

function GlobalHelper(a, b)
  return a + b
end

local function localFunc()
end

Events.OnGameStart.Add(MyMod.doStuff)
"#;
        let (globals, hooks) = extract_lua_globals_and_hooks(content, "lua/client/test.lua");

        // Should find MyMod table and GlobalHelper function
        assert!(globals.iter().any(|g| g.symbol_name == "MyMod" && g.symbol_type == "table"));
        assert!(globals.iter().any(|g| g.symbol_name == "GlobalHelper" && g.symbol_type == "function"));

        // Should NOT find localFunc
        assert!(!globals.iter().any(|g| g.symbol_name == "localFunc"));

        // Should find the event hook
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].event_name, "OnGameStart");
    }

    #[test]
    fn test_extract_derive_class() {
        let content = r#"
MyPanel = ISPanel:derive("MyPanel")
MyPanel.Type = "MyPanel"

function MyPanel:new(x, y)
  local o = {}
  return o
end
"#;
        let (globals, _) = extract_lua_globals_and_hooks(content, "lua/client/MyPanel.lua");
        assert!(globals.iter().any(|g| g.symbol_name == "MyPanel" && g.symbol_type == "table"));
    }

    #[test]
    fn test_extract_event_hooks() {
        let content = r#"
local function onPlayerDeath(player)
  print("died")
end

local function onZombieDead(zombie)
  print("zombie died")
end

Events.OnPlayerDeath.Add(onPlayerDeath)
Events.OnZombieDead.Add(onZombieDead)
"#;
        let (_, hooks) = extract_lua_globals_and_hooks(content, "lua/client/hooks.lua");
        assert_eq!(hooks.len(), 2);
        assert!(hooks.iter().any(|h| h.event_name == "OnPlayerDeath"));
        assert!(hooks.iter().any(|h| h.event_name == "OnZombieDead"));
    }

    #[test]
    fn test_builtin_not_tracked() {
        let content = "Events.OnGameStart.Add(myFunc)\n";
        let (globals, _) = extract_lua_globals_and_hooks(content, "lua/test.lua");
        // "Events" should not appear in globals (it's a built-in)
        assert!(!globals.iter().any(|g| g.symbol_name == "Events"));
    }
}
