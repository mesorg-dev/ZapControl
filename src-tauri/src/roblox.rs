use crate::elev;
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxStatus {
    pub applied: bool,
    pub entry_count: usize,
    pub updated: Option<String>,
}

pub async fn fetch(url: &str) -> Result<RobloxFile, String> {
    // cache-bust
    let bust = uuid::Uuid::new_v4();
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

pub async fn apply(file: &RobloxFile) -> elev::ElevatedResult {
    let block = build_hosts_block(file);
    let block_b64 = base64_encode(block.as_bytes());

    let script = format!(
        r#"
$b64 = '{b64}'
$bytes = [Convert]::FromBase64String($b64)
$block = [Text.Encoding]::UTF8.GetString($bytes)
$hosts = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
Zapret-Log ("hosts: " + $hosts)
if (-not (Test-Path -LiteralPath $hosts)) {{ throw "hosts file not found" }}
$orig = [IO.File]::ReadAllText($hosts)
$begin = '{begin_marker}'
$end = '{end_marker}'
# strip previous block
$si = $orig.IndexOf($begin)
$ei = $orig.IndexOf($end)
if ($si -ge 0 -and $ei -gt $si) {{
    $tail = $orig.Substring($ei + $end.Length)
    $head = $orig.Substring(0, $si)
    $orig = $head + $tail
}}
$new = $orig.TrimEnd() + "`r`n`r`n" + $block + "`r`n"
[IO.File]::WriteAllText($hosts, $new, [Text.UTF8Encoding]::new($false))
Zapret-Log ("applied, entries block length: " + $block.Length)
# flush DNS cache
& ipconfig /flushdns 2>&1 | ForEach-Object {{ Zapret-Log $_ }}
"#,
        b64 = block_b64,
        begin_marker = HOSTS_BEGIN,
        end_marker = HOSTS_END,
    );

    elev::run_elevated_ps(&script).await
}

pub async fn remove() -> elev::ElevatedResult {
    let script = format!(
        r#"
$hosts = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
if (-not (Test-Path -LiteralPath $hosts)) {{ Zapret-Log "no hosts"; exit 0 }}
$orig = [IO.File]::ReadAllText($hosts)
$begin = '{begin_marker}'
$end = '{end_marker}'
$si = $orig.IndexOf($begin)
$ei = $orig.IndexOf($end)
if ($si -ge 0 -and $ei -gt $si) {{
    $head = $orig.Substring(0, $si)
    $tail = $orig.Substring($ei + $end.Length)
    $new = ($head + $tail).TrimEnd() + "`r`n"
    [IO.File]::WriteAllText($hosts, $new, [Text.UTF8Encoding]::new($false))
    Zapret-Log "removed"
    & ipconfig /flushdns 2>&1 | Out-Null
}} else {{
    Zapret-Log "block not found"
}}
"#,
        begin_marker = HOSTS_BEGIN,
        end_marker = HOSTS_END,
    );
    elev::run_elevated_ps(&script).await
}

pub async fn status() -> RobloxStatus {
    let hosts = std::path::Path::new(r"C:\Windows\System32\drivers\etc\hosts");
    let alt = std::env::var("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join("System32").join("drivers").join("etc").join("hosts"))
        .ok();
    let path = if hosts.exists() {
        Some(hosts.to_path_buf())
    } else {
        alt
    };

    if let Some(p) = path {
        if let Ok(txt) = std::fs::read_to_string(&p) {
            let applied = txt.contains(HOSTS_BEGIN);
            let mut updated = None;
            if applied {
                for line in txt.lines() {
                    if let Some(rest) = line.trim().strip_prefix("# updated:") {
                        updated = Some(rest.trim().to_string());
                        break;
                    }
                }
            }
            let entry_count = if applied {
                txt.lines()
                    .skip_while(|l| !l.contains(HOSTS_BEGIN))
                    .skip(1)
                    .take_while(|l| !l.contains(HOSTS_END))
                    .filter(|l| {
                        let l = l.trim();
                        !l.is_empty() && !l.starts_with('#')
                    })
                    .count()
            } else {
                0
            };
            return RobloxStatus {
                applied,
                entry_count,
                updated,
            };
        }
    }
    RobloxStatus {
        applied: false,
        entry_count: 0,
        updated: None,
    }
}

/// Minimal base64 without external crate (for embedding block into PS script).
fn base64_encode(data: &[u8]) -> String {
    const CH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(CH[((n >> 18) & 63) as usize] as char);
        out.push(CH[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            CH[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CH[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
