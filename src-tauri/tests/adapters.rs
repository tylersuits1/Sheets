use sheets_lib::apps::TerminalApp;
use sheets_lib::backup;
use sheets_lib::theme::{FontSettings, Period};
use sheets_lib::theme_store;
use std::fs;
use std::sync::Mutex;

// Both tests mutate the process-global `XDG_CONFIG_HOME` env var, so they
// must not run concurrently.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Exercises the full backbone end to end: pointing all three adapters at a
/// scratch config directory, applying a built-in theme + font + opacity, and
/// checking the files they write are sane. Everything runs in one test
/// function since it depends on the process-global `XDG_CONFIG_HOME`.
#[test]
fn adapters_write_expected_config_files() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-adapter-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let theme = theme_store::get_theme("tokyo-night").expect("built-in theme should exist");
    let font = FontSettings { family: "JetBrains Mono".into(), size: 13.0 };

    for app in TerminalApp::ALL {
        let adapter = app.adapter();
        assert!(!adapter.is_installed(), "{app:?} config should not exist yet");
        assert!(
            adapter.read_current_palette().unwrap().is_none(),
            "{app:?} should report no palette before anything is applied"
        );

        adapter.apply_theme(&theme).expect("apply_theme should succeed");
        adapter.apply_font(&font).expect("apply_font should succeed");
        adapter.apply_opacity(0.85).expect("apply_opacity should succeed");

        assert!(adapter.is_installed(), "{app:?} config should exist after writing");

        let current = adapter.read_current().expect("read_current should succeed");
        assert_eq!(current.font_family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(current.font_size, Some(13.0));
        assert!((current.opacity.unwrap() - 0.85).abs() < 0.001);

        let path = adapter.config_path().unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(
            contents.contains(&theme.palette.background) || contents.contains("1a1b26"),
            "{app:?} config should contain the theme's background color:\n{contents}"
        );
        if app != TerminalApp::Alacritty {
            let marker_count = contents.matches("Managed by Sheets").count();
            assert_eq!(
                marker_count, 1,
                "{app:?} should have exactly one managed-block marker after 3 applies, found {marker_count}:\n{contents}"
            );
        }

        let read_back = adapter
            .read_current_palette()
            .unwrap()
            .unwrap_or_else(|| panic!("{app:?} should report a full palette after apply_theme"));
        assert_eq!(read_back.background.to_lowercase(), theme.palette.background.to_lowercase());
        assert_eq!(read_back.foreground.to_lowercase(), theme.palette.foreground.to_lowercase());
        let identified = theme_store::identify_theme(&read_back).unwrap();
        assert_eq!(
            identified.map(|t| t.id),
            Some("tokyo-night".to_string()),
            "{app:?} should reverse-match back to the tokyo-night theme"
        );

        // Re-applying must not duplicate managed entries or corrupt the file.
        adapter.apply_theme(&theme).expect("second apply_theme should succeed");
        let contents_after_reapply = fs::read_to_string(&path).unwrap();
        let background_occurrences = contents_after_reapply.matches(&theme.palette.background).count();
        assert_eq!(
            background_occurrences, 1,
            "{app:?} should have exactly one background entry after reapplying, found {background_occurrences}:\n{contents_after_reapply}"
        );
    }

    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn ghostty_preserves_hand_written_lines_outside_the_managed_block() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-preserve-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let ghostty_dir = scratch.join("ghostty");
    fs::create_dir_all(&ghostty_dir).unwrap();
    fs::write(ghostty_dir.join("config"), "keybind = ctrl+shift+c=copy\n# a comment\n").unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let theme = theme_store::get_theme("solarized-light").unwrap();
    TerminalApp::Ghostty.adapter().apply_theme(&theme).unwrap();

    let contents = fs::read_to_string(ghostty_dir.join("config")).unwrap();
    assert!(contents.contains("keybind = ctrl+shift+c=copy"));
    assert!(contents.contains("# a comment"));
    assert!(contents.contains("background = fdf6e3"));

    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn day_and_night_period_queries_filter_by_variant() {
    let day = theme_store::list_themes_for_period(Period::Day).unwrap();
    assert!(day.iter().any(|t| t.id == "solarized-light"), "Day should include the Light theme");
    assert!(!day.iter().any(|t| t.id == "tokyo-night"), "Day should exclude the Dark theme");

    let night = theme_store::list_themes_for_period(Period::Night).unwrap();
    assert!(night.iter().any(|t| t.id == "tokyo-night"), "Night should include the Dark theme");
    assert!(!night.iter().any(|t| t.id == "solarized-light"), "Night should exclude the Light theme");
}

#[test]
fn identify_theme_returns_none_for_unrecognized_colors() {
    let mut custom = theme_store::get_theme("tokyo-night").unwrap().palette;
    custom.background = "ffffff".into();
    assert!(theme_store::identify_theme(&custom).unwrap().is_none());
}

#[test]
fn undo_restores_the_hand_written_config_that_preceded_a_change() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-undo-existing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let ghostty_dir = scratch.join("ghostty");
    fs::create_dir_all(&ghostty_dir).unwrap();
    let original = "keybind = ctrl+shift+c=copy\nfont-family = Menlo\n";
    fs::write(ghostty_dir.join("config"), original).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let adapter = TerminalApp::Ghostty.adapter();
    let path = adapter.config_path().unwrap();

    assert!(!backup::has_backup(TerminalApp::Ghostty));
    backup::snapshot(TerminalApp::Ghostty, &path).unwrap();
    assert!(backup::has_backup(TerminalApp::Ghostty));

    let theme = theme_store::get_theme("tokyo-night").unwrap();
    adapter.apply_theme(&theme).unwrap();
    assert_ne!(fs::read_to_string(&path).unwrap(), original, "sanity check: the apply should have changed the file");

    backup::undo(TerminalApp::Ghostty, &path).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!backup::has_backup(TerminalApp::Ghostty), "undo should consume the backup");

    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn undo_removes_a_config_that_did_not_exist_before_the_change() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-undo-fresh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let adapter = TerminalApp::Kitty.adapter();
    let path = adapter.config_path().unwrap();
    assert!(!path.exists());

    backup::snapshot(TerminalApp::Kitty, &path).unwrap();
    let theme = theme_store::get_theme("solarized-light").unwrap();
    adapter.apply_theme(&theme).unwrap();
    assert!(path.exists());

    backup::undo(TerminalApp::Kitty, &path).unwrap();
    assert!(!path.exists(), "undo should remove a config Sheets created from nothing");

    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn undo_with_nothing_to_undo_is_an_error() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-undo-none-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    assert!(!backup::has_backup(TerminalApp::Alacritty));
    let path = TerminalApp::Alacritty.adapter().config_path().unwrap();
    assert!(backup::undo(TerminalApp::Alacritty, &path).is_err());

    let _ = fs::remove_dir_all(&scratch);
}
