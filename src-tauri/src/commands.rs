//! Commands the UI can invoke. Each is a thin wrapper: the engine owns all behaviour.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crispy_core::config::Settings;
use crispy_core::engine::Command;
use crispy_core::keymap::{key_table as keys, KeyInfo};
use crispy_core::layout::Pos;
use crispy_core::state::AppState;
use crispy_core::types::Os;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::AppCtx;

type Res<T> = Result<T, String>;

fn gone() -> String {
    "Crispy's engine stopped. Please restart Crispy.".into()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    os: Os,
    window_effects: bool,
    save_dir: String,
    log_dir: String,
}

#[tauri::command]
pub async fn get_state(ctx: State<'_, AppCtx>) -> Res<AppState> {
    ctx.engine.state().await.ok_or_else(gone)
}

#[tauri::command]
pub async fn get_settings(ctx: State<'_, AppCtx>) -> Res<Settings> {
    ctx.engine.settings().await.ok_or_else(gone)
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, ctx: State<'_, AppCtx>, settings: Settings) -> Res<Settings> {
    let before = ctx.engine.settings().await.ok_or_else(gone)?;
    let after = ctx.engine.set_settings(settings).await.ok_or_else(gone)?;
    if before.general.launch_at_login != after.general.launch_at_login {
        sync_autostart(&app, after.general.launch_at_login);
    }
    if before.advanced.log_level != after.advanced.log_level {
        let _ = ctx.log_filter.modify(|f| *f = crate::filter_for(&after.advanced.log_level));
    }
    Ok(after)
}

pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let current = launcher.is_enabled().unwrap_or(false);
    let result = match (enabled, current) {
        (true, false) => launcher.enable(),
        (false, true) => launcher.disable(),
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::warn!("couldn't change launch at login: {e}");
    }
}

#[tauri::command]
pub async fn get_app_info(ctx: State<'_, AppCtx>) -> Res<AppInfo> {
    let info = ctx.engine.info().await.ok_or_else(gone)?;
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        os: Os::current(),
        window_effects: *ctx.window_effects.lock(),
        save_dir: info.save_dir.to_string_lossy().into_owned(),
        log_dir: info.log_dir.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn key_table() -> Vec<KeyInfo> {
    keys()
}

#[tauri::command]
pub fn pair_start(ctx: State<'_, AppCtx>, peer_id: String) {
    ctx.engine.send(Command::PairStart(peer_id));
}

#[tauri::command]
pub fn pair_submit(ctx: State<'_, AppCtx>, code: String) {
    ctx.engine.send(Command::PairSubmit(code));
}

#[tauri::command]
pub fn pair_cancel(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::PairCancel);
}

#[tauri::command]
pub fn unpair(ctx: State<'_, AppCtx>, peer_id: String) {
    ctx.engine.send(Command::Unpair(peer_id));
}

#[tauri::command]
pub fn set_peer_enabled(ctx: State<'_, AppCtx>, peer_id: String, enabled: bool) {
    ctx.engine.send(Command::SetPeerEnabled(peer_id, enabled));
}

#[tauri::command]
pub fn set_layout(ctx: State<'_, AppCtx>, positions: BTreeMap<String, Pos>) {
    ctx.engine.send(Command::SetLayout(positions));
}

#[tauri::command]
pub fn send_files(ctx: State<'_, AppCtx>, peer_id: String, paths: Vec<String>) {
    ctx.engine.send(Command::SendFiles(peer_id, paths.into_iter().map(PathBuf::from).collect()));
}

#[tauri::command]
pub fn cancel_transfer(ctx: State<'_, AppCtx>, id: String) {
    ctx.engine.send(Command::CancelTransfer(id));
}

#[tauri::command]
pub fn answer_transfer(ctx: State<'_, AppCtx>, id: String, accept: bool) {
    ctx.engine.send(Command::AnswerTransfer(id, accept));
}

#[tauri::command]
pub fn clear_transfers(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::ClearTransfers);
}

#[tauri::command]
pub fn reveal_path(app: AppHandle, path: String) -> Res<()> {
    let p = PathBuf::from(&path);
    let result = if p.is_dir() { app.opener().open_path(&path, None::<&str>) } else { app.opener().reveal_item_in_dir(&p) };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_save_dir(app: AppHandle, ctx: State<'_, AppCtx>) -> Res<()> {
    let dir = ctx.engine.info().await.ok_or_else(gone)?.save_dir;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_lock(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::ToggleLock);
}

#[tauri::command]
pub fn toggle_pause(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::TogglePause);
}

#[tauri::command]
pub fn request_permissions(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::RequestPermissions);
}

#[tauri::command]
pub fn open_permission_settings(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::OpenPermissionSettings);
}

#[tauri::command]
pub fn connect_address(ctx: State<'_, AppCtx>, address: String) {
    ctx.engine.send(Command::ConnectAddress(address));
}

#[tauri::command]
pub fn regenerate_identity(ctx: State<'_, AppCtx>) {
    ctx.engine.send(Command::RegenerateIdentity);
}

#[tauri::command]
pub fn open_logs(app: AppHandle, ctx: State<'_, AppCtx>) -> Res<()> {
    let dir = ctx.paths.logs();
    let _ = std::fs::create_dir_all(&dir);
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Res<()> {
    quit(&app).await;
    Ok(())
}

pub async fn quit(app: &AppHandle) {
    let ctx = app.state::<AppCtx>();
    *ctx.quitting.lock() = true;
    ctx.engine.shutdown().await;
    app.exit(0);
}

/// Files handed to Crispy (Send to / Dock) that the UI hasn't picked a destination for yet.
#[tauri::command]
pub fn take_send_request(ctx: State<'_, AppCtx>) -> Vec<String> {
    std::mem::take(&mut *ctx.pending_send.lock())
}
