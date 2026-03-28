//! Search index for PZ API documentation.
//! Loads the API snapshot JSON and provides fast text search over classes, methods, events.

use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSearchResult {
    pub kind: String,        // "java_class", "lua_class", "method", "event", "field"
    pub class_name: String,
    pub name: String,
    pub description: Option<String>,
    pub signature: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiClassDetail {
    pub name: String,
    pub kind: String, // "java" or "lua"
    pub qualified_name: Option<String>,
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: Vec<ApiFieldInfo>,
    pub methods: Vec<ApiMethodInfo>,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFieldInfo {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMethodInfo {
    pub name: String,
    pub params: Vec<ApiParamInfo>,
    pub returns: Vec<String>,
    pub description: Option<String>,
    pub overload_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiParamInfo {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEventInfo {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<ApiParamInfo>,
    pub context: Vec<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStats {
    pub java_class_count: usize,
    pub lua_class_count: usize,
    pub event_count: usize,
    pub java_method_count: usize,
    pub lua_method_count: usize,
    pub version: String,
}

/// Raw snapshot format (matches pz-api-extractor output)
#[derive(Debug, Deserialize)]
pub struct RawSnapshot {
    pub version: String,
    pub java_classes: std::collections::HashMap<String, RawJavaClass>,
    pub lua_classes: std::collections::HashMap<String, RawLuaClass>,
    pub events: Vec<RawEvent>,
    pub stats: RawStats,
}

#[derive(Debug, Deserialize)]
pub struct RawJavaClass {
    pub qualified_name: String,
    pub simple_name: String,
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<RawMethod>,
    pub fields: Vec<RawField>,
}

#[derive(Debug, Deserialize)]
pub struct RawLuaClass {
    pub name: String,
    pub parent: Option<String>,
    pub type_field: Option<String>,
    pub methods: Vec<RawMethod>,
    pub fields: Vec<RawField>,
    pub source_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawMethod {
    pub name: String,
    pub params: Vec<RawParam>,
    pub returns: Vec<String>,
    pub description: Option<String>,
    pub overload_index: u32,
}

#[derive(Debug, Deserialize)]
pub struct RawParam {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RawField {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawEvent {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<RawParam>,
    pub context: Vec<String>,
    pub deprecated: bool,
}

#[derive(Debug, Deserialize)]
pub struct RawStats {
    pub java_class_count: usize,
    pub lua_class_count: usize,
    pub event_count: usize,
    pub java_method_count: usize,
    pub lua_method_count: usize,
}

/// Load and index the API snapshot for searching.
pub fn load_snapshot(path: &Path) -> Result<RawSnapshot, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read API snapshot: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse API snapshot: {}", e))
}

/// Search the API snapshot for matching classes, methods, events.
pub fn search_api(snapshot: &RawSnapshot, query: &str, limit: usize) -> Vec<ApiSearchResult> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();
    let mut results = Vec::new();

    if terms.is_empty() {
        return results;
    }

    // Search Java classes
    for (name, cls) in &snapshot.java_classes {
        let name_lower = name.to_lowercase();
        if terms.iter().all(|t| name_lower.contains(t)) {
            results.push(ApiSearchResult {
                kind: "java_class".into(),
                class_name: name.clone(),
                name: name.clone(),
                description: Some(format!("Java class — {} methods, {} fields",
                    cls.methods.len(), cls.fields.len())),
                signature: cls.parent.as_ref().map(|p| format!("extends {}", p)),
                score: if name_lower == query_lower { 1.0 } else { 0.9 },
            });
        }
        // Search methods
        for method in &cls.methods {
            let method_lower = method.name.to_lowercase();
            let full = format!("{}.{}", name_lower, method_lower);
            if terms.iter().all(|t| full.contains(t) || method_lower.contains(t)) {
                let sig = format!("{}({}) -> {}",
                    method.name,
                    method.params.iter().map(|p| format!("{}: {}", p.name, p.param_type)).collect::<Vec<_>>().join(", "),
                    if method.returns.is_empty() { "void".to_string() } else { method.returns.join(", ") }
                );
                results.push(ApiSearchResult {
                    kind: "method".into(),
                    class_name: name.clone(),
                    name: method.name.clone(),
                    description: method.description.clone(),
                    signature: Some(sig),
                    score: if method_lower == query_lower { 0.85 } else { 0.7 },
                });
            }
        }
    }

    // Search Lua classes
    for (name, cls) in &snapshot.lua_classes {
        let name_lower = name.to_lowercase();
        if terms.iter().all(|t| name_lower.contains(t)) {
            results.push(ApiSearchResult {
                kind: "lua_class".into(),
                class_name: name.clone(),
                name: name.clone(),
                description: Some(format!("Lua class — {} methods", cls.methods.len())),
                signature: cls.parent.as_ref().map(|p| format!("extends {}", p)),
                score: if name_lower == query_lower { 1.0 } else { 0.9 },
            });
        }
        for method in &cls.methods {
            let method_lower = method.name.to_lowercase();
            let full = format!("{}.{}", name_lower, method_lower);
            if terms.iter().all(|t| full.contains(t) || method_lower.contains(t)) {
                results.push(ApiSearchResult {
                    kind: "method".into(),
                    class_name: name.clone(),
                    name: method.name.clone(),
                    description: method.description.clone(),
                    signature: None,
                    score: if method_lower == query_lower { 0.85 } else { 0.7 },
                });
            }
        }
    }

    // Search events
    for event in &snapshot.events {
        let name_lower = event.name.to_lowercase();
        if terms.iter().all(|t| name_lower.contains(t)) {
            results.push(ApiSearchResult {
                kind: "event".into(),
                class_name: "Events".into(),
                name: event.name.clone(),
                description: event.description.clone(),
                signature: Some(format!("Events.{}.Add(function({}) end)",
                    event.name,
                    event.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ")
                )),
                score: if name_lower == query_lower { 1.0 } else { 0.8 },
            });
        }
    }

    // Sort by score descending, limit
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Get full class details
pub fn get_class_detail(snapshot: &RawSnapshot, class_name: &str) -> Option<ApiClassDetail> {
    // Check Java classes
    if let Some(cls) = snapshot.java_classes.get(class_name) {
        return Some(ApiClassDetail {
            name: cls.simple_name.clone(),
            kind: "java".into(),
            qualified_name: Some(cls.qualified_name.clone()),
            parent: cls.parent.clone(),
            interfaces: cls.interfaces.clone(),
            fields: cls.fields.iter().map(|f| ApiFieldInfo {
                name: f.name.clone(), field_type: f.field_type.clone(), description: f.description.clone(),
            }).collect(),
            methods: cls.methods.iter().map(|m| ApiMethodInfo {
                name: m.name.clone(),
                params: m.params.iter().map(|p| ApiParamInfo { name: p.name.clone(), param_type: p.param_type.clone() }).collect(),
                returns: m.returns.clone(),
                description: m.description.clone(),
                overload_index: m.overload_index,
            }).collect(),
            source_file: None,
        });
    }

    // Check Lua classes
    if let Some(cls) = snapshot.lua_classes.get(class_name) {
        return Some(ApiClassDetail {
            name: cls.name.clone(),
            kind: "lua".into(),
            qualified_name: None,
            parent: cls.parent.clone(),
            interfaces: Vec::new(),
            fields: cls.fields.iter().map(|f| ApiFieldInfo {
                name: f.name.clone(), field_type: f.field_type.clone(), description: f.description.clone(),
            }).collect(),
            methods: cls.methods.iter().map(|m| ApiMethodInfo {
                name: m.name.clone(),
                params: m.params.iter().map(|p| ApiParamInfo { name: p.name.clone(), param_type: p.param_type.clone() }).collect(),
                returns: m.returns.clone(),
                description: m.description.clone(),
                overload_index: m.overload_index,
            }).collect(),
            source_file: cls.source_file.clone(),
        });
    }

    None
}

/// Get all events
pub fn get_events(snapshot: &RawSnapshot) -> Vec<ApiEventInfo> {
    snapshot.events.iter().map(|e| ApiEventInfo {
        name: e.name.clone(),
        description: e.description.clone(),
        params: e.params.iter().map(|p| ApiParamInfo { name: p.name.clone(), param_type: p.param_type.clone() }).collect(),
        context: e.context.clone(),
        deprecated: e.deprecated,
    }).collect()
}

/// Get snapshot stats
pub fn get_stats(snapshot: &RawSnapshot) -> ApiStats {
    ApiStats {
        java_class_count: snapshot.stats.java_class_count,
        lua_class_count: snapshot.stats.lua_class_count,
        event_count: snapshot.stats.event_count,
        java_method_count: snapshot.stats.java_method_count,
        lua_method_count: snapshot.stats.lua_method_count,
        version: snapshot.version.clone(),
    }
}
