//! Thin re-export of zapret-core settings with Windows paths.
pub use zapret_core::settings::{AutostartMode, Settings, SharedSettings};

pub fn load() -> Settings {
    zapret_core::settings::load()
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    zapret_core::settings::save(settings)
}
