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
mod update_check;

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
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

const GITHUB_URL: &str = "https://github.com/tylersuits1/Sheets";
const WEBSITE_URL: &str = "https://tylersuits.com";
const CONTACT_EMAIL_URL: &str = "mailto:hello@tylersuits.com";

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

    let check_updates = MenuItemBuilder::with_id("check-for-updates", "Check for Updates…").build(app)?;
    let sheets_github = MenuItemBuilder::with_id("sheets-github", "Sheets on GitHub").build(app)?;
    let visit_website = MenuItemBuilder::with_id("visit-website", "More Apps at tylersuits.com").build(app)?;
    let contact_email = MenuItemBuilder::with_id("contact-email", "Contact hello@tylersuits.com").build(app)?;
    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&check_updates)
        .separator()
        .item(&sheets_github)
        .item(&visit_website)
        .separator()
        .item(&contact_email)
        .build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
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

/// Checks GitHub for a newer release, off the main thread since it's a
/// blocking network call. Clicking through on an available update just opens
/// the release page — its own "Install" instructions cover how to update.
fn check_for_updates(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    std::thread::spawn(move || match update_check::check() {
        Ok(status) if status.update_available => {
            let release_url = status.release_url;
            let opener_handle = app_handle.clone();
            app_handle
                .dialog()
                .message(format!(
                    "You're on version {}. Version {} is available.",
                    status.current_version, status.latest_version
                ))
                .title("Update Available")
                .buttons(MessageDialogButtons::OkCancelCustom("View Release".into(), "Later".into()))
                .show(move |view_release| {
                    if view_release {
                        let _ = opener_handle.opener().open_url(release_url, None::<&str>);
                    }
                });
        }
        Ok(status) => {
            app_handle
                .dialog()
                .message(format!("You're on the latest version ({}).", status.current_version))
                .title("Up to Date")
                .buttons(MessageDialogButtons::Ok)
                .show(|_| {});
        }
        Err(err) => {
            app_handle
                .dialog()
                .message(format!("Couldn't check for updates: {err}"))
                .title("Update Check Failed")
                .kind(MessageDialogKind::Error)
                .buttons(MessageDialogButtons::Ok)
                .show(|_| {});
        }
    });
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
            "check-for-updates" => check_for_updates(app),
            "sheets-github" => {
                let _ = app.opener().open_url(GITHUB_URL, None::<&str>);
            }
            "visit-website" => {
                let _ = app.opener().open_url(WEBSITE_URL, None::<&str>);
            }
            "contact-email" => {
                let _ = app.opener().open_url(CONTACT_EMAIL_URL, None::<&str>);
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

    app.run(|_app_handle, _event| {
        // `RunEvent::Opened` only exists on macOS/iOS/Android — fired there
        // when a file is opened via Finder (double-click or "Open With >
        // Sheets"), whether Sheets was already running or this launched it.
        // There's no Linux equivalent to wire up here; Omarchy support is
        // otherwise just the adapters' own XDG-path handling.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = _event {
            for url in urls {
                if let Ok(path) = url.to_file_path() {
                    import_theme_and_notify(_app_handle, &path);
                }
            }
        }
    });
}
