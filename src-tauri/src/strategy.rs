use crate::config::GameFilter;
use crate::paths;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub args_template: String,
}

pub fn display_name(id: &str) -> String {
    match id {
        "general" => "General".into(),
        "exp" => "Experimental".into(),
        other => {
            let base = other.trim_start_matches("alt").trim_start_matches('-');
            if other.starts_with("fake-tls-auto") {
                format!("Fake TLS{}", suffix(other, "fake-tls-auto"))
            } else if other.starts_with("simple-fake") {
                format!("Simple Fake{}", suffix(other, "simple-fake"))
            } else if other.starts_with("alt") && !base.is_empty() && base.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                format!("ALT{}", base)
            } else {
                other.to_string()
            }
        }
    }
}

fn suffix(id: &str, prefix: &str) -> String {
    let rest = id.trim_start_matches(prefix);
    if rest.is_empty() {
        String::new()
    } else {
        format!(" {}", rest.trim_start_matches('-'))
    }
}

pub fn load_all() -> Vec<Strategy> {
    let dir = paths::strategies_dir();
    let mut ids: Vec<String> = Vec::new();

    let index = dir.join("index.txt");
    if let Ok(txt) = std::fs::read_to_string(&index) {
        for line in txt.lines() {
            let l = line.trim();
            if !l.is_empty() && !l.starts_with('#') {
                ids.push(l.to_string());
            }
        }
    }

    if ids.is_empty() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut entries: Vec<String> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|x| x == "txt").unwrap_or(false))
                .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_string()))
                .filter(|s| s != "index")
                .collect();
            entries.sort();
            ids = entries;
        }
    }

    ids.into_iter()
        .filter_map(|id| {
            let p = dir.join(format!("{id}.txt"));
            let template = std::fs::read_to_string(&p).ok()?;
            let template = strip_comments(&template);
            Some(Strategy {
                name: display_name(&id),
                id,
                args_template: template,
            })
        })
        .collect()
}

fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct RenderCtx {
    pub bin: String,
    pub lists: String,
    pub game_filter: GameFilter,
}

impl RenderCtx {
    pub fn from_env() -> Self {
        let bin = path_str(&paths::bin_dir());
        let lists = path_str(&paths::lists_dir());
        let gf = crate::config::load().game_filter;
        Self {
            bin,
            lists,
            game_filter: gf,
        }
    }
}

/// Windows path string with trailing backslash for template substitution.
fn path_str(p: &Path) -> String {
    let mut s = p.to_string_lossy().to_string();
    if !s.ends_with('\\') && !s.ends_with('/') {
        s.push('\\');
    }
    s
}

pub fn render_args(template: &str, ctx: &RenderCtx) -> String {
    template
        .replace("%BIN%", &ctx.bin)
        .replace("%LISTS%", &ctx.lists)
        .replace("%GameFilterTCP%", &ctx.game_filter.tcp_placeholder())
        .replace("%GameFilterUDP%", &ctx.game_filter.udp_placeholder())
}
