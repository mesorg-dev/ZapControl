//! UniFFI surface for zapret-core.
//! JSON-in/JSON-out keeps the Kotlin side simple; typed structs can come later.

use zapret_core::roblox;
use zapret_core::settings::{self, Settings};
use zapret_core::strategy;
use zapret_core::targets;
use zapret_core::tester;

/// Load all strategy templates from a directory. Returns JSON array.
#[uniffi::export]
pub fn load_strategies_json(dir: String) -> String {
    let list = strategy::load_all(std::path::Path::new(&dir));
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
}

/// Load targets from a targets.txt path; falls back to defaults.
/// Returns JSON array of {name, url, ping}.
#[uniffi::export]
pub fn load_targets_json(path: String) -> String {
    let list = targets::load_targets(std::path::Path::new(&path));
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
}

/// Fetch roblox-ips.json from URL. Returns RobloxFile JSON.
#[uniffi::export]
pub async fn fetch_roblox_json(url: String) -> Result<String, String> {
    let file = roblox::fetch(&url).await?;
    serde_json::to_string(&file).map_err(|e| e.to_string())
}

/// Build DNS/hosts body from RobloxFile JSON.
#[uniffi::export]
pub fn build_roblox_hosts(file_json: String) -> Result<String, String> {
    let file = roblox::parse_file(&file_json)?;
    Ok(roblox::build_hosts_block(&file))
}

/// Parse TestSummary JSON and return best strategy id (or null).
#[uniffi::export]
pub fn pick_best_strategy(summary_json: String) -> Option<String> {
    let summary: tester::TestSummary = serde_json::from_str(&summary_json).ok()?;
    summary.best_strategy
}

/// Load settings JSON from a path (Android DataStore path or Windows default).
#[uniffi::export]
pub fn load_settings_json(path: String) -> String {
    let s: Settings = if path.is_empty() {
        settings::load()
    } else {
        settings::load_from(std::path::Path::new(&path))
    };
    serde_json::to_string(&s).unwrap_or_default()
}

/// Save settings JSON to a path; returns "ok" or error string.
#[uniffi::export]
pub fn save_settings_json(path: String, json: String) -> Result<(), String> {
    let s: Settings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let p = std::path::Path::new(&path);
    if path.is_empty() {
        settings::save(&s).map_err(|e| e.to_string())
    } else {
        settings::save_to(p, &s).map_err(|e| e.to_string())
    }
}

uniffi::setup_scaffolding!();
