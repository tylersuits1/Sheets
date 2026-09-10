use crate::apps::{CurrentConfig, TerminalApp};
use crate::backup;
use crate::config_override;
use crate::day_night;
use crate::fonts;
use crate::theme::{FontSettings, Palette, Period, Theme, ThemeVariant};
use crate::theme_store;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

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

/// Called after the user picks a config file by hand (via a native "Locate
/// config file…" dialog) for an app Sheets couldn't auto-detect, e.g. a
/// custom dotfiles setup that doesn't live at the usual XDG path.
#[tauri::command]
pub fn set_config_path(app: TerminalApp, path: String) -> Result<(), String> {
    config_override::set(app, &PathBuf::from(path))
}

#[tauri::command]
pub fn clear_config_path(app: TerminalApp) -> Result<(), String> {
    config_override::clear(app)
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

/// The raw colors currently configured for `app`, regardless of whether they
/// match a known theme — used by "Edit Current Theme" to pre-fill the editor
/// even when the live colors don't identify to anything Sheets recognizes.
#[tauri::command]
pub fn get_current_palette(app: TerminalApp) -> Result<Option<Palette>, String> {
    app.adapter().read_current_palette()
}

#[derive(Serialize)]
pub struct DayNightThemes {
    day: Option<Theme>,
    night: Option<Theme>,
}

/// The themes designated for day/night use on `app`, separate from
/// whichever theme is actually applied right now. Either side is `None`
/// until the user sets one via `set_day_theme`/`set_night_theme`.
#[tauri::command]
pub fn get_day_night_themes(app: TerminalApp) -> Result<DayNightThemes, String> {
    let day = match day_night::get_day_theme_id(app)? {
        Some(id) => theme_store::get_theme(&id).ok(),
        None => None,
    };
    let night = match day_night::get_night_theme_id(app)? {
        Some(id) => theme_store::get_theme(&id).ok(),
        None => None,
    };
    Ok(DayNightThemes { day, night })
}

#[tauri::command]
pub fn set_day_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    theme_store::get_theme(&theme_id)?;
    day_night::set_day_theme(app, theme_id)
}

#[tauri::command]
pub fn set_night_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    theme_store::get_theme(&theme_id)?;
    day_night::set_night_theme(app, theme_id)
}

#[tauri::command]
pub fn apply_theme(app: TerminalApp, theme_id: String) -> Result<(), String> {
    let theme = theme_store::get_theme(&theme_id)?;
    let adapter = app.adapter();
    backup::snapshot(app, &adapter.config_path()?)?;
    adapter.apply_theme(&theme)
}

#[tauri::command]
pub fn list_font_families() -> Result<Vec<String>, String> {
    fonts::list_font_families()
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
pub fn remove_user_theme(theme_id: String) -> Result<(), String> {
    theme_store::remove_user_theme(&theme_id)
}

#[tauri::command]
pub fn create_custom_theme(name: String, variant: ThemeVariant, palette: Palette) -> Result<Theme, String> {
    theme_store::save_user_theme(name, variant, palette)
}

/// Only user-created/user-installed themes are exportable; the picker on the
/// frontend should already be filtered to those, but this is the actual
/// enforcement point.
#[tauri::command]
pub fn list_exportable_themes() -> Result<Vec<Theme>, String> {
    use crate::theme::ThemeSource;
    Ok(theme_store::list_themes()?.into_iter().filter(|t| t.source == ThemeSource::UserInstalled).collect())
}

#[tauri::command]
pub fn export_theme_to_path(theme_id: String, path: String) -> Result<(), String> {
    let json = theme_store::export_theme_json(&theme_id)?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Used by File > Import Theme… (a native file picker) — opening a
/// `sheets-theme.json` from Finder's "Open With" goes through the same
/// `theme_store::import_theme_from_file` directly from `lib.rs` instead,
/// since that path starts outside any invoke() call.
#[tauri::command]
pub fn import_theme_from_file(path: String) -> Result<Theme, String> {
    theme_store::import_theme_from_file(&path)
}

/// Hand-off state for "Edit Current Theme": the main window (which knows
/// which app tab is selected) computes the seed colors and stashes them here
/// just before opening the Create Theme window, which reads them back once
/// on load via `take_edit_seed`. A plain "File > Create Theme" leaves this
/// empty, so the window opens blank as usual.
pub struct EditSeedState(pub Mutex<Option<EditSeed>>);

#[derive(Clone, Serialize, Deserialize)]
pub struct EditSeed {
    pub name: String,
    pub variant: ThemeVariant,
    pub palette: Palette,
}

#[tauri::command]
pub fn set_edit_seed(seed: EditSeed, state: tauri::State<EditSeedState>) {
    *state.0.lock().unwrap() = Some(seed);
}

#[tauri::command]
pub fn take_edit_seed(state: tauri::State<EditSeedState>) -> Option<EditSeed> {
    state.0.lock().unwrap().take()
}

/// Opens (or focuses) the Create Theme window from the frontend — used for
/// "Edit Current Theme", which needs the main window's own knowledge of
/// which app tab is selected before it can compute seed colors, so it can't
/// be triggered as a plain native menu action the way Create/Export are.
#[tauri::command]
pub fn open_create_theme_window(app: tauri::AppHandle) {
    crate::open_or_focus(&app, "create-theme", "create-theme.html", "Create Theme");
}
