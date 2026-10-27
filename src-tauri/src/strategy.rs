//! Windows-specific glue over zapret-core strategy engine.
use crate::config;
use crate::paths;

pub use zapret_core::strategy::{RenderCtx, Strategy};

/// Load strategies from install-root/strategies.
pub fn load_all() -> Vec<Strategy> {
    zapret_core::strategy::load_all(&paths::strategies_dir())
}

/// RenderCtx bound to current install layout + saved game filter.
pub fn from_env() -> RenderCtx {
    let gf = config::load().game_filter;
    RenderCtx::new(paths::bin_dir(), paths::lists_dir(), gf)
}
