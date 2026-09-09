pub mod apps;
pub mod backup;
mod commands;
pub mod config_dir;
mod config_override;
mod day_night;
mod fonts;
mod ghostty_import;
mod kv_config;
pub mod theme;
pub mod theme_store;

// Every unit test that sets the process-global `XDG_CONFIG_HOME` env var
// (in day_night, config_override, etc.) must serialize on this ONE lock, not
// a module-local one — cargo runs unit tests in the same binary on separate
// threads, so two different `Mutex`es don't actually prevent one test's env
// var from stomping another's mid-run.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
}

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn build_menu(app: &tauri::App) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let create_theme = MenuItemBuilder::with_id("create-theme", "Create Theme…")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let export_theme = MenuItemBuilder::with_id("export-theme", "Export Theme…")
        .accelerator("CmdOrCtrl+E")
        .build(app)?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&create_theme)
        .item(&export_theme)
        .separator()
        .close_window()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window").minimize().build()?;

    let app_menu = SubmenuBuilder::new(app, "Sheets").about(None).separator().quit().build()?;

    MenuBuilder::new(app).item(&app_menu).item(&file_menu).item(&edit_menu).item(&window_menu).build()
}

fn open_or_focus(app: &tauri::AppHandle, label: &str, page: &str, title: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(460.0, 720.0)
        .build();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let menu = build_menu(app)?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "create-theme" => open_or_focus(app, "create-theme", "create-theme.html", "Create Theme"),
            "export-theme" => open_or_focus(app, "export-theme", "export-theme.html", "Export Theme"),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::get_current_config,
            commands::set_config_path,
            commands::clear_config_path,
            commands::list_themes,
            commands::list_themes_for_period,
            commands::get_current_theme,
            commands::get_day_night_themes,
            commands::set_day_theme,
            commands::set_night_theme,
            commands::apply_theme,
            commands::list_font_families,
            commands::apply_font,
            commands::apply_opacity,
            commands::can_undo,
            commands::undo_last_change,
            commands::remove_user_theme,
            commands::create_custom_theme,
            commands::list_exportable_themes,
            commands::export_theme_to_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
