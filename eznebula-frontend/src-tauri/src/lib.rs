mod crypto;
mod models;
mod nebula;
mod network;
mod state;

use state::AppState;
use tauri::Manager;
use tauri::webview::PageLoadEvent;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_log::{Target, TargetKind};

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
        ])
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), PageLoadEvent::Finished) {
                log::info!("main webview loaded");
                let _ = webview.window().show();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
