use crate::elev::{self, ElevatedResult};
use crate::paths;
use crate::strategy::from_env;
use zapret_core::strategy::render_args;
use std::process::Stdio;
use tokio::process::{Child, Command};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Engine {
    child: tokio::sync::Mutex<Option<Child>>,
    active_strategy: tokio::sync::Mutex<Option<String>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            child: tokio::sync::Mutex::new(None),
            active_strategy: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn is_running(&self) -> bool {
        winws_running().await
    }

    pub async fn active_strategy(&self) -> Option<String> {
        self.active_strategy.lock().await.clone()
    }

    pub async fn start(&self, strategy_id: &str, template: &str) -> Result<(), String> {
        self.stop().await?;

        if !paths::winws_exe().exists() {
            return Err(format!(
                "winws.exe не найден: {} — скопируй bin/ из zapret-discord-youtube рядом с приложением",
                paths::winws_exe().display()
            ));
        }

        let ctx = from_env();
        let args = render_args(template, &ctx);

        let bin = paths::bin_dir();
        let winws = paths::winws_exe();

        let mut cmd = Command::new(&winws);
        cmd.current_dir(&bin)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
            // exact command line control: append raw args without re-quoting
            cmd.raw_arg(args);
        }
        #[cfg(not(windows))]
        {
            for a in args.split_whitespace() {
                cmd.arg(a);
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("не удалось запустить winws.exe: {e}"))?;

        // give winws a moment; verify it stayed alive
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
        if !winws_running().await {
            return Err(
                "winws.exe завершился сразу после запуска — проверь права (нужен админ) и конфликты"
                    .into(),
            );
        }

        *self.child.lock().await = Some(child);
        *self.active_strategy.lock().await = Some(strategy_id.to_string());

        // TCP timestamps (best-effort, same as Flowseal)
        let _ = elev::run_hidden(
            "netsh",
            &["interface", "tcp", "set", "global", "timestamps=enabled"],
        )
        .await;

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        {
            let mut guard = self.child.lock().await;
            if let Some(mut ch) = guard.take() {
                let _ = ch.kill().await;
            }
        }
        // also kill any stray winws (e.g. after service tests)
        let _ = elev::run_hidden("taskkill", &["/IM", "winws.exe", "/F"]).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        *self.active_strategy.lock().await = None;
        Ok(())
    }

    pub async fn install_service(&self, _strategy_id: &str, template: &str) -> ElevatedResult {
        let ctx = from_env();
        let args = render_args(template, &ctx);
        let winws = paths::winws_exe();
        let winws_s = winws.to_string_lossy().replace('\'', "''");

        // stop existing first (non-elevated attempt)
        let _ = elev::run_hidden("sc", &["stop", "zapret"]).await;

        let script = format!(
            "$winws = '{winws}'\n\
             $args = '{args}'\n\
             $binPath = '\"' + $winws + '\" ' + $args\n\
             Zapret-Log ('binPath length: ' + $binPath.Length)\n\
             & sc.exe stop zapret 2>$null | Out-Null\n\
             & sc.exe delete zapret 2>$null | Out-Null\n\
             Start-Sleep -Milliseconds 400\n\
             $r = & sc.exe create zapret binPath= $binPath start= auto DisplayName= zapret 2>&1\n\
             Zapret-Log ($r -join \"`n\")\n\
             if ($LASTEXITCODE -ne 0) {{\n\
             \x20   $r2 = New-Service -Name zapret -BinaryPathName $binPath -DisplayName 'zapret' -StartupType Automatic -ErrorAction SilentlyContinue 2>&1\n\
             \x20   Zapret-Log ($r2 | Out-String)\n\
             }}\n\
             & sc.exe description zapret 'Zapret DPI bypass (zapret-gui)' 2>$null | Out-Null\n\
             & sc.exe start zapret 2>&1 | ForEach-Object {{ Zapret-Log $_ }}\n\
             Start-Sleep -Milliseconds 500\n\
             $st = (& sc.exe query zapret | Out-String)\n\
             Zapret-Log $st\n",
            winws = winws_s,
            args = args.replace('\'', "''"),
        );

        elev::run_elevated_ps(&script).await
    }

    pub async fn remove_service(&self) -> ElevatedResult {
        let script = r#"
& sc.exe stop zapret 2>&1 | ForEach-Object { Zapret-Log $_ }
& sc.exe delete zapret 2>&1 | ForEach-Object { Zapret-Log $_ }
& taskkill /IM winws.exe /F 2>&1 | ForEach-Object { Zapret-Log $_ }
& sc.exe stop WinDivert 2>$null | Out-Null
& sc.exe delete WinDivert 2>$null | Out-Null
& sc.exe stop WinDivert14 2>$null | Out-Null
& sc.exe delete WinDivert14 2>$null | Out-Null
Zapret-Log "done"
"#
        .to_string();
        elev::run_elevated_ps(&script).await
    }
}

pub async fn winws_running() -> bool {
    match elev::run_hidden("tasklist", &["/FI", "IMAGENAME eq winws.exe", "/NH"]).await {
        Ok(out) => out.to_lowercase().contains("winws.exe"),
        Err(_) => false,
    }
}

pub async fn service_installed() -> bool {
    match elev::run_hidden("sc", &["query", "zapret"]).await {
        Ok(out) => out.contains("SERVICE_NAME") || out.contains("Имя службы") || out.to_lowercase().contains("zapret"),
        Err(_) => false,
    }
}
