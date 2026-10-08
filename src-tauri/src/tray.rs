//! The menu bar / notification area icon: shows where the keyboard and mouse are and offers
//! quick controls without opening the window.

use crispy_core::state::{AppState, FocusView, PeerStatus};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::AppCtx;

const TRAY_ID: &str = "crispy";

pub struct TrayItems {
    status: MenuItem<Wry>,
    lock: CheckMenuItem<Wry>,
    pause: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Crispy", true, None::<&str>)?;
    let lock = CheckMenuItem::with_id(app, "lock", "Lock Cursor to This Screen", true, false, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause Sharing", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Crispy", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &lock,
            &pause,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    // A monochrome template image on macOS (tinted by the menu bar); the full-colour chip elsewhere.
    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Crispy")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => crate::show_main(app),
            "lock" => app.state::<AppCtx>().engine.send(crispy_core::engine::Command::ToggleLock),
            "pause" => app.state::<AppCtx>().engine.send(crispy_core::engine::Command::TogglePause),
            "quit" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { crate::commands::quit(&app).await });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                if !cfg!(target_os = "macos") {
                    crate::show_main(tray.app_handle());
                }
            }
        })
        .build(app)?;

    app.manage(TrayItems { status, lock, pause });
    Ok(())
}

/// Keep the menu in step with the engine.
pub fn update(app: &AppHandle, s: &AppState) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let connected = s.peers.iter().filter(|p| p.paired && p.status == PeerStatus::Connected).count();
    let text = match &s.focus {
        FocusView::Remote { peer_id } => {
            let name = s.peers.iter().find(|p| p.id == *peer_id).map(|p| p.name.as_str()).unwrap_or("another computer");
            format!("Keyboard & mouse on {name}")
        }
        FocusView::Local if s.paused => "Sharing paused".to_string(),
        FocusView::Local => match s.peers.iter().find(|p| p.controlling_me) {
            Some(p) => format!("Controlled from {}", p.name),
            None if connected == 0 => "No computers connected".to_string(),
            None if connected == 1 => "Sharing with 1 computer".to_string(),
            None => format!("Sharing with {connected} computers"),
        },
    };
    let _ = items.status.set_text(text);
    let _ = items.lock.set_checked(s.locked);
    let _ = items.pause.set_checked(s.paused);
}

pub fn set_focus(app: &AppHandle, peer: Option<&str>) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tip = match peer {
            Some(name) => format!("Crispy — keyboard & mouse on {name}"),
            None => "Crispy".to_string(),
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}
