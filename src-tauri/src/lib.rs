//! The Crispy desktop app: a window, a tray icon, and a thin bridge between the UI and the engine.

mod commands;
mod tray;

use std::sync::Arc;
use std::time::Duration;

use crispy_core::config::{load_json, Paths, Settings};
use crispy_core::engine::{self, EngineHandle, EngineOptions};
use crispy_core::state::UiEvent;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tracing_subscriber::{layer::SubscriberExt, reload, util::SubscriberInitExt, EnvFilter, Registry};

pub const MAIN: &str = "main";

pub struct AppCtx {
    pub engine: EngineHandle,
    pub paths: Paths,
    pub log_filter: reload::Handle<EnvFilter, Registry>,
    pub window_effects: Mutex<bool>,
    /// Files handed to Crispy before the UI was ready to ask where to send them.
    pub pending_send: Mutex<Vec<String>>,
    pub quitting: Mutex<bool>,
}

fn init_logging(paths: &Paths, level: &str) -> (reload::Handle<EnvFilter, Registry>, tracing_appender::non_blocking::WorkerGuard) {
    let _ = std::fs::create_dir_all(paths.logs());
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("crispy")
        .filename_suffix("log")
        .max_log_files(7)
        .build(paths.logs())
        .unwrap_or_else(|_| tracing_appender::rolling::never(std::env::temp_dir(), "crispy.log"));
    let (writer, guard) = tracing_appender::non_blocking(file);
    let (filter, handle) = reload::Layer::new(filter_for(level));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(writer).with_ansi(false))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
    (handle, guard)
}

pub fn filter_for(level: &str) -> EnvFilter {
    let level = match level {
        "error" | "warn" | "info" | "debug" | "trace" => level,
        _ => "info",
    };
    EnvFilter::new(format!("warn,crispy={level},crispy_core={level},crispy_app={level}"))
}

/// Paths passed on the command line (Explorer's "Send to", or `crispy --send file...`).
fn paths_from_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut take = false;
    for a in args.iter().skip(1) {
        if a == "--send" {
            take = true;
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        if take || std::path::Path::new(a).exists() {
            out.push(a.clone());
        }
    }
    out
}

pub fn show_main(app: &AppHandle) {
    let window = match app.get_webview_window(MAIN) {
        Some(w) => w,
        None => match create_main_window(app) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("couldn't open the window: {e}");
                return;
            }
        },
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn deliver_send_request(app: &AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    show_main(app);
    let ctx = app.state::<AppCtx>();
    ctx.pending_send.lock().extend(paths);
    // Give a freshly opened window a moment to start listening.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        let paths: Vec<String> = std::mem::take(&mut *app.state::<AppCtx>().pending_send.lock());
        if !paths.is_empty() {
            let _ = app.emit("crispy://send-request", serde_json::json!({ "paths": paths }));
        }
    });
}

fn create_main_window(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let ctx = app.state::<AppCtx>();
    let settings: Settings = load_json(&ctx.paths.settings());
    let translucent = settings.general.translucent_window;

    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::default())
        .title("Crispy")
        .inner_size(1100.0, 740.0)
        .min_inner_size(880.0, 600.0)
        .center()
        .visible(false);

    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(18.0, 22.0))
        .transparent(translucent);

    #[cfg(target_os = "windows")]
    let builder = builder.decorations(false).shadow(true).transparent(translucent);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let builder = builder.transparent(false);

    let window = builder.build()?;

    let effects = translucent && apply_window_material(&window);
    *ctx.window_effects.lock() = effects;

    let win = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            let app = win.app_handle();
            let ctx = app.state::<AppCtx>();
            if *ctx.quitting.lock() {
                return;
            }
            let settings: Settings = load_json(&ctx.paths.settings());
            if settings.general.close_to_tray {
                api.prevent_close();
                let _ = win.hide();
            } else {
                *ctx.quitting.lock() = true;
                let engine = ctx.engine.clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    engine.shutdown().await;
                    app.exit(0);
                });
                api.prevent_close();
            }
        }
    });
    Ok(window)
}

/// Let the desktop show through the window: vibrancy on macOS, Mica on Windows 11.
/// Returns false where unsupported (Windows 10, Linux); the UI then stays opaque.
fn apply_window_material(window: &tauri::WebviewWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, Some(NSVisualEffectState::FollowsWindowActiveState), None).is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        window_vibrancy::apply_mica(window, None).is_ok()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = window;
        false
    }
}

