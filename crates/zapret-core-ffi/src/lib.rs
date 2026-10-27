//! UniFFI surface for zapret-core.
//! JSON-in/JSON-out keeps the Kotlin side simple; typed structs can come later.

use zapret_core::roblox;
use zapret_core::settings::{self, Settings};
use zapret_core::strategy;
use zapret_core::targets;
use zapret_core::tester;

/// Flat error: single exception class in Kotlin, message = Display.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum FfiError {
    #[error("{0}")]
    Message(String),
}

impl From<String> for FfiError {
    fn from(s: String) -> Self {
        FfiError::Message(s)
    }
}

fn err(s: impl std::fmt::Display) -> FfiError {
    FfiError::Message(s.to_string())
}

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
pub async fn fetch_roblox_json(url: String) -> Result<String, FfiError> {
    let file = roblox::fetch(&url).await.map_err(err)?;
    serde_json::to_string(&file).map_err(err)
}

/// Build DNS/hosts body from RobloxFile JSON.
#[uniffi::export]
pub fn build_roblox_hosts(file_json: String) -> Result<String, FfiError> {
    let file = roblox::parse_file(&file_json).map_err(err)?;
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

/// Save settings JSON to a path; returns Ok or FfiError.
#[uniffi::export]
pub fn save_settings_json(path: String, json: String) -> Result<(), FfiError> {
    let s: Settings = serde_json::from_str(&json).map_err(err)?;
    let p = std::path::Path::new(&path);
    if path.is_empty() {
        settings::save(&s).map_err(|e| err(e.to_string()))
    } else {
        settings::save_to(p, &s).map_err(|e| err(e.to_string()))
    }
}

uniffi::setup_scaffolding!();
