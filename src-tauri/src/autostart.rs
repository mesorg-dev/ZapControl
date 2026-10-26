use crate::config::AutostartMode;

#[cfg(windows)]
mod win {
    use super::AutostartMode;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "zapret-gui";

    pub fn set_tray_autostart(enabled: bool) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .create_subkey(RUN_KEY)
            .map_err(|e| format!("registry: {e}"))?
            .0;

        if enabled {
            let exe = std::env::current_exe().map_err(|e| format!("exe path: {e}"))?;
            let cmd = format!("\"{}\" --minimized", exe.display());
            key.set_value(VALUE_NAME, &cmd)
                .map_err(|e| format!("set_value: {e}"))?;
        } else {
            let _ = key.delete_value(VALUE_NAME);
        }
        Ok(())
    }

    pub fn tray_autostart_enabled() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey(RUN_KEY)
            .ok()
            .and_then(|k| k.get_value::<String, _>(VALUE_NAME).ok())
            .is_some()
    }

    pub fn apply(mode: AutostartMode, enabled: bool) -> Result<(), String> {
        match mode {
            AutostartMode::Tray => set_tray_autostart(enabled),
            _ => set_tray_autostart(false),
        }
    }
}

#[cfg(not(windows))]
mod win {
    use super::AutostartMode;
    pub fn apply(_mode: AutostartMode, _enabled: bool) -> Result<(), String> {
        Err("autostart доступен только на Windows".into())
    }
    pub fn tray_autostart_enabled() -> bool {
        false
    }
}

#[allow(unused_imports)]
pub use win::{apply, tray_autostart_enabled};
