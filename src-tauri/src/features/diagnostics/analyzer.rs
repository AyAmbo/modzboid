use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::app_core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub timestamp: Option<String>,
    pub error_type: String,
    pub log_excerpt: String,
    pub suspect_mods: Vec<SuspectMod>,
    pub full_log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspectMod {
    pub mod_id: String,
    pub confidence: String, // "high", "medium", "low"
    pub reason: String,
}

/// Crash marker patterns we scan for (backward from end of log).
const CRASH_MARKERS: &[&str] = &[
    "java.lang.",
    "Exception",
    "ERROR",
    "STACK TRACE",
    "LuaError",
];

/// Analyze the most recent crash from `{zomboid_user_dir}/console.txt`.
///
/// Reads the last 1000 lines, finds the last crash marker, extracts an excerpt,
/// scans for mod references, and ranks suspect mods by mention frequency.
pub fn analyze_crash_log(
    zomboid_user_dir: &Path,
    enabled_mod_ids: &[String],
) -> Result<CrashReport, AppError> {
    let console_path = zomboid_user_dir.join("console.txt");
    if !console_path.exists() {
        return Err(AppError::NotFound(format!(
            "Console log not found: {}",
            console_path.display()
        )));
    }

    let metadata = std::fs::metadata(&console_path)?;
    let file_size = metadata.len();

    // Read at most last 2MB to avoid OOM on huge console.txt files
    const MAX_READ: u64 = 2 * 1024 * 1024;
    let all_lines: Vec<String> = if file_size > MAX_READ {
        let mut file = std::fs::File::open(&console_path)?;
        file.seek(std::io::SeekFrom::End(-(MAX_READ as i64)))?;
        let reader = BufReader::new(file);
        // Skip first partial line after seeking
        let mut lines = reader.lines();
        let _ = lines.next(); // discard partial line
        lines.collect::<Result<Vec<_>, _>>()?
    } else {
        let file = std::fs::File::open(&console_path)?;
        let reader = BufReader::new(file);
        reader.lines().collect::<Result<Vec<_>, _>>()?
    };

    // Take last 1000 lines
    let tail_start = if all_lines.len() > 1000 {
        all_lines.len() - 1000
    } else {
        0
    };
    let tail_lines = &all_lines[tail_start..];

    // Find last crash marker by scanning backward
    let crash_line_idx = find_last_crash_marker(tail_lines);

    let crash_line_idx = match crash_line_idx {
        Some(idx) => idx,
        None => {
            return Ok(CrashReport {
                timestamp: None,
                error_type: "NoCrash".to_string(),
                log_excerpt: "No crash detected in this log file. The game appears to have started normally.".to_string(),
                suspect_mods: vec![],
                full_log_path: console_path.to_string_lossy().into_owned(),
            });
        }
    };

    // Extract 30 lines around the crash (15 before, 15 after)
    let excerpt_start = crash_line_idx.saturating_sub(15);
    let excerpt_end = (crash_line_idx + 15).min(tail_lines.len());
    let excerpt_lines = &tail_lines[excerpt_start..excerpt_end];
    let log_excerpt = excerpt_lines.join("\n");

    // Determine error type from the crash line
    let error_type = classify_error(&tail_lines[crash_line_idx]);

    // Extract timestamp from surrounding lines if available
    let timestamp = extract_timestamp(excerpt_lines);

    // Scan a wider area for mod references (50 lines around crash)
    let scan_start = crash_line_idx.saturating_sub(25);
    let scan_end = (crash_line_idx + 25).min(tail_lines.len());
    let scan_area = &tail_lines[scan_start..scan_end];

    let enabled_set: std::collections::HashSet<&str> =
        enabled_mod_ids.iter().map(|s| s.as_str()).collect();

    let suspect_mods = find_suspect_mods(scan_area, &enabled_set);

    Ok(CrashReport {
        timestamp,
        error_type,
        log_excerpt,
        suspect_mods,
        full_log_path: console_path.to_string_lossy().into_owned(),
    })
}

/// Scan backward through lines to find the last line containing a crash marker.
fn find_last_crash_marker(lines: &[String]) -> Option<usize> {
    for (i, line) in lines.iter().enumerate().rev() {
        for marker in CRASH_MARKERS {
            if line.contains(marker) {
                return Some(i);
            }
        }
    }
    None
}

