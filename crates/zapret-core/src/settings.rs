use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub const DEFAULT_ROBLOX_JSON_URL: &str =
    "https://raw.githubusercontent.com/CHANGE_ME/zapret-data/main/roblox-ips.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub last_strategy: String,
    pub auto_start_bypass: bool,
    pub autostart_mode: AutostartMode,
    pub roblox_fix_enabled: bool,
    pub roblox_json_url: String,
    pub roblox_last_update: Option<String>,
    pub game_filter: GameFilter,
    pub check_updates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutostartMode {
    /// HKCU Run key — launches app with --minimized
    Tray,
    /// Windows service (sc create, start=auto)
    Service,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameFilter {
    pub mode: GameFilterMode,
    pub tcp_range: String,
    pub udp_range: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameFilterMode {
    Disabled,
    Tcp,
    Udp,
    All,
}

impl Default for GameFilter {
    fn default() -> Self {
        Self {
            mode: GameFilterMode::Disabled,
            tcp_range: "1024-65535".into(),
            udp_range: "1024-65535".into(),
        }
    }
}

impl GameFilter {
    pub fn tcp_placeholder(&self) -> String {
        match self.mode {
            GameFilterMode::Disabled => "12".into(),
            GameFilterMode::Udp => "12".into(),
            _ => self.tcp_range.clone(),
        }
    }
    pub fn udp_placeholder(&self) -> String {
        match self.mode {
            GameFilterMode::Disabled => "12".into(),
            GameFilterMode::Tcp => "12".into(),
            _ => self.udp_range.clone(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_strategy: "general".into(),
            auto_start_bypass: false,
            autostart_mode: AutostartMode::None,
            roblox_fix_enabled: false,
            roblox_json_url: DEFAULT_ROBLOX_JSON_URL.into(),
            roblox_last_update: None,
            game_filter: GameFilter::default(),
            check_updates: true,
        }
    }
}

pub type SharedSettings = Arc<RwLock<Settings>>;

pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("zapret-gui")
        .join("settings.json")
}

pub fn load() -> Settings {
    let p = settings_path();
    match std::fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let p = settings_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, serde_json::to_string_pretty(settings).unwrap())
}

/// Load settings from an explicit path (Android/other platforms).
pub fn load_from(path: &std::path::Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Save settings to an explicit path.
pub fn save_to(path: &std::path::Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)
}
