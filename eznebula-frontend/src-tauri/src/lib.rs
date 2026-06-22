mod crypto;
mod models;
mod nebula;
mod network;
mod settings;
mod state;

use state::AppState;
use network::ServerManager;
use tauri::{Manager, Emitter};
use tauri::webview::PageLoadEvent;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::image::Image;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_log::{Target, TargetKind};

/// 从 PNG 字节创建 Tauri Image（用于托盘图标）
fn png_to_image(bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(bytes).expect("failed to parse icon");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Image::new_owned(rgba.into_raw(), w, h)
}

include!("../icons/single_icon.rs");

#[tauri::command]
fn set_close_behavior(state: tauri::State<AppState>, behavior: String) -> Result<(), String> {
    *state.close_behavior.lock().map_err(|e| e.to_string())? = behavior.clone();
    settings::set_close_behavior(behavior);
    Ok(())
}

#[tauri::command]
fn get_close_behavior() -> Result<String, String> {
    Ok(settings::get_close_behavior())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    log::info!("quit_app called, cleaning up before exit");

    // 杀掉 nebula 进程
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "nebula.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .arg("nebula")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // 关闭所有子窗口
    for (label, w) in app.webview_windows().iter() {
        if label.starts_with("sub_") {
            let _ = w.close();
        }
    }
    app.exit(0);
}

#[tauri::command]
fn open_window(app: tauri::AppHandle, view: String, title: String, width: f64, height: f64) -> Result<(), String> {
    let label = format!("sub_{}", view);

    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.set_focus();
        return Ok(());
    }

    let url = format!("index.html?view={}", view);
    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App(url.into()))
        .title(&title)
        .inner_size(width, height)
        .center()
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| format!("Failed to create window: {:?}", e))?;

    Ok(())
}

fn external_navigation_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(url.host_str(), Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1"));
            if url.scheme() == "tauri" || is_internal_host { return true; }
            if matches!(url.scheme(), "http" | "https" | "mailto" | "tel") {
                log::info!("external link: {}", url);
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }
            true
        }).build()
}

/// 检查是否已有实例在运行（Windows 互斥体方式）
#[cfg(windows)]
fn check_single_instance() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(lpMutexAttributes: *const u8, bInitialOwner: i32, lpName: *const u16) -> isize;
        fn GetLastError() -> u32;
    }

    let name: Vec<u16> = OsStr::new("Global\\EZNebula_SingleInstance")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());

        if handle == 0 {
            return false;
        }

        // ERROR_ALREADY_EXISTS = 183
        GetLastError() != 183
    }
}

#[cfg(not(windows))]
fn check_single_instance() -> bool { true }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
            Target::new(TargetKind::Webview),
        ]).build())
        .plugin(tauri_plugin_opener::init())
        .plugin(external_navigation_plugin())
        .setup(|app| {
            // ── 单实例检查 ──
            if !check_single_instance() {
                log::warn!("Another instance is already running, exiting");
                // 尝试聚焦已有窗口后退出
                std::process::exit(0);
            }

            // 初始化设置和服务器管理器（settings::init 必须在 get_close_behavior 之前）
            let app_data_dir = app.path().app_data_dir().expect("failed to resolve app_data_dir");
            settings::init(app_data_dir.clone());
            app.manage(ServerManager::new(app_data_dir));

            // 初始化应用状态
            let mut s = AppState::new();
            s.close_behavior = std::sync::Arc::new(std::sync::Mutex::new(settings::get_close_behavior()));
            app.manage(s);

            // 设置主窗口图标
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_icon(png_to_image(ICON_PNG_BYTES));
            }

            // ── 系统托盘 ──
            let show_hide = MenuItemBuilder::with_id("show_hide", "显示/隐藏").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app).item(&show_hide).separator().item(&quit).build()?;

            let tray_icon = png_to_image(ICON_PNG_BYTES);

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon)
                .tooltip("EZNebula - 未连接")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show_hide" => {
                            if let Some(w) = app.get_webview_window("main") {
                                if w.is_visible().unwrap_or(false) {
                                    let _ = w.hide();
                                } else {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                        "quit" => {
                            // 清理 nebula 进程
                            #[cfg(windows)]
                            {
                                let _ = std::process::Command::new("taskkill")
                                    .args(["/F", "/IM", "nebula.exe"])
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .status();
                            }
                            // 关闭所有子窗口
                            for (label, w) in app.webview_windows().iter() {
                                if label.starts_with("sub_") {
                                    let _ = w.close();
                                }
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crypto::generate_keypair,
            nebula::join_network,
            nebula::disconnect_network,
            nebula::get_status,
            nebula::get_network_stats,
            nebula::get_peers,
            network::save_server,
            network::get_servers,
            network::delete_server,
            network::measure_server_rtt,
            open_window,
            set_close_behavior,
            get_close_behavior,
            quit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label().to_string();
                if label == "main" {
                    let behavior = window.state::<AppState>().close_behavior.lock().unwrap().clone();
                    if behavior == "close" {
                        // 关闭所有子窗口
                        let app = window.app_handle();
                        for (sub_label, w) in app.webview_windows().iter() {
                            if sub_label.starts_with("sub_") {
                                let _ = w.close();
                            }
                        }
                        let _ = window.emit("confirm-close", ());
                    } else {
                        let _ = window.hide();
                    }
                    api.prevent_close();
                }
            } else if let tauri::WindowEvent::Destroyed = event {
                // 子窗口销毁时清理（无需特别操作）
            }
        })
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = webview.window().show();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