/// Classify the error type based on the crash line content.
fn classify_error(line: &str) -> String {
    if line.contains("LuaError") {
        "LuaError".to_string()
    } else if line.contains("NullPointerException") {
        "NullPointerException".to_string()
    } else if line.contains("OutOfMemoryError") {
        "OutOfMemoryError".to_string()
    } else if line.contains("StackOverflowError") {
        "StackOverflowError".to_string()
    } else if line.contains("java.lang.") {
        // Extract the exception class name
        if let Some(start) = line.find("java.lang.") {
            let rest = &line[start..];
            let end = rest
                .find(|c: char| c == ':' || c == ' ' || c == '\n')
                .unwrap_or(rest.len());
            rest[..end].to_string()
        } else {
            "JavaException".to_string()
        }
    } else if line.contains("STACK TRACE") {
        "StackTrace".to_string()
    } else if line.contains("Exception") {
        "Exception".to_string()
    } else if line.contains("ERROR") {
        "Error".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Try to extract a timestamp from the lines near the crash.
fn extract_timestamp(lines: &[String]) -> Option<String> {
    // PZ log timestamps typically look like: [dd-mm-yy hh:mm:ss.mmm] or similar
    let ts_re =
        Regex::new(r"\[?(\d{2,4}[-/]\d{2}[-/]\d{2,4}\s+\d{2}:\d{2}:\d{2}(?:\.\d+)?)\]?")
            .ok()?;
    for line in lines.iter().rev() {
        if let Some(caps) = ts_re.captures(line) {
            return Some(caps[1].to_string());
        }
    }
    None
}

/// Scan the area around the crash for mod references.
/// Returns suspects sorted by confidence (high first).
fn find_suspect_mods(lines: &[String], enabled_mod_ids: &std::collections::HashSet<&str>) -> Vec<SuspectMod> {
    let mut mention_counts: HashMap<String, usize> = HashMap::new();
    let mut loaded_near_crash: HashMap<String, bool> = HashMap::new();

    // Pattern: media/lua|scripts/client|server|shared/<mod_id>/
    let media_re = Regex::new(
        r"media/(?:lua|scripts)/(?:client|server|shared)/([^/\s]+)/"
    )
    .expect("invalid regex");

    // Pattern: Loading mod: <mod_id>
    let loading_re = Regex::new(r"Loading\s+mod:\s*(\S+)").expect("invalid regex");

    let combined_text = lines.join("\n");

    // Count media path references
    for caps in media_re.captures_iter(&combined_text) {
        let mod_id = caps[1].to_string();
        *mention_counts.entry(mod_id).or_insert(0) += 1;
    }

    // Check for "Loading mod:" references
    for caps in loading_re.captures_iter(&combined_text) {
        let mod_id = caps[1].to_string();
        loaded_near_crash.insert(mod_id.clone(), true);
        *mention_counts.entry(mod_id).or_insert(0) += 1;
    }

    let mut suspects: Vec<SuspectMod> = Vec::new();

    for (mod_id, count) in &mention_counts {
        // If we have an enabled list, cross-reference
        if !enabled_mod_ids.is_empty() && !enabled_mod_ids.contains(mod_id.as_str()) {
            continue;
        }

        let (confidence, reason) = if *count >= 3 {
            (
                "high".to_string(),
                format!("Referenced {} times in crash area", count),
            )
        } else if *count >= 1 {
            (
                "medium".to_string(),
                format!("Referenced {} time(s) in crash area", count),
            )
        } else if loaded_near_crash.contains_key(mod_id) {
            (
                "low".to_string(),
                "Loaded near the crash location".to_string(),
            )
        } else {
            continue;
        };

        suspects.push(SuspectMod {
            mod_id: mod_id.clone(),
            confidence,
            reason,
        });
    }

    // Sort: high first, then medium, then low
    suspects.sort_by(|a, b| {
        let order = |c: &str| match c {
            "high" => 0,
            "medium" => 1,
            "low" => 2,
            _ => 3,
        };
        order(&a.confidence).cmp(&order(&b.confidence))
    });

    suspects
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_console_log(dir: &Path, content: &str) {
        let path = dir.join("console.txt");
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_analyze_crash_log_lua_error() {
        let dir = tempfile::tempdir().unwrap();
        let log = r#"
[14-03-26 10:00:00.000] Game starting
[14-03-26 10:00:01.000] Loading mod: CoolMod
[14-03-26 10:00:02.000] Loading mod: BrokenMod
[14-03-26 10:00:03.000] Loading file: media/lua/client/BrokenMod/init.lua
[14-03-26 10:00:03.001] Loading file: media/lua/client/BrokenMod/utils.lua
[14-03-26 10:00:03.002] Loading file: media/lua/client/BrokenMod/main.lua
[14-03-26 10:00:04.000] LuaError: attempt to index a nil value
[14-03-26 10:00:04.001]   at media/lua/client/BrokenMod/main.lua:42
[14-03-26 10:00:04.002]   at media/lua/client/BrokenMod/init.lua:10
"#;
        write_console_log(dir.path(), log);

        let result = analyze_crash_log(dir.path(), &["BrokenMod".into(), "CoolMod".into()]);
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.error_type, "LuaError");
        assert!(!report.suspect_mods.is_empty());

        // BrokenMod should be the top suspect with high confidence (4+ mentions)
        let top = &report.suspect_mods[0];
        assert_eq!(top.mod_id, "BrokenMod");
        assert_eq!(top.confidence, "high");
    }

    #[test]
    fn test_analyze_crash_log_java_exception() {
        let dir = tempfile::tempdir().unwrap();
        let log = r#"
Game running normally
Some normal log line
java.lang.NullPointerException: Something went wrong
    at zombie.characters.IsoPlayer.update(IsoPlayer.java:1234)
    at zombie.core.Core.doFrame(Core.java:567)
"#;
        write_console_log(dir.path(), log);

        let result = analyze_crash_log(dir.path(), &[]);
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.error_type, "NullPointerException");
    }

    #[test]
    fn test_analyze_crash_log_no_crash() {
        let dir = tempfile::tempdir().unwrap();
        let log = "Game starting\nAll is well\nNo problems here\n";
        write_console_log(dir.path(), log);

        let result = analyze_crash_log(dir.path(), &[]);
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.error_type, "NoCrash");
        assert!(report.suspect_mods.is_empty());
        assert!(report.log_excerpt.contains("No crash detected"));
    }

    #[test]
    fn test_analyze_crash_log_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = analyze_crash_log(dir.path(), &[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Console log not found"));
    }

    #[test]
    fn test_find_last_crash_marker() {
        let lines: Vec<String> = vec![
            "Normal line".into(),
            "ERROR: first error".into(),
            "Normal line".into(),
            "java.lang.NullPointerException".into(),
            "Normal line after crash".into(),
        ];
        let idx = find_last_crash_marker(&lines);
        assert_eq!(idx, Some(3));
    }

    #[test]
    fn test_classify_error() {
        assert_eq!(classify_error("LuaError: bad index"), "LuaError");
        assert_eq!(
            classify_error("java.lang.NullPointerException: foo"),
            "NullPointerException"
        );
        assert_eq!(
            classify_error("java.lang.OutOfMemoryError"),
            "OutOfMemoryError"
        );
        assert_eq!(
            classify_error("java.lang.RuntimeException: bar"),
            "java.lang.RuntimeException"
        );
        assert_eq!(classify_error("STACK TRACE follows"), "StackTrace");
        assert_eq!(classify_error("ERROR: something"), "Error");
        assert_eq!(classify_error("nothing special"), "Unknown");
    }

    #[test]
    fn test_find_suspect_mods_ranking() {
        let lines: Vec<String> = vec![
            "Loading file: media/lua/client/ModA/init.lua".into(),
            "Loading file: media/lua/client/ModA/main.lua".into(),
            "Loading file: media/lua/client/ModA/utils.lua".into(),
            "Loading file: media/lua/client/ModB/init.lua".into(),
            "LuaError: nil value".into(),
        ];
        let enabled: std::collections::HashSet<&str> =
            ["ModA", "ModB"].iter().cloned().collect();
        let suspects = find_suspect_mods(&lines, &enabled);

        assert_eq!(suspects.len(), 2);
        // ModA has 3 mentions → high
        assert_eq!(suspects[0].mod_id, "ModA");
        assert_eq!(suspects[0].confidence, "high");
        // ModB has 1 mention → medium
        assert_eq!(suspects[1].mod_id, "ModB");
        assert_eq!(suspects[1].confidence, "medium");
    }

    #[test]
    fn test_find_suspect_mods_filters_by_enabled() {
        let lines: Vec<String> = vec![
            "Loading file: media/lua/client/ModA/init.lua".into(),
            "Loading file: media/lua/client/DisabledMod/init.lua".into(),
        ];
        let enabled: std::collections::HashSet<&str> =
            ["ModA"].iter().cloned().collect();
        let suspects = find_suspect_mods(&lines, &enabled);

        assert_eq!(suspects.len(), 1);
        assert_eq!(suspects[0].mod_id, "ModA");
    }

    #[test]
    fn test_extract_timestamp() {
        let lines: Vec<String> = vec![
            "[14-03-26 10:00:04.000] LuaError: something".into(),
            "no timestamp here".into(),
        ];
        let ts = extract_timestamp(&lines);
        assert!(ts.is_some());
        assert!(ts.unwrap().contains("10:00:04"));
    }

    #[test]
    fn test_loading_mod_pattern() {
        let lines: Vec<String> = vec![
            "Loading mod: TestMod".into(),
            "LuaError: crash".into(),
        ];
        let enabled: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let suspects = find_suspect_mods(&lines, &enabled);
        assert!(!suspects.is_empty());
        assert_eq!(suspects[0].mod_id, "TestMod");
    }
}
