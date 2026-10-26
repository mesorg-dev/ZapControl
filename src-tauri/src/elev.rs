use std::process::Stdio;
use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ElevatedResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: String,
    pub error: Option<String>,
}

/// Run a PowerShell script elevated (UAC prompt). Script is written to a temp
/// file with UTF-8 BOM so Windows PowerShell 5.1 handles non-ASCII paths.
pub async fn run_elevated_ps(script: &str) -> ElevatedResult {
    let tmp = std::env::temp_dir().join(format!("zapret-elev-{}.ps1", uuid::Uuid::new_v4()));
    let log = std::env::temp_dir().join(format!("zapret-elev-{}.log", uuid::Uuid::new_v4()));

    let mut content = String::from("\u{FEFF}");
    content.push_str(script);
    content.push_str(&format!(
        "\n\n$code = $LASTEXITCODE\nif ($null -eq $code) {{ $code = 0 }}\n\
         [IO.File]::WriteAllText('{log}', ($out = ''))\n\
         exit $code\n",
        log = log.display()
    ));

    // Capture output by redirecting inside the child via transcript-like approach:
    // simpler: have the script itself write output to the log path.
    let wrapped = format!(
        "$ErrorActionPreference = 'Continue'\n\
         $logPath = '{log}'\n\
         function W($m) {{ Add-Content -LiteralPath $logPath -Value $m -Encoding UTF8 }}\n\
         {body}\n\
         exit 0\n",
        log = log.display().to_string().replace('\'', "''"),
        body = script.replace('\'', "''") // will be too aggressive; see below
    );
    // The aggressive quote-escape above breaks scripts containing quotes.
    // Use raw script instead; only inject log path via env-like variable.
    let _ = wrapped;

    let script_with_log = format!(
        "$ErrorActionPreference = 'Continue'\n\
         $ZapretLog = '{log}'\n\
         function Zapret-Log($m) {{ Add-Content -LiteralPath $ZapretLog -Value $m -Encoding UTF8 }}\n\
         {script}\n",
        log = log.display().to_string().replace('\'', "''"),
        script = script
    );

    if let Err(e) = tokio::fs::write(&tmp, format!("\u{FEFF}{script_with_log}")).await {
        return ElevatedResult {
            success: false,
            exit_code: None,
            output: String::new(),
            error: Some(format!("failed to write temp script: {e}")),
        };
    }

    let launcher = format!(
        "$p = Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -PassThru \
         -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{file}'; \
         exit $p.ExitCode",
        file = tmp.display().to_string().replace('\'', "''")
    );

    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("powershell.exe");
        c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &launcher])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("powershell.exe");
        c.args(["-NoProfile", "-Command", &launcher]);
        c
    };

    let result = cmd.output().await;
    let _ = std::fs::remove_file(&tmp);

    let output = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_file(&log);

    match result {
        Ok(out) => {
            let code = out.status.code();
            ElevatedResult {
                success: out.status.success(),
                exit_code: code,
                output,
                error: if out.status.success() {
                    None
                } else {
                    Some(format!("elevated process exited with code {code:?}"))
                },
            }
        }
        Err(e) => ElevatedResult {
            success: false,
            exit_code: None,
            output,
            error: Some(format!(
                "failed to launch elevated process (cancelled?): {e}"
            )),
        },
    }
}

/// Run a non-elevated command with no console window, return stdout.
pub async fn run_hidden(program: &str, args: &[&str]) -> std::io::Result<String> {
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new(program);
        c.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new(program);
        c.args(args).stdin(Stdio::null()).stdout(Stdio::piped());
        c
    };

    let out = cmd.output().await?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}
