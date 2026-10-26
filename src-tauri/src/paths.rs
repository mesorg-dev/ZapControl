use std::path::PathBuf;

/// Resolve install root: directory containing the executable (portable layout).
/// In dev, fall back to workspace root (parent of src-tauri).
pub fn install_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // dev target/debug layout: go up to workspace
            let name = exe
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "zapret-gui" || name == "zapret_gui" {
                if dir.ends_with("debug") || dir.ends_with("release") {
                    if let Some(ws) = dir.parent().and_then(|p| p.parent()) {
                        return ws.to_path_buf();
                    }
                }
            }
            return dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

pub fn bin_dir() -> PathBuf {
    install_root().join("bin")
}

pub fn lists_dir() -> PathBuf {
    install_root().join("lists")
}

pub fn strategies_dir() -> PathBuf {
    install_root().join("strategies")
}

pub fn winws_exe() -> PathBuf {
    bin_dir().join("winws.exe")
}

pub fn ensure_runtime_layout() -> std::io::Result<()> {
    let lists = lists_dir();
    std::fs::create_dir_all(&lists)?;

    let user_files = [
        ("list-general-user.txt", "# Never leave this file empty\ndomain.example.abc\n"),
        ("list-exclude-user.txt", "domain.example.abc\n"),
        ("ipset-exclude-user.txt", "203.0.113.113/32\n"),
    ];
    for (name, content) in user_files {
        let p = lists.join(name);
        if !p.exists() {
            std::fs::write(p, content)?;
        }
    }
    let ipset = lists.join("ipset-all.txt");
    if !ipset.exists() {
        std::fs::write(&ipset, "")?;
    }
    Ok(())
}
