mod crypto;
mod models;
mod nebula;
mod network;
mod state;

use state::AppState;
use tauri::Manager;
use tauri::Emitter;
use tauri::webview::PageLoadEvent;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_log::{Target, TargetKind};

#[tauri::command]
fn set_close_behavior(state: tauri::State<AppState>, behavior: String) -> Result<(), String> {
    *state.close_behavior.lock().map_err(|e| e.to_string())? = behavior;
    Ok(())
}

#[tauri::command]
fn get_close_behavior(state: tauri::State<AppState>) -> Result<String, String> {
    state.close_behavior.lock().map_err(|e| e.to_string()).map(|b| b.clone())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_window(app: tauri::AppHandle, view: String, title: String, width: f64, height: f64) -> Result<(), String> {
    let label = format!("sub_{}", view);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.set_focus();
        return Ok(());
    }
    let url = format!("http://localhost:1421/index.html?view={}", view);
    log::info!("Opening sub-window: label={}, url={}", label, url);
    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().map_err(|e| format!("URL parse: {}", e))?))
        .title(title)
        .inner_size(width, height)
        .center()
        .resizable(true)
        .build()
        .map_err(|e| format!("Window build: {}", e))?;
    log::info!("Window opened: {}", label);
    Ok(())
}

fn external_navigation_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1")
            );
            let is_internal = url.scheme() == "tauri" || is_internal_host;
            if is_internal { return true; }
            let is_external = matches!(url.scheme(), "http" | "https" | "mailto" | "tel");
            if is_external {
                log::info!("opening external link: {}", url);
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }
            true
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(external_navigation_plugin())
        .setup(|app| {
            app.manage(AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crypto::generate_keypair,
            nebula::join_network,
            nebula::disconnect_network,
            nebula::get_status,
            nebula::get_network_stats,
            network::save_server,
            network::get_servers,
            network::delete_server,
            open_window,
            set_close_behavior,
            get_close_behavior,
            quit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let state = window.state::<AppState>();
                    let behavior = state.close_behavior.lock().unwrap().clone();
                    if behavior == "close" {
                        let _ = window.emit("confirm-close", ());
                        api.prevent_close();
                    } else {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                }
            }
        })
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), PageLoadEvent::Finished) {
                log::info!("main webview loaded");
                let _ = webview.window().show();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
