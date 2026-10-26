use crate::config::{self, AutostartMode, Settings, SharedSettings};
use crate::engine::{self, Engine};
use crate::roblox;
use crate::strategy::{self, Strategy};
use crate::tester;
use crate::{autostart, paths};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub settings: SharedSettings,
    pub engine: Engine,
    pub test_cancel: Arc<AtomicBool>,
    pub testing: Arc<AtomicBool>,
}

impl AppState {
    pub fn load(_app: &AppHandle) -> Self {
        let _ = paths::ensure_runtime_layout();
        Self {
            settings: Arc::new(RwLock::new(config::load())),
            engine: Engine::new(),
            test_cancel: Arc::new(AtomicBool::new(false)),
            testing: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn save_state(s: &Settings) {
    let _ = config::save(s);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    pub service_installed: bool,
    pub active_strategy: Option<String>,
    pub winws_path_ok: bool,
    pub tray_autostart: bool,
}

#[tauri::command]
pub fn get_strategies() -> Vec<Strategy> {
    strategy::load_all()
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let prev = state.settings.read().unwrap().clone();

    if settings.autostart_mode != prev.autostart_mode
        || (settings.autostart_mode == AutostartMode::Tray)
    {
        let _ = autostart::apply(settings.autostart_mode, settings.autostart_mode == AutostartMode::Tray)
            .map_err(|e| format!("автозапуск: {e}"))?;
    }

    *state.settings.write().unwrap() = settings.clone();
    save_state(&settings);
    Ok(settings)
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<Status, String> {
    Ok(Status {
        running: state.engine.is_running().await,
        service_installed: engine::service_installed().await,
        active_strategy: state.engine.active_strategy().await,
        winws_path_ok: paths::winws_exe().exists(),
        tray_autostart: autostart::tray_autostart_enabled(),
    })
}

pub async fn start_bypass_inner(
    app: &AppHandle,
    state: &AppState,
    strategy_id: &str,
) -> Result<(), String> {
    let strategies = strategy::load_all();
    let strat = strategies
        .iter()
        .find(|s| s.id == strategy_id)
        .ok_or_else(|| format!("стратегия не найдена: {strategy_id}"))?;

    state.engine.start(&strat.id, &strat.args_template).await?;

    {
        let mut s = state.settings.write().unwrap();
        s.last_strategy = strat.id.clone();
        save_state(&s);
    }

    let _ = app.emit(
        "status-changed",
        serde_json::json!({ "running": true, "strategy": strat.id }),
    );
    Ok(())
}

#[tauri::command]
pub async fn start_bypass(
    app: AppHandle,
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<(), String> {
    start_bypass_inner(&app, &state, &strategy_id).await
}

#[tauri::command]
pub async fn stop_bypass(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.stop().await?;
    let _ = app.emit(
        "status-changed",
        serde_json::json!({ "running": false }),
    );
    Ok(())
}

#[tauri::command]
pub async fn install_service(
    app: AppHandle,
    state: State<'_, AppState>,
    strategy_id: String,
) -> Result<String, String> {
    let strategies = strategy::load_all();
    let strat = strategies
        .iter()
        .find(|s| s.id == strategy_id)
        .ok_or_else(|| format!("стратегия не найдена: {strategy_id}"))?;

    let res = state.engine.install_service(&strat.id, &strat.args_template).await;
    let _ = app.emit(
        "log",
        format!("install_service: ok={} out={}", res.success, res.output),
    );

    if res.success {
        let mut s = state.settings.write().unwrap();
        s.last_strategy = strat.id.clone();
        s.autostart_mode = AutostartMode::Service;
        save_state(&s);
        Ok(res.output)
    } else {
        Err(res
            .error
            .unwrap_or_else(|| format!("установка службы не удалась: {}", res.output)))
    }
}

#[tauri::command]
pub async fn remove_service(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let res = state.engine.remove_service().await;
    let _ = app.emit(
        "log",
        format!("remove_service: ok={} out={}", res.success, res.output),
    );
    if res.success {
        let mut s = state.settings.write().unwrap();
        if s.autostart_mode == AutostartMode::Service {
            s.autostart_mode = AutostartMode::None;
            save_state(&s);
        }
        Ok(res.output)
    } else {
        Err(res.error.unwrap_or_else(|| "удаление службы не удалось".into()))
    }
}

#[tauri::command]
pub async fn run_auto_test(
    app: AppHandle,
    state: State<'_, AppState>,
    strategy_ids: Option<Vec<String>>,
) -> Result<tester::TestSummary, String> {
    if state.testing.swap(true, Ordering::SeqCst) {
        return Err("тест уже запущен".into());
    }
    state.test_cancel.store(false, Ordering::SeqCst);

    // service conflicts with direct winws spawn
    if engine::service_installed().await {
        state.testing.store(false, Ordering::SeqCst);
        return Err(
            "служба zapret установлена — удали её (кнопка «Служба»), иначе тесты конфликтуют"
                .into(),
        );
    }

    // stop current bypass during tests
    let _ = state.engine.stop().await;

    let mut strategies = strategy::load_all();
    if let Some(ids) = strategy_ids {
        if !ids.is_empty() {
            strategies.retain(|s| ids.contains(&s.id));
        }
    }
    if strategies.is_empty() {
        state.testing.store(false, Ordering::SeqCst);
        return Err("нет стратегий для теста".into());
    }

    let cancel = state.test_cancel.clone();
    let app2 = app.clone();
    let result = tester::run(app2, strategies, cancel).await;

    state.testing.store(false, Ordering::SeqCst);

    if let Some(best) = &result.best_strategy {
        let mut s = state.settings.write().unwrap();
        s.last_strategy = best.clone();
        save_state(&s);
    }

    let _ = app.emit("test-finished", &result);
    Ok(result)
}

#[tauri::command]
pub fn cancel_auto_test(state: State<AppState>) {
    state.test_cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn roblox_fetch_preview(state: State<'_, AppState>) -> Result<roblox::RobloxFile, String> {
    let url = state.settings.read().unwrap().roblox_json_url.clone();
    roblox::fetch(&url).await
}

#[tauri::command]
pub async fn roblox_apply(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let url = state.settings.read().unwrap().roblox_json_url.clone();
    let file = roblox::fetch(&url).await?;
    let res = roblox::apply(&file).await;
    let _ = app.emit(
        "log",
        format!("roblox_apply: ok={} out={}", res.success, res.output),
    );
    if res.success {
        let mut s = state.settings.write().unwrap();
        s.roblox_fix_enabled = true;
        s.roblox_last_update = file
            .updated
            .clone()
            .or_else(|| Some(chrono::Utc::now().to_rfc3339()));
        save_state(&s);
        Ok(res.output)
    } else {
        Err(res.error.unwrap_or_else(|| "применение hosts не удалось".into()))
    }
}

#[tauri::command]
pub async fn roblox_remove(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let res = roblox::remove().await;
    let _ = app.emit(
        "log",
        format!("roblox_remove: ok={} out={}", res.success, res.output),
    );
    if res.success {
        let mut s = state.settings.write().unwrap();
        s.roblox_fix_enabled = false;
        save_state(&s);
        Ok(res.output)
    } else {
        Err(res.error.unwrap_or_else(|| "удаление из hosts не удалось".into()))
    }
}

#[tauri::command]
pub async fn roblox_status() -> roblox::RobloxStatus {
    roblox::status().await
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IpsetMode {
    Loaded,
    None,
    Any,
}

#[tauri::command]
pub fn get_ipset_mode() -> IpsetMode {
    let p = paths::lists_dir().join("ipset-all.txt");
    let raw = std::fs::read_to_string(&p).unwrap_or_default();
    if raw.trim().is_empty() {
        IpsetMode::Any
    } else if raw.contains("203.0.113.113/32") {
        IpsetMode::None
    } else {
        IpsetMode::Loaded
    }
}

#[tauri::command]
pub fn set_ipset_mode(mode: String) -> Result<IpsetMode, String> {
    let p = paths::lists_dir().join("ipset-all.txt");
    let backup = paths::lists_dir().join("ipset-all.txt.backup");

    match mode.as_str() {
        "any" => {
            if p.exists() {
                let _ = std::fs::copy(&p, &backup);
            }
            std::fs::write(&p, "").map_err(|e| e.to_string())?;
            Ok(IpsetMode::Any)
        }
        "none" => {
            if p.exists() && !backup.exists() {
                let _ = std::fs::copy(&p, &backup);
            }
            std::fs::write(&p, "203.0.113.113/32\n").map_err(|e| e.to_string())?;
            Ok(IpsetMode::None)
        }
        "loaded" => {
            if backup.exists() {
                let _ = std::fs::rename(&backup, &p);
            } else if !p.exists() {
                std::fs::write(&p, "").map_err(|e| e.to_string())?;
            }
            Ok(IpsetMode::Loaded)
        }
        other => Err(format!("неизвестный режим: {other}")),
    }
}

#[tauri::command]
pub fn open_logs_dir() -> Result<String, String> {
    let p = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("zapret-gui");
    let _ = std::fs::create_dir_all(&p);
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(&p).spawn();
    }
    Ok(p.display().to_string())
}
