mod autostart;
mod commands;
mod config;
mod elev;
mod engine;
mod paths;
mod roblox;
mod strategy;
mod tester;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let state = commands::AppState::load(&handle);
            app.manage(state);

            // tray: menu + double-click to show (icon from bundle) — desktop only
            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;

                if let Some(icon) = app.default_window_icon().cloned() {
                    let show = MenuItem::with_id(app, "show", "Открыть", true, None::<&str>)?;
                    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
                    let menu = Menu::with_items(app, &[&show, &quit])?;

                    let _ = TrayIconBuilder::with_id("main-tray")
                        .icon(icon)
                        .tooltip("zapret")
                        .menu(&menu)
                        .show_menu_on_left_click(false)
                        .on_menu_event(|app, event| match event.id.as_ref() {
                            "show" => {
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                            "quit" => {
                                let _ = std::process::Command::new("taskkill")
                                    .args(["/IM", "winws.exe", "/F"])
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .status();
                                app.exit(0);
                            }
                            _ => {}
                        })
                        .on_tray_icon_event(|tray, event| {
                            if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                                let app = tray.app_handle();
                                if let Some(win) = app.get_webview_window("main") {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        })
                        .build(app);
                }
            }

            // autostart bypass on launch if enabled
            {
                let state = app.state::<commands::AppState>();
                let s = state.settings.read().unwrap().clone();
                if s.auto_start_bypass && !s.last_strategy.is_empty() {
                    let handle2 = handle.clone();
                    let strat = s.last_strategy.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        let st = handle2.state::<commands::AppState>();
                        let _ = commands::start_bypass_inner(&handle2, &st, &strat).await;
                    });
                }
            }

            if std::env::args().any(|a| a == "--minimized") {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_strategies,
            commands::get_settings,
            commands::save_settings,
            commands::get_status,
            commands::start_bypass,
            commands::stop_bypass,
            commands::install_service,
            commands::remove_service,
            commands::run_auto_test,
            commands::cancel_auto_test,
            commands::roblox_fetch_preview,
            commands::roblox_apply,
            commands::roblox_remove,
            commands::roblox_status,
            commands::set_ipset_mode,
            commands::get_ipset_mode,
            commands::open_logs_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
