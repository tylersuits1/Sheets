use crate::theme::{Palette, Theme, ThemeSource, ThemeVariant};
use crate::theme_store::slugify;
use std::fs;
use std::path::{Path, PathBuf};

/// Ghostty ships ~450 built-in color schemes as plain files (one theme per
/// file, named by theme name, in the same flat `key = value` format as its
/// main config) under its install's resources directory. This scans that
/// directory and folds them into Sheets' theme list so they're available
/// for all three apps, not just Ghostty.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join("Applications/Ghostty.app/Contents/Resources/ghostty/themes"));
    }
    dirs.push(PathBuf::from("/Applications/Ghostty.app/Contents/Resources/ghostty/themes"));
    // Best-effort guesses for Linux packaging (XDG data dir convention);
    // unverified against a real Omarchy/Arch install.
    dirs.push(PathBuf::from("/usr/share/ghostty/themes"));
    dirs.push(PathBuf::from("/usr/local/share/ghostty/themes"));
    dirs.push(PathBuf::from("/usr/lib/ghostty/themes"));
    dirs
}

pub fn bundled_themes_dir() -> Option<PathBuf> {
    candidate_dirs().into_iter().find(|d| d.is_dir())
}

fn strip_hash(value: &str) -> String {
    value.trim().trim_start_matches('#').to_string()
}

fn parse_theme_file(contents: &str) -> Option<Palette> {
    let mut background = None;
    let mut foreground = None;
    let mut cursor = None;
    let mut selection_background = None;
    let mut selection_foreground = None;
    let mut ansi: [String; 16] = core::array::from_fn(|_| String::new());
    let mut ansi_set = [false; 16];

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        match key {
            "background" => background = Some(strip_hash(value)),
            "foreground" => foreground = Some(strip_hash(value)),
            "cursor-color" => cursor = Some(strip_hash(value)),
            "selection-background" => selection_background = Some(strip_hash(value)),
            "selection-foreground" => selection_foreground = Some(strip_hash(value)),
            "palette" => {
                if let Some((idx_str, hex)) = value.split_once('=') {
                    if let Ok(idx) = idx_str.trim().parse::<usize>() {
                        if idx < 16 {
                            ansi[idx] = strip_hash(hex);
                            ansi_set[idx] = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if !ansi_set.iter().all(|&set| set) {
        return None;
    }

    Some(Palette {
        background: background?,
        foreground: foreground?,
        cursor,
        selection_background,
        selection_foreground,
        ansi,
    })
}

fn relative_luminance(hex: &str) -> Option<f32> {
    if hex.len() != 6 {
        return None;
    }
    let channel = |slice: &str| u8::from_str_radix(slice, 16).ok().map(|v| v as f32 / 255.0);
    let r = channel(&hex[0..2])?;
    let g = channel(&hex[2..4])?;
    let b = channel(&hex[4..6])?;
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

fn guess_variant(palette: &Palette) -> ThemeVariant {
    match relative_luminance(&palette.background) {
        Some(l) if l > 0.5 => ThemeVariant::Light,
        _ => ThemeVariant::Dark,
    }
}

/// Core, directory-injectable logic (kept separate from `list_bundled_themes`
/// so tests don't depend on a real Ghostty install being present).
pub fn themes_in_dir(dir: &Path) -> Vec<Theme> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut themes: Vec<Theme> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            let contents = fs::read_to_string(entry.path()).ok()?;
            let palette = parse_theme_file(&contents)?;
            Some(Theme {
                id: format!("ghostty-{}", slugify(&name)),
                variant: guess_variant(&palette),
                source: ThemeSource::BuiltIn,
                git_url: None,
                name,
                palette,
            })
        })
        .collect();
    themes.sort_by(|a, b| a.name.cmp(&b.name));
    themes
}

pub fn list_bundled_themes() -> Vec<Theme> {
    match bundled_themes_dir() {
        Some(dir) => themes_in_dir(&dir),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK_THEME: &str = "\
palette = 0=#262427
palette = 1=#ff666d
palette = 2=#b3e03a
palette = 3=#ffc739
palette = 4=#00cde8
palette = 5=#a392e8
palette = 6=#9deaf6
palette = 7=#fcfcfa
palette = 8=#545452
palette = 9=#ff7e83
palette = 10=#bee55e
palette = 11=#ffd05e
palette = 12=#1bd5eb
palette = 13=#b0a3eb
palette = 14=#acedf8
palette = 15=#fcfcfa
background = #262427
foreground = #fcfcfa
cursor-color = #fcfcfa
selection-background = #fcfcfa
selection-foreground = #262427
";

    const LIGHT_THEME: &str = "\
palette = 0=#1a1a1a
palette = 1=#cc372e
palette = 2=#26a439
palette = 3=#cdac08
palette = 4=#0869cb
palette = 5=#9647bf
palette = 6=#479ec2
palette = 7=#98989d
palette = 8=#464646
palette = 9=#ff453a
palette = 10=#32d74b
palette = 11=#e5bc00
palette = 12=#0a84ff
palette = 13=#bf5af2
palette = 14=#69c9f2
palette = 15=#ffffff
background = #feffff
foreground = #000000
cursor-color = #98989d
selection-background = #abd8ff
selection-foreground = #000000
";

    #[test]
    fn parses_a_real_ghostty_theme_file() {
        let palette = parse_theme_file(DARK_THEME).unwrap();
        assert_eq!(palette.background, "262427");
        assert_eq!(palette.foreground, "fcfcfa");
        assert_eq!(palette.ansi[0], "262427");
        assert_eq!(palette.ansi[15], "fcfcfa");
    }

    #[test]
    fn rejects_a_file_missing_palette_entries() {
        let truncated = "background = #000000\nforeground = #ffffff\npalette = 0=#000000\n";
        assert!(parse_theme_file(truncated).is_none());
    }

    #[test]
    fn guesses_dark_and_light_variants_from_background_luminance() {
        assert_eq!(guess_variant(&parse_theme_file(DARK_THEME).unwrap()), ThemeVariant::Dark);
        assert_eq!(guess_variant(&parse_theme_file(LIGHT_THEME).unwrap()), ThemeVariant::Light);
    }

    #[test]
    fn themes_in_dir_reads_every_file_and_slugifies_names_with_spaces() {
        let dir = std::env::temp_dir().join(format!("sheets-ghostty-import-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("3024 Day"), LIGHT_THEME).unwrap();
        fs::write(dir.join("Aardvark Blue"), DARK_THEME).unwrap();
        fs::write(dir.join("not-a-theme.txt"), "garbage, not key=value at all").unwrap();

        let themes = themes_in_dir(&dir);
        assert_eq!(themes.len(), 2, "the malformed file should be skipped, not error");
        assert!(themes.iter().any(|t| t.id == "ghostty-3024-day" && t.variant == ThemeVariant::Light));
        assert!(themes.iter().any(|t| t.id == "ghostty-aardvark-blue" && t.variant == ThemeVariant::Dark));
        assert!(themes.iter().all(|t| t.source == ThemeSource::BuiltIn));

        let _ = fs::remove_dir_all(&dir);
    }
}
