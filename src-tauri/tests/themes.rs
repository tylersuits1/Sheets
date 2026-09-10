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
fn exported_theme_matches_the_documented_sheets_theme_json_shape() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-export-shape-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let palette = Palette {
        background: "1a1b26".into(),
        foreground: "c0caf5".into(),
        cursor: Some("c0caf5".into()),
        selection_background: Some("283457".into()),
        selection_foreground: None,
        ansi: [
            "15161e", "f7768e", "9ece6a", "e0af68", "7aa2f7", "bb9af7", "7dcfff", "a9b1d6", "414868", "f7768e",
            "9ece6a", "e0af68", "7aa2f7", "bb9af7", "7dcfff", "c0caf5",
        ]
        .map(String::from),
    };
    theme_store::save_user_theme("Tokyo Night Example".into(), ThemeVariant::Dark, palette).unwrap();
    let exported = theme_store::export_theme_json("tokyo-night-example").unwrap();

    // Kept in sync with the README's "Theme repo format" example: same key
    // names, key order, and null handling for an unset optional color.
    let expected = r#"{
  "name": "Tokyo Night Example",
  "variant": "dark",
  "palette": {
    "background": "1a1b26",
    "foreground": "c0caf5",
    "cursor": "c0caf5",
    "selection_background": "283457",
    "selection_foreground": null,
    "ansi": [
      "15161e",
      "f7768e",
      "9ece6a",
      "e0af68",
      "7aa2f7",
      "bb9af7",
      "7dcfff",
      "a9b1d6",
      "414868",
      "f7768e",
      "9ece6a",
      "e0af68",
      "7aa2f7",
      "bb9af7",
      "7dcfff",
      "c0caf5"
    ]
  }
}"#;
    assert_eq!(exported, expected);

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn imported_theme_matches_the_documented_sheets_theme_json_shape() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-import-shape-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let manifest_path = scratch.join("sheets-theme.json");
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::write(
        &manifest_path,
        r#"{
  "name": "Imported Example",
  "variant": "dark",
  "palette": {
    "background": "1a1b26",
    "foreground": "c0caf5",
    "cursor": "c0caf5",
    "selection_background": "283457",
    "selection_foreground": null,
    "ansi": [
      "15161e", "f7768e", "9ece6a", "e0af68",
      "7aa2f7", "bb9af7", "7dcfff", "a9b1d6",
      "414868", "f7768e", "9ece6a", "e0af68",
      "7aa2f7", "bb9af7", "7dcfff", "c0caf5"
    ]
  }
}"#,
    )
    .unwrap();

    let theme = theme_store::import_theme_from_file(manifest_path.to_str().unwrap()).unwrap();
    assert_eq!(theme.id, "imported-example");
    assert_eq!(theme.name, "Imported Example");
    assert_eq!(theme.variant, ThemeVariant::Dark);
    assert_eq!(theme.source, ThemeSource::UserInstalled);
    assert_eq!(theme.palette.background, "1a1b26");
    assert_eq!(theme.palette.selection_foreground, None);

    // It's now a real listed, exportable user theme, same as one created by hand.
    let listed = theme_store::get_theme("imported-example").unwrap();
    assert_eq!(listed.palette.foreground, "c0caf5");
    assert!(theme_store::export_theme_json("imported-example").is_ok());

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn importing_a_non_theme_json_file_is_a_clear_error() {
    let scratch = std::env::temp_dir().join(format!("sheets-import-bad-json-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let bad_path = scratch.join("not-a-theme.json");
    std::fs::write(&bad_path, r#"{"hello": "world"}"#).unwrap();

    let err = theme_store::import_theme_from_file(bad_path.to_str().unwrap()).unwrap_err();
    assert!(err.contains("not a valid sheets-theme.json"), "unexpected error: {err}");

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn saving_a_theme_named_after_a_builtin_is_rejected() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-builtin-collision-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    // "Edit Current Theme" on a pre-installed theme must never be able to
    // shadow or overwrite the original — it has to be saved under a
    // different name, becoming its own separate theme.
    let err = theme_store::save_user_theme("Tokyo Night".into(), ThemeVariant::Dark, sample_palette()).unwrap_err();
    assert!(err.contains("pre-installed theme"), "unexpected error: {err}");
    assert!(theme_store::get_theme("tokyo-night").unwrap().source == ThemeSource::BuiltIn, "the original must be untouched");

    // A distinct name for the same edited colors is fine.
    let saved =
        theme_store::save_user_theme("Tokyo Night (edited)".into(), ThemeVariant::Dark, sample_palette()).unwrap();
    assert_eq!(saved.source, ThemeSource::UserInstalled);

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn saving_a_theme_under_an_existing_user_themes_name_overwrites_it() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-overwrite-user-theme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    theme_store::save_user_theme("My Theme".into(), ThemeVariant::Dark, sample_palette()).unwrap();

    let mut edited = sample_palette();
    edited.background = "202020".into();
    let updated = theme_store::save_user_theme("My Theme".into(), ThemeVariant::Light, edited).unwrap();
    assert_eq!(updated.variant, ThemeVariant::Light);

    let listed = theme_store::get_theme("my-theme").unwrap();
    assert_eq!(listed.palette.background, "202020");
    assert_eq!(theme_store::list_themes().unwrap().iter().filter(|t| t.id == "my-theme").count(), 1);

    let _ = std::fs::remove_dir_all(&scratch);
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
