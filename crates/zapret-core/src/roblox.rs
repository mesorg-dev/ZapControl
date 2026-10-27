use serde::{Deserialize, Serialize};

pub const HOSTS_BEGIN: &str = "# ZAPRET-GUI ROBLOX BEGIN";
pub const HOSTS_END: &str = "# ZAPRET-GUI ROBLOX END";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxFile {
    pub version: Option<i32>,
    pub updated: Option<String>,
    pub description: Option<String>,
    pub entries: Vec<RobloxEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxEntry {
    pub host: String,
    pub ips: Vec<String>,
}

/// Fetch roblox-ips.json from a URL (cache-busted).
pub async fn fetch(url: &str) -> Result<RobloxFile, String> {
    let bust = uuid_v4_like();
    let full = if url.contains('?') {
        format!("{url}&t={bust}")
    } else {
        format!("{url}?t={bust}")
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&full)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("запрос не удался: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    resp.json::<RobloxFile>()
        .await
        .map_err(|e| format!("не удалось разобрать JSON: {e}"))
}

/// Build hosts-file block (Windows) or DNS answer set (Android) body.
pub fn build_hosts_block(file: &RobloxFile) -> String {
    let mut lines = vec![
        HOSTS_BEGIN.to_string(),
        format!(
            "# updated: {}",
            file.updated.as_deref().unwrap_or("unknown")
        ),
    ];
    for e in &file.entries {
        for ip in &e.ips {
            let ip = ip.trim();
            if ip.is_empty() {
                continue;
            }
            lines.push(format!("{ip} {}", e.host.trim()));
        }
    }
    lines.push(HOSTS_END.to_string());
    lines.join("\n")
}

/// Parse JSON into RobloxFile.
pub fn parse_file(json: &str) -> Result<RobloxFile, String> {
    serde_json::from_str(json).map_err(|e| format!("не удалось разобрать JSON: {e}"))
}

/// Simple unique-ish cache buster without extra deps.
fn uuid_v4_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let rand = std::process::id() as u128;
    format!("{nanos:032x}-{rand:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_contains_markers_and_ips() {
        let f = RobloxFile {
            version: Some(1),
            updated: Some("2026-10-27T00:00:00Z".into()),
            description: None,
            entries: vec![RobloxEntry {
                host: "tr.rbxcdn.com".into(),
                ips: vec!["1.2.3.4".into(), " 5.6.7.8 ".into()],
            }],
        };
        let block = build_hosts_block(&f);
        assert!(block.starts_with(HOSTS_BEGIN));
        assert!(block.contains("1.2.3.4 tr.rbxcdn.com"));
        assert!(block.contains("5.6.7.8 tr.rbxcdn.com"));
        assert!(block.ends_with(HOSTS_END));
    }
}
