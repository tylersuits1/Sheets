pub mod apps;
mod commands;
pub mod config_dir;
mod kv_config;
pub mod theme;
pub mod theme_store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::get_current_config,
            commands::list_themes,
            commands::list_themes_for_period,
            commands::get_current_theme,
            commands::apply_theme,
            commands::apply_font,
            commands::apply_opacity,
            commands::install_theme_from_git,
            commands::remove_user_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
