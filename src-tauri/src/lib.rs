mod config;
mod launch;
mod probes;
mod window;

use config::Config;
use probes::{ProbeState, Snapshot};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};
use window::WidthMode;

struct AppState {
    cfg: Mutex<Config>,
    probes: ProbeState,
    client: reqwest::Client,
    snapshot: Mutex<Option<Snapshot>>,
}

#[tauri::command]
fn get_snapshot(state: tauri::State<'_, Arc<AppState>>) -> Snapshot {
    state
        .snapshot
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(probes::empty_snapshot)
}

#[tauri::command]
fn run_action(
    state: tauri::State<'_, Arc<AppState>>,
    cell_id: String,
    action: String,
) -> Result<String, String> {
    let cfg = state.cfg.lock().unwrap().clone();
    launch::run_action(&cfg, &cell_id, &action)
}

#[tauri::command]
fn set_width_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> Result<(), String> {
    let win = app.get_webview_window("main").ok_or("no window")?;
    let cfg = state.cfg.lock().unwrap().clone();
    let m = match mode.as_str() {
        "full" => WidthMode::Full,
        _ => WidthMode::Half,
    };
    window::dock_and_size(&win, &cfg, m)
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Arc<AppState>>) -> Config {
    state.cfg.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    cfg: Config,
) -> Result<String, String> {
    config::save_config(&cfg)?;
    *state.cfg.lock().unwrap() = cfg.clone();
    if let Some(win) = app.get_webview_window("main") {
        let _ = window::dock_and_size(&win, &cfg, WidthMode::Half);
    }
    Ok(format!("saved {}", config::user_config_path().display()))
}

#[tauri::command]
fn apply_preset(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
) -> Result<String, String> {
    let cfg = config::apply_preset(&name)?;
    *state.cfg.lock().unwrap() = cfg.clone();
    if let Some(win) = app.get_webview_window("main") {
        let _ = window::dock_and_size(&win, &cfg, WidthMode::Half);
    }
    Ok(format!("preset {name}"))
}

#[tauri::command]
fn app_meta() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "config_path": config::user_config_path().display().to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config::load_config().unwrap_or_else(|e| {
        eprintln!("pulse: {e}");
        std::process::exit(1);
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(2))
        .no_proxy()
        .build()
        .expect("http client");
    let state = Arc::new(AppState {
        cfg: Mutex::new(cfg),
        probes: ProbeState::new(),
        client,
        snapshot: Mutex::new(None),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.set_focus();
                let _ = win.set_always_on_top(true);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .manage(state.clone())
        .setup(move |app| {
            ensure_default_autostart(app.handle());
            let win = app.get_webview_window("main").expect("main window");
            let cfg = state.cfg.lock().unwrap().clone();
            let _ = window::dock_and_size(&win, &cfg, WidthMode::Half);
            let clamp_win = win.clone();
            let loop_state = state.clone();
            win.on_window_event(move |ev| {
                if matches!(ev, tauri::WindowEvent::Resized(_)) {
                    let cfg = loop_state.cfg.lock().unwrap().clone();
                    window::clamp_resize(&clamp_win, &cfg);
                }
            });
            let handle = app.handle().clone();
            let probe_state = state.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let cfg = probe_state.cfg.lock().unwrap().clone();
                    let snap = probes::collect(&cfg, &probe_state.probes, &probe_state.client).await;
                    *probe_state.snapshot.lock().unwrap() = Some(snap.clone());
                    let _ = handle.emit("snapshot", snap);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            run_action,
            set_width_mode,
            get_settings,
            save_settings,
            apply_preset,
            app_meta
        ])
        .run(tauri::generate_context!())
        .expect("error while running pulse");
}

fn ensure_default_autostart(app: &tauri::AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    let marker = config::autostart_inited_path();
    if marker.is_file() {
        return;
    }
    if let Some(dir) = marker.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = app.autolaunch().enable();
    let _ = std::fs::write(&marker, b"1\n");
}
