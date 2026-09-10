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
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

fn build_menu(app: &tauri::App) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let create_theme = MenuItemBuilder::with_id("create-theme", "Create Theme…")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let import_theme = MenuItemBuilder::with_id("import-theme", "Import Theme…")
        .accelerator("CmdOrCtrl+I")
        .build(app)?;
    let export_theme = MenuItemBuilder::with_id("export-theme", "Export Theme…")
        .accelerator("CmdOrCtrl+E")
        .build(app)?;
    let edit_current_theme = MenuItemBuilder::with_id("edit-current-theme", "Edit Current Theme…")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(app)?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&create_theme)
        .item(&edit_current_theme)
        .item(&import_theme)
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

pub(crate) fn open_or_focus(app: &tauri::AppHandle, label: &str, page: &str, title: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .title(title)
        .inner_size(460.0, 720.0)
        .build();
}

/// Imports a `sheets-theme.json` file and tells the main window about the
/// result, whether that came from File > Import Theme… (a picked file path)
/// or macOS "Open With > Sheets" / double-clicking one (`RunEvent::Opened`).
fn import_theme_and_notify(app: &tauri::AppHandle, path: &std::path::Path) {
    let result = theme_store::import_theme_from_file(&path.display().to_string());
    let Some(window) = app.get_webview_window("main") else { return };
    let _ = window.set_focus();
    match result {
        Ok(theme) => {
            let _ = window.emit("theme-imported", &theme);
        }
        Err(err) => {
            let _ = window.emit("theme-import-error", &err);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let menu = build_menu(app)?;
            app.set_menu(menu)?;
            Ok(())
        })
        .manage(commands::EditSeedState(std::sync::Mutex::new(None)))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "create-theme" => open_or_focus(app, "create-theme", "create-theme.html", "Create Theme"),
            "export-theme" => open_or_focus(app, "export-theme", "export-theme.html", "Export Theme"),
            // The main window is the one that knows which app tab is
            // selected, so it computes the seed colors itself once notified.
            "edit-current-theme" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("menu-edit-current-theme", ());
                }
            }
            "import-theme" => {
                let app_handle = app.clone();
                app.dialog().file().add_filter("Sheets theme", &["json"]).pick_file(move |file| {
                    let Some(file) = file else { return };
                    let Ok(path) = file.into_path() else { return };
                    import_theme_and_notify(&app_handle, &path);
                });
            }
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
            commands::get_current_palette,
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
            commands::import_theme_from_file,
            commands::list_exportable_themes,
            commands::export_theme_to_path,
            commands::set_edit_seed,
            commands::take_edit_seed,
            commands::open_create_theme_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        // Fired on macOS when a file is opened via Finder (double-click or
        // "Open With > Sheets"), whether Sheets was already running or this
        // is what launched it.
        if let tauri::RunEvent::Opened { urls } = event {
            for url in urls {
                if let Ok(path) = url.to_file_path() {
                    import_theme_and_notify(app_handle, &path);
                }
            }
        }
    });
}
