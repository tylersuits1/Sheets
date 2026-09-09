use crate::apps::{CurrentConfig, TerminalApp};
use crate::theme::{FontSettings, Theme};
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
pub fn apply_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    let theme = theme_store::get_theme(&theme_id)?;
    app.adapter().apply_theme(&theme)
}

#[tauri::command]
pub fn apply_font(app: TerminalApp, font: FontSettings) -> Result<(), String> {
    app.adapter().apply_font(&font)
}

#[tauri::command]
pub fn apply_opacity(app: TerminalApp, opacity: f32) -> Result<(), String> {
    app.adapter().apply_opacity(opacity)
}

#[tauri::command]
pub fn install_theme_from_git(git_url: String) -> Result<Theme, String> {
    theme_store::install_theme_from_git(&git_url)
}

#[tauri::command]
pub fn remove_user_theme(theme_id: String) -> Result<(), String> {
    theme_store::remove_user_theme(&theme_id)
}
