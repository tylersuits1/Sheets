use sheets_lib::theme::{Palette, ThemeSource, ThemeVariant};
use sheets_lib::theme_store;
use std::sync::Mutex;

// save_user_theme/export_theme_json touch the shared themes.json manifest
// under XDG_CONFIG_HOME, same as the adapter tests, so serialize access.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn sample_palette() -> Palette {
    Palette {
        background: "101010".into(),
        foreground: "efefef".into(),
        cursor: None,
        selection_background: None,
        selection_foreground: None,
        ansi: [
            "000000", "aa0000", "00aa00", "aa5500", "0000aa", "aa00aa", "00aaaa", "aaaaaa", "555555",
            "ff5555", "55ff55", "ffff55", "5555ff", "ff55ff", "55ffff", "ffffff",
        ]
        .map(String::from),
    }
}

#[test]
fn save_user_theme_is_listed_and_exportable() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-create-theme-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let created = theme_store::save_user_theme("My Spooky Theme".into(), ThemeVariant::Dark, sample_palette()).unwrap();
    assert_eq!(created.id, "my-spooky-theme");
    assert_eq!(created.source, ThemeSource::UserInstalled);
    assert!(created.git_url.is_none());

    let listed = theme_store::get_theme("my-spooky-theme").unwrap();
    assert_eq!(listed.palette.background, "101010");

    let exported = theme_store::export_theme_json("my-spooky-theme").unwrap();
    assert!(exported.contains("\"My Spooky Theme\""));
    assert!(exported.contains("\"dark\""));
    assert!(exported.contains("101010"));

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn built_in_themes_cannot_be_exported() {
    let err = theme_store::export_theme_json("tokyo-night").unwrap_err();
    assert!(err.contains("isn't a user-created theme"), "unexpected error: {err}");
}

#[test]
fn empty_theme_name_is_rejected() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-create-theme-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    assert!(theme_store::save_user_theme("   ".into(), ThemeVariant::Both, sample_palette()).is_err());

    let _ = std::fs::remove_dir_all(&scratch);
}
