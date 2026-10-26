use crate::paths;
use crate::strategy::{render_args, RenderCtx, Strategy};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestTarget {
    pub name: String,
    pub url: Option<String>,
    pub ping: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCheck {
    pub label: String, // HTTP | TLS1.2 | TLS1.3
    pub status: String, // OK | ERR | UNSUP | SSL
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetResult {
    pub target: String,
    pub checks: Vec<TargetCheck>,
    pub ping: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyReport {
    pub strategy_id: String,
    pub strategy_name: String,
    pub ok_count: i32,
    pub err_count: i32,
    pub results: Vec<TargetResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSummary {
    pub best_strategy: Option<String>,
    pub reports: Vec<StrategyReport>,
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
        ping: Some(host_of(url).into()),
    }
}

fn ping_t(name: &str, ip: &str) -> TestTarget {
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

fn load_targets() -> Vec<TestTarget> {
    let p = paths::install_root().join("utils").join("targets.txt");
    if let Ok(txt) = std::fs::read_to_string(&p) {
        let mut out = Vec::new();
        for line in txt.lines() {
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
        if !out.is_empty() {
            return out;
        }
    }
    default_targets()
}

pub async fn run(
    app: AppHandle,
    strategies: Vec<Strategy>,
    cancel: Arc<AtomicBool>,
) -> TestSummary {
    let targets = load_targets();
    let mut reports = Vec::new();
    let mut best: Option<(String, i32)> = None;

    let total = strategies.len();
    for (idx, strat) in strategies.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let _ = app.emit(
            "test-progress",
            serde_json::json!({
                "phase": "strategy",
                "index": idx + 1,
                "total": total,
                "strategyId": strat.id,
                "strategyName": strat.name,
            }),
        );

        // stop previous instance
        let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;
        tokio::time::sleep(Duration::from_millis(250)).await;

        let ctx = RenderCtx::from_env();
        let args = render_args(&strat.args_template, &ctx);

        match spawn_winws(&args).await {
            Ok(()) => {}
            Err(e) => {
                let _ = app.emit(
                    "test-log",
                    format!("[{}] не удалось запустить: {e}", strat.name),
                );
                continue;
            }
        }

        // wait ready
        let mut ready = false;
        for _ in 0..25 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if crate::engine::winws_running().await {
                ready = true;
                break;
            }
        }
        if !ready {
            let _ = app.emit(
                "test-log",
                format!("[{}] winws не поднялся, пропуск", strat.name),
            );
            let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;
            continue;
        }

        tokio::time::sleep(Duration::from_millis(400)).await;

        let results = test_all_targets(&app, &targets, &cancel).await;
        let ok_count = results
            .iter()
            .filter(|r| r.checks.iter().any(|c| c.status == "OK"))
            .count() as i32;
        let err_count = results
            .iter()
            .flat_map(|r| r.checks.iter())
            .filter(|c| c.status == "ERR" || c.status == "SSL")
            .count() as i32;

        let _ = app.emit(
            "test-strategy-done",
            serde_json::json!({
                "strategyId": strat.id,
                "strategyName": strat.name,
                "okCount": ok_count,
                "errCount": err_count,
            }),
        );

        match &best {
            Some((_, bk)) if ok_count <= *bk => {}
            _ => best = Some((strat.id.clone(), ok_count)),
        }

        reports.push(StrategyReport {
            strategy_id: strat.id.clone(),
            strategy_name: strat.name.clone(),
            ok_count,
            err_count,
            results,
        });

        let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;

    TestSummary {
        best_strategy: best.map(|(id, _)| id),
        reports,
    }
}

async fn spawn_winws(args: &str) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::process::Command;

    let winws = paths::winws_exe();
    let bin = paths::bin_dir();

    let mut cmd = Command::new(&winws);
    cmd.current_dir(&bin)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.raw_arg(args.to_string());
    }
    #[cfg(not(windows))]
    {
        for a in args.split_whitespace() {
            cmd.arg(a);
        }
    }

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("spawn failed: {e}"))
}

async fn test_all_targets(
    app: &AppHandle,
    targets: &[TestTarget],
    cancel: &Arc<AtomicBool>,
) -> Vec<TargetResult> {
    let mut out = Vec::new();
    for target in targets {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let mut checks = Vec::new();
        if let Some(url) = &target.url {
            for (label, extra) in [
                ("HTTP", vec!["--http1.1"]),
                ("TLS1.2", vec!["--tlsv1.2", "--tls-max", "1.2"]),
                ("TLS1.3", vec!["--tlsv1.3", "--tls-max", "1.3"]),
            ] {
                let status = curl_head(url, &extra).await;
                checks.push(TargetCheck {
                    label: label.into(),
                    status,
                });
            }
        }

        let ping = match &target.ping {
            Some(host) => ping_ms(host).await,
            None => "n/a".into(),
        };

        let _ = app.emit(
            "test-target",
            serde_json::json!({
                "target": target.name,
                "checks": checks,
                "ping": ping,
            }),
        );

        out.push(TargetResult {
            target: target.name.clone(),
            checks,
            ping,
        });
    }
    out
}

async fn curl_head(url: &str, extra: &[&str]) -> String {
    let mut args: Vec<String> = vec![
        "-I".into(),
        "-s".into(),
        "-m".into(),
        "4".into(),
        "--connect-timeout".into(),
        "2".into(),
        "-o".into(),
        "NUL".into(),
        "-w".into(),
        "%{http_code}".into(),
    ];
    for e in extra {
        args.push(e.to_string());
    }
    args.push(url.to_string());

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match crate::elev::run_hidden("curl.exe", &arg_refs).await {
        Ok(out) => {
            let trimmed = out.trim();
            let code = trimmed
                .lines()
                .rev()
                .find(|l| l.trim().chars().all(|c| c.is_ascii_digit()) && l.trim().len() == 3)
                .unwrap_or("")
                .trim()
                .to_string();

            let low = out.to_lowercase();
            if low.contains("could not resolve")
                || low.contains("certificate")
                || low.contains("ssl")
                || low.contains("schannel")
            {
                return "SSL".into();
            }
            if low.contains("unsupported")
                || low.contains("not supported")
                || low.contains("unknown option")
            {
                return "UNSUP".into();
            }
            if code.starts_with('2') || code.starts_with('3') {
                "OK".into()
            } else if code == "000" || code.is_empty() {
                "ERR".into()
            } else {
                // got an HTTP code => TLS handshake worked, count as OK-ish
                "OK".into()
            }
        }
        Err(_) => "ERR".into(),
    }
}

async fn ping_ms(host: &str) -> String {
    match crate::elev::run_hidden("ping", &["-n", "1", "-w", "1000", host]).await {
        Ok(out) => {
            for line in out.lines() {
                let l = line.to_lowercase();
                if l.contains("time=") || l.contains("время=") {
                    // extract ms
                    if let Some(pos) = l.find("ms") {
                        let before = &l[..pos];
                        if let Some(start) = before.rfind(|c: char| c.is_ascii_digit() || c == '.') {
                            let mut s = start;
                            while s > 0 && (before.as_bytes()[s - 1].is_ascii_digit() || before.as_bytes()[s - 1] == b'.') {
                                s -= 1;
                            }
                            return format!("{} ms", &before[s..pos].trim_end_matches('='));
                        }
                    }
                    return "ok".into();
                }
            }
            "timeout".into()
        }
        Err(_) => "timeout".into(),
    }
}