/// Forward engine events to the UI, the tray and the OS.
fn pump_events(app: AppHandle, mut rx: tokio::sync::mpsc::UnboundedReceiver<UiEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                UiEvent::State(s) => {
                    tray::update(&app, &s);
                    let _ = app.emit("crispy://state", &*s);
                }
                UiEvent::Pointer(p) => {
                    let _ = app.emit("crispy://pointer", &p);
                }
                UiEvent::Toast(t) => {
                    let _ = app.emit("crispy://toast", &t);
                }
                UiEvent::Attention => show_main(&app),
                UiEvent::Notify { title, body } => {
                    let focused = app.get_webview_window(MAIN).and_then(|w| w.is_focused().ok()).unwrap_or(false);
                    if !focused {
                        let _ = app.notification().builder().title(title).body(body).show();
                    }
                }
                UiEvent::Reveal(path) => {
                    let _ = app.opener().reveal_item_in_dir(&path);
                }
                UiEvent::FocusChanged(peer) => tray::set_focus(&app, peer.as_deref()),
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = Paths::default_location();
    let settings: Settings = load_json(&paths.settings());
    let (log_filter, log_guard) = init_logging(&paths, &settings.advanced.log_level);
    let log_guard = Arc::new(log_guard);

    // Whatever happens, never leave the system cursor hidden or keys held down.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        crispy_core::platform::emergency_restore();
        tracing::error!("crashed: {info}");
        default_hook(info);
    }));

    let args: Vec<String> = std::env::args().collect();
    let start_hidden = args.iter().any(|a| a == "--minimized") || settings.general.start_minimized;
    let initial_send = paths_from_args(&args);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let paths = paths_from_args(&argv);
            if paths.is_empty() {
                show_main(app);
            } else {
                deliver_send_request(app, paths);
            }
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_settings,
            commands::set_settings,
            commands::get_app_info,
            commands::key_table,
            commands::pair_start,
            commands::pair_submit,
            commands::pair_cancel,
            commands::unpair,
            commands::set_peer_enabled,
            commands::set_layout,
            commands::send_files,
            commands::cancel_transfer,
            commands::answer_transfer,
            commands::clear_transfers,
            commands::reveal_path,
            commands::open_save_dir,
            commands::toggle_lock,
            commands::toggle_pause,
            commands::request_permissions,
            commands::open_permission_settings,
            commands::connect_address,
            commands::regenerate_identity,
            commands::open_logs,
            commands::quit_app,
            commands::take_send_request,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let (ui_tx, ui_rx) = tokio::sync::mpsc::unbounded_channel();
            let engine = tauri::async_runtime::block_on(engine::start(
                EngineOptions {
                    paths: paths.clone(),
                    platform: crispy_core::platform::native(),
                    clipboard: crispy_core::clipboard::native(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    port_override: None,
                    discovery: true,
                },
                ui_tx,
            ))?;
            app.manage(AppCtx {
                engine,
                paths: paths.clone(),
                log_filter: log_filter.clone(),
                window_effects: Mutex::new(false),
                pending_send: Mutex::new(initial_send.clone()),
                quitting: Mutex::new(false),
            });
            app.manage(log_guard.clone());
            commands::sync_autostart(&handle, settings.general.launch_at_login);
            tray::create(&handle)?;
            pump_events(handle.clone(), ui_rx);

            let window = create_main_window(&handle)?;
            if !start_hidden || !initial_send.is_empty() || !settings.general.onboarded {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Crispy");

    app.run(|app, event| match event {
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main(app),
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        RunEvent::Opened { urls } => {
            let paths: Vec<String> = urls
                .into_iter()
                .filter_map(|u| u.to_file_path().ok())
                .map(|p: std::path::PathBuf| p.to_string_lossy().into_owned())
                .collect();
            deliver_send_request(app, paths);
        }
        RunEvent::ExitRequested { api, code, .. } => {
            // Closing the last window shouldn't quit while Crispy lives in the tray.
            let ctx = app.state::<AppCtx>();
            if code.is_none() && !*ctx.quitting.lock() {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            crispy_core::platform::emergency_restore();
        }
        _ => {}
    });
}
