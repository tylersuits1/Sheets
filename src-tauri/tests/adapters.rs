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
fn ghostty_apply_theme_clears_a_conflicting_hand_written_theme_directive() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-theme-directive-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let ghostty_dir = scratch.join("ghostty");
    fs::create_dir_all(&ghostty_dir).unwrap();
    // Ghostty's own `theme` directive (commonly auto-set to follow macOS's
    // system appearance) loads a whole named theme and overrides explicit
    // background/foreground/palette settings regardless of file order -
    // applying a Sheets theme has to actually take visible effect over it.
    fs::write(
        ghostty_dir.join("config"),
        "theme = dark:Apple System Colors, light:Apple System Colors Light\nfont-family = Menlo\n",
    )
    .unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let theme = theme_store::get_theme("tokyo-night").unwrap();
    TerminalApp::Ghostty.adapter().apply_theme(&theme).unwrap();

    let contents = fs::read_to_string(ghostty_dir.join("config")).unwrap();
    assert!(!contents.contains("theme ="), "the conflicting theme directive should be removed:\n{contents}");
    assert!(contents.contains("font-family = Menlo"), "unrelated hand-written lines should survive");
    assert!(contents.contains("background = 1a1b26"));

    let _ = fs::remove_dir_all(&scratch);
}

#[test]
fn ghostty_apply_theme_overrides_a_hand_written_palette_ghostty_actually_keeps() {
    let _guard = ENV_LOCK.lock().unwrap();
    let scratch = std::env::temp_dir().join(format!("sheets-first-wins-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let ghostty_dir = scratch.join("ghostty");
    fs::create_dir_all(&ghostty_dir).unwrap();
    // Confirmed against a real Ghostty install: when a scalar key like
    // `background` is defined twice, Ghostty renders using the FIRST
    // definition, not the last. A hand-written palette from before Sheets
    // was ever used (like a dotfiles-managed config) must not survive
    // alongside Sheets' managed block, or Sheets' apply is silently a no-op.
    let hand_written = "\
macos-titlebar-style = native
background-opacity = 0.8

foreground = #d3c6aa
background = #1e2326
cursor-color = #e69875
palette = 0=#7a8478
palette = 1=#e67e80
palette = 2=#a7c080
palette = 3=#dbbc7f
palette = 4=#7fbbb3
palette = 5=#d699b6
palette = 6=#83c092
palette = 7=#f2efdf
palette = 8=#a6b0a0
palette = 9=#f85552
palette = 10=#8da101
palette = 11=#dfa000
palette = 12=#3a94c5
palette = 13=#df69ba
palette = 14=#35a77c
palette = 15=#fffbef
";
    fs::write(ghostty_dir.join("config"), hand_written).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &scratch);

    let theme = theme_store::get_theme("tokyo-night").unwrap();
    TerminalApp::Ghostty.adapter().apply_theme(&theme).unwrap();

    let contents = fs::read_to_string(ghostty_dir.join("config")).unwrap();
    let count_key = |key: &str| contents.lines().filter(|l| l.trim_start().starts_with(&format!("{key} = "))).count();
    assert_eq!(count_key("background"), 1, "only one background definition should remain:\n{contents}");
    assert_eq!(count_key("foreground"), 1, "only one foreground definition should remain:\n{contents}");
    assert!(!contents.contains("#1e2326"), "the old hand-written background must be gone:\n{contents}");
    assert!(contents.contains("background = 1a1b26"), "the new theme's background must be present:\n{contents}");
    assert!(contents.contains("macos-titlebar-style = native"), "unrelated hand-written settings must survive");

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
