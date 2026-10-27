use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTarget {
    pub name: String,
    pub url: Option<String>,
    pub ping: Option<String>,
}

pub fn default_targets() -> Vec<TestTarget> {
    vec![
        t("Discord", "https://discord.com"),
        t("Discord Gateway", "https://gateway.discord.gg"),
        t("Discord CDN", "https://cdn.discordapp.com"),
        t("Discord Updates", "https://updates.discord.com"),
        t("YouTube", "https://www.youtube.com"),
        t("YouTube Short", "https://youtu.be"),
        t("YouTube Image", "https://i.ytimg.com"),
        t("YouTube Redirect", "https://redirector.googlevideo.com"),
        t("Google", "https://www.google.com"),
        t("Cloudflare", "https://www.cloudflare.com"),
        ping_t("DNS 1.1.1.1", "1.1.1.1"),
        ping_t("DNS 8.8.8.8", "8.8.8.8"),
    ]
}

fn t(name: &str, url: &str) -> TestTarget {
    TestTarget {
        name: name.into(),
        url: Some(url.into()),
        ping: Some(host_of(url)),
    }
}

pub fn ping_t(name: &str, ip: &str) -> TestTarget {
    TestTarget {
        name: name.into(),
        url: None,
        ping: Some(ip.into()),
    }
}

fn host_of(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Parse targets.txt content (`Key = "value"` lines, `PING:` prefix supported).
/// Returns empty vec if no valid lines.
pub fn parse_targets_content(content: &str) -> Vec<TestTarget> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, val)) = line.split_once('=') {
            let name = name.trim().to_string();
            let val = val.trim().trim_matches('"').to_string();
            if let Some(ip) = val.strip_prefix("PING:") {
                out.push(ping_t(&name, ip.trim()));
            } else if val.starts_with("http") {
                out.push(t(&name, &val));
            }
        }
    }
    out
}

/// Load targets from a targets.txt file, falling back to defaults when
/// the file is missing or has no valid entries.
pub fn load_targets(path: &std::path::Path) -> Vec<TestTarget> {
    if let Ok(txt) = std::fs::read_to_string(path) {
        let parsed = parse_targets_content(&txt);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    default_targets()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mixed_lines() {
        let txt = r#"
# comment
Discord = "https://discord.com"
DNS 1.1.1.1 = "PING:1.1.1.1"
broken line without equals
YouTube = "not-a-url"
"#;
        let t = parse_targets_content(txt);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].url.as_deref(), Some("https://discord.com"));
        assert_eq!(t[0].ping.as_deref(), Some("discord.com"));
        assert_eq!(t[1].url, None);
        assert_eq!(t[1].ping.as_deref(), Some("1.1.1.1"));
    }

    #[test]
    fn missing_file_gives_defaults() {
        let t = load_targets(std::path::Path::new("/nonexistent/targets.txt"));
        assert_eq!(t.len(), default_targets().len());
    }
}
