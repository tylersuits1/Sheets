use sheets_lib::apps::TerminalApp;
use sheets_lib::theme::FontSettings;
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
