//! Steam Web API client for Workshop metadata.
//!
//! Uses public endpoints that don't require an API key:
//! - ISteamRemoteStorage/GetPublishedFileDetails — batch file details
//! - IPublishedFileService/QueryFiles — search Workshop
//!
//! PZ's Steam App ID is 108600.

use serde::{Deserialize, Serialize};

const PZ_APP_ID: u64 = 108600;
const BATCH_SIZE: usize = 100; // Steam API limit per request

/// Workshop item metadata from Steam API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemMeta {
    pub published_file_id: String,
    pub title: String,
    pub description: String,
    pub preview_url: Option<String>,
    pub tags: Vec<String>,
    pub time_created: u64,
    pub time_updated: u64,
    pub file_size: u64,
    pub subscriptions: u64,
    pub favorited: u64,
    pub views: u64,
    pub creator_id: String,
    /// Workshop dependencies (other mod IDs this mod requires)
    pub dependencies: Vec<String>,
    /// Whether this item was found on Steam
    pub found: bool,
}

/// Fetch metadata for multiple Workshop items by their published file IDs.
pub async fn get_published_file_details(file_ids: &[String]) -> Result<Vec<WorkshopItemMeta>, String> {
    let client = reqwest::Client::new();
    let mut all_results = Vec::new();

    // Process in batches of BATCH_SIZE
    for chunk in file_ids.chunks(BATCH_SIZE) {
        let mut form_data: Vec<(String, String)> = vec![
            ("itemcount".to_string(), chunk.len().to_string()),
        ];
        for (i, id) in chunk.iter().enumerate() {
            form_data.push((format!("publishedfileids[{}]", i), id.clone()));
        }

        let resp = client
            .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
            .form(&form_data)
            .send()
            .await
            .map_err(|e| format!("Steam API request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Steam API returned status {}", resp.status()));
        }

        let body: SteamApiResponse = resp.json().await
            .map_err(|e| format!("Failed to parse Steam API response: {}", e))?;

        if let Some(details) = body.response.publishedfiledetails {
            for item in details {
                all_results.push(parse_steam_item(item));
            }
        }
    }

    Ok(all_results)
}

/// Fetch dependencies for a single Workshop item.
pub async fn get_item_dependencies(file_id: &str) -> Result<Vec<String>, String> {
    // The GetPublishedFileDetails response doesn't always include children.
    // Use a separate call for dependency info if needed.
    let items = get_published_file_details(&[file_id.to_string()]).await?;
    Ok(items.first().map(|i| i.dependencies.clone()).unwrap_or_default())
}

/// Search Workshop items by text query.
pub async fn search_workshop(
    query: &str,
    page: u32,
    num_per_page: u32,
) -> Result<Vec<WorkshopItemMeta>, String> {
    let client = reqwest::Client::new();

    let url = format!(
        "https://api.steampowered.com/IPublishedFileService/QueryFiles/v1/\
         ?key=&query_type=1\
         &page={}\
         &numperpage={}\
         &appid={}\
         &search_text={}\
         &return_tags=true\
         &return_metadata=true\
         &return_previews=true\
         &return_children=true",
        page, num_per_page, PZ_APP_ID,
        urlencoding::encode(query)
    );

    let resp = client.get(&url).send().await
        .map_err(|e| format!("Steam search API failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Steam API returned status {}", resp.status()));
    }

    let body: SteamQueryResponse = resp.json().await
        .map_err(|e| format!("Failed to parse search response: {}", e))?;

    let mut results = Vec::new();
    if let Some(details) = body.response.publishedfiledetails {
        for item in details {
            results.push(parse_steam_item(item));
        }
    }

    Ok(results)
}

fn parse_steam_item(item: SteamPublishedFile) -> WorkshopItemMeta {
    let tags: Vec<String> = item.tags.unwrap_or_default()
        .into_iter()
        .map(|t| t.tag)
        .collect();

    let dependencies: Vec<String> = item.children.unwrap_or_default()
        .into_iter()
        .map(|c| c.publishedfileid)
        .collect();

    WorkshopItemMeta {
        published_file_id: item.publishedfileid.unwrap_or_default(),
        title: item.title.unwrap_or_default(),
        description: item.description.unwrap_or_default(),
        preview_url: item.preview_url,
        tags,
        time_created: item.time_created.unwrap_or(0),
        time_updated: item.time_updated.unwrap_or(0),
        file_size: item.file_size.unwrap_or(0),
        subscriptions: item.subscriptions.unwrap_or(0),
        favorited: item.favorited.unwrap_or(0),
        views: item.views.unwrap_or(0),
        creator_id: item.creator.unwrap_or_default(),
        dependencies,
        found: item.result == Some(1),
    }
}

// --- Steam API response types (internal) ---

#[derive(Deserialize)]
struct SteamApiResponse {
    response: SteamApiResponseInner,
}

#[derive(Deserialize)]
struct SteamApiResponseInner {
    publishedfiledetails: Option<Vec<SteamPublishedFile>>,
}

#[derive(Deserialize)]
struct SteamQueryResponse {
    response: SteamQueryResponseInner,
}

#[derive(Deserialize)]
struct SteamQueryResponseInner {
    #[serde(default)]
    publishedfiledetails: Option<Vec<SteamPublishedFile>>,
    #[serde(default)]
    #[allow(dead_code)]
    total: Option<u32>,
}

#[derive(Deserialize)]
struct SteamPublishedFile {
    publishedfileid: Option<String>,
    result: Option<i32>,
    title: Option<String>,
    description: Option<String>,
    preview_url: Option<String>,
    tags: Option<Vec<SteamTag>>,
    time_created: Option<u64>,
    time_updated: Option<u64>,
    file_size: Option<u64>,
    subscriptions: Option<u64>,
    favorited: Option<u64>,
    views: Option<u64>,
    creator: Option<String>,
    children: Option<Vec<SteamChild>>,
}

#[derive(Deserialize)]
struct SteamTag {
    tag: String,
}

#[derive(Deserialize)]
struct SteamChild {
    publishedfileid: String,
}

/// URL encoding helper (avoid adding another dep).
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("test&foo=bar"), "test%26foo%3Dbar");
    }
}
