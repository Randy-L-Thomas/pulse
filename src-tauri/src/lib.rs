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
    cfg: Config,
    probes: ProbeState,
    client: reqwest::Client,
    snapshot: Mutex<Option<Snapshot>>,
}

#[tauri::command]
fn get_snapshot(state: tauri::State<'_, Arc<AppState>>) -> Snapshot {
    state.snapshot.lock().unwrap().clone().unwrap_or_else(probes::empty_snapshot)
}

#[tauri::command]
fn run_action(
    state: tauri::State<'_, Arc<AppState>>,
    cell_id: String,
    action: String,
) -> Result<String, String> {
    launch::run_action(&state.cfg, &cell_id, &action)
}

#[tauri::command]
fn set_width_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> Result<(), String> {
    let win = app.get_webview_window("main").ok_or("no window")?;
    let m = match mode.as_str() {
        "full" => WidthMode::Full,
        _ => WidthMode::Half,
    };
    window::dock_and_size(&win, &state.cfg, m)
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
        cfg,
        probes: ProbeState::new(),
        client,
        snapshot: Mutex::new(None),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .setup(move |app| {
            let win = app.get_webview_window("main").expect("main window");
            let _ = window::dock_and_size(&win, &state.cfg, WidthMode::Half);
            let handle = app.handle().clone();
            let loop_state = state.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let snap = probes::collect(&loop_state.cfg, &loop_state.probes, &loop_state.client).await;
                    *loop_state.snapshot.lock().unwrap() = Some(snap.clone());
                    let _ = handle.emit("snapshot", snap);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_snapshot, run_action, set_width_mode])
        .run(tauri::generate_context!())
        .expect("error while running pulse");
}
