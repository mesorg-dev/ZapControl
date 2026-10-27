//! Windows tester: winws spawn + curl probes, scoring from zapret-core.
use crate::paths;
use crate::strategy::{self, from_env};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use zapret_core::targets::{self, TestTarget};
use zapret_core::tester::{StrategyReport, TargetCheck, TargetResult};

pub use zapret_core::tester::TestSummary;

fn load_targets() -> Vec<TestTarget> {
    let p = paths::install_root().join("utils").join("targets.txt");
    targets::load_targets(&p)
}

pub async fn run(
    app: AppHandle,
    strategies: Vec<strategy::Strategy>,
    cancel: Arc<AtomicBool>,
) -> TestSummary {
    let targets = load_targets();
    let mut reports = Vec::new();

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

        let ctx = from_env();
        let args = zapret_core::strategy::render_args(&strat.args_template, &ctx);

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
        let report = StrategyReport::from_results(&strat.id, &strat.name, results);

        let _ = app.emit(
            "test-strategy-done",
            serde_json::json!({
                "strategyId": report.strategy_id,
                "strategyName": report.strategy_name,
                "okCount": report.ok_count,
                "errCount": report.err_count,
            }),
        );

        reports.push(report);

        let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let _ = crate::elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;

    zapret_core::tester::summarize(reports)
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
    targets_list: &[TestTarget],
    cancel: &Arc<AtomicBool>,
) -> Vec<TargetResult> {
    let mut out = Vec::new();
    for target in targets_list {
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
                    if let Some(pos) = l.find("ms") {
                        let before = &l[..pos];
                        if let Some(start) = before.rfind(|c: char| c.is_ascii_digit() || c == '.') {
                            let mut s = start;
                            while s > 0
                                && (before.as_bytes()[s - 1].is_ascii_digit()
                                    || before.as_bytes()[s - 1] == b'.')
                            {
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
