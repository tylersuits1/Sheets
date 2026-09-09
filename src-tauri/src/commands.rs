use crate::apps::{CurrentConfig, TerminalApp};
use crate::backup;
use crate::theme::{FontSettings, Period, Theme};
use crate::theme_store;
use serde::Serialize;

#[derive(Serialize)]
pub struct AppInfo {
    app: TerminalApp,
    installed: bool,
    config_path: Option<String>,
}

#[tauri::command]
pub fn list_apps() -> Vec<AppInfo> {
    TerminalApp::ALL
        .into_iter()
        .map(|app| {
            let adapter = app.adapter();
            AppInfo {
                app,
                installed: adapter.is_installed(),
                config_path: adapter.config_path().ok().map(|p| p.display().to_string()),
            }
        })
        .collect()
}

#[tauri::command]
pub fn get_current_config(app: TerminalApp) -> Result<CurrentConfig, String> {
    app.adapter().read_current()
}

#[tauri::command]
pub fn list_themes() -> Result<Vec<Theme>, String> {
    theme_store::list_themes()
}

#[tauri::command]
pub fn list_themes_for_period(period: Period) -> Result<Vec<Theme>, String> {
    theme_store::list_themes_for_period(period)
}

/// The theme currently applied to `app`, if its colors match a known theme.
/// `Ok(None)` covers both "nothing applied yet" and "hand-edited, unknown
/// colors" — either way, there's no Sheets theme to highlight as active.
#[tauri::command]
pub fn get_current_theme(app: TerminalApp) -> Result<Option<Theme>, String> {
    match app.adapter().read_current_palette()? {
        Some(palette) => theme_store::identify_theme(&palette),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn apply_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    let theme = theme_store::get_theme(&theme_id)?;
    let adapter = app.adapter();
    backup::snapshot(app, &adapter.config_path()?)?;
    adapter.apply_theme(&theme)
}

#[tauri::command]
pub fn apply_font(app: TerminalApp, font: FontSettings) -> Result<(), String> {
    let adapter = app.adapter();
    backup::snapshot(app, &adapter.config_path()?)?;
    adapter.apply_font(&font)
}

#[tauri::command]
pub fn apply_opacity(app: TerminalApp, opacity: f32) -> Result<(), String> {
    let adapter = app.adapter();
    backup::snapshot(app, &adapter.config_path()?)?;
    adapter.apply_opacity(opacity)
}

#[tauri::command]
pub fn can_undo(app: TerminalApp) -> bool {
    backup::has_backup(app)
}

#[tauri::command]
pub fn undo_last_change(app: TerminalApp) -> Result<(), String> {
    let path = app.adapter().config_path()?;
    backup::undo(app, &path)
}

#[tauri::command]
pub fn install_theme_from_git(git_url: String) -> Result<Theme, String> {
    theme_store::install_theme_from_git(&git_url)
}

#[tauri::command]
pub fn remove_user_theme(theme_id: String) -> Result<(), String> {
    theme_store::remove_user_theme(&theme_id)
}
