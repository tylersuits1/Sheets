use super::{ConfigAdapter, CurrentConfig, TerminalApp};
use crate::config_dir::xdg_config_home;
use crate::config_override;
use crate::theme::{FontSettings, Palette, Theme};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut, Item, Table};

/// Gets (creating as needed) the table at `path`. Intermediate segments that
/// only ever hold subtables (e.g. `colors` above `colors.primary`) are
/// marked implicit so they don't render an empty `[colors]` header of their
/// own; the final segment is left explicit since callers assign values to
/// it directly.
fn table_mut<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> &'a mut Table {
    let mut table: &mut Table = doc;
    let last = path.len() - 1;
    for (i, key) in path.iter().enumerate() {
        let is_new = table.get(key).is_none();
        let item = table.entry(key).or_insert(Item::Table(Table::new()));
        if is_new && i != last {
            if let Item::Table(t) = item {
                t.set_implicit(true);
            }
        }
        table = item.as_table_mut().expect("path segment exists but is not a table");
    }
    table
}

pub struct AlacrittyAdapter;

const ANSI_NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

fn hex(v: &str) -> String {
    format!("#{v}")
}

/// f32 -> f64 widening introduces binary-representation noise (e.g. 0.85
/// becomes 0.8500000238418579); round to 4 decimal places so the written
/// TOML looks like what the user actually typed.
fn rounded(v: f32) -> f64 {
    ((v as f64) * 10_000.0).round() / 10_000.0
}

fn load_doc(path: &PathBuf) -> Result<DocumentMut, String> {
    let contents = if path.exists() {
        fs::read_to_string(path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    contents.parse::<DocumentMut>().map_err(|e| e.to_string())
}

fn save_doc(path: &PathBuf, doc: &DocumentMut) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, doc.to_string()).map_err(|e| e.to_string())
}

fn get_str(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    let mut item: &Item = doc.get(path[0])?;
    for key in &path[1..] {
        item = item.get(key)?;
    }
    item.as_str().map(|s| s.trim_start_matches('#').to_string())
}

impl ConfigAdapter for AlacrittyAdapter {
    fn config_path(&self) -> Result<PathBuf, String> {
        if let Some(p) = config_override::get(TerminalApp::Alacritty)? {
            return Ok(p);
        }
        Ok(xdg_config_home()?.join("alacritty").join("alacritty.toml"))
    }

    fn read_current(&self) -> Result<CurrentConfig, String> {
        let doc = load_doc(&self.config_path()?)?;
        let font_family = doc
            .get("font")
            .and_then(|f| f.get("normal"))
            .and_then(|n| n.get("family"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let font_size = doc
            .get("font")
            .and_then(|f| f.get("size"))
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .map(|v| v as f32);
        let opacity = doc
            .get("window")
            .and_then(|w| w.get("opacity"))
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .map(|v| v as f32);

        Ok(CurrentConfig { font_family, font_size, opacity })
    }

    fn read_current_palette(&self) -> Result<Option<Palette>, String> {
        let doc = load_doc(&self.config_path()?)?;
        let (background, foreground) = match (
            get_str(&doc, &["colors", "primary", "background"]),
            get_str(&doc, &["colors", "primary", "foreground"]),
        ) {
            (Some(b), Some(f)) => (b, f),
            _ => return Ok(None),
        };

        let mut ansi: [String; 16] = core::array::from_fn(|_| String::new());
        for (i, name) in ANSI_NAMES.iter().enumerate() {
            match get_str(&doc, &["colors", "normal", name]) {
                Some(v) => ansi[i] = v,
                None => return Ok(None),
            }
        }
        for (i, name) in ANSI_NAMES.iter().enumerate() {
            match get_str(&doc, &["colors", "bright", name]) {
                Some(v) => ansi[8 + i] = v,
                None => return Ok(None),
            }
        }

        Ok(Some(Palette {
            background,
            foreground,
            cursor: get_str(&doc, &["colors", "cursor", "cursor"]),
            selection_background: get_str(&doc, &["colors", "selection", "background"]),
            selection_foreground: get_str(&doc, &["colors", "selection", "text"]),
            ansi,
        }))
    }

    fn apply_theme(&self, theme: &Theme) -> Result<(), String> {
        let path = self.config_path()?;
        let mut doc = load_doc(&path)?;

        let primary = table_mut(&mut doc, &["colors", "primary"]);
        primary["background"] = value(hex(&theme.palette.background));
        primary["foreground"] = value(hex(&theme.palette.foreground));

        if let Some(cursor) = &theme.palette.cursor {
            let cursor_table = table_mut(&mut doc, &["colors", "cursor"]);
            cursor_table["cursor"] = value(hex(cursor));
        }

        if theme.palette.selection_background.is_some() || theme.palette.selection_foreground.is_some() {
            let selection = table_mut(&mut doc, &["colors", "selection"]);
            if let Some(bg) = &theme.palette.selection_background {
                selection["background"] = value(hex(bg));
            }
            if let Some(fg) = &theme.palette.selection_foreground {
                selection["text"] = value(hex(fg));
            }
        }

        {
            let normal = table_mut(&mut doc, &["colors", "normal"]);
            for (name, hex_value) in ANSI_NAMES.iter().zip(&theme.palette.ansi[0..8]) {
                normal[*name] = value(hex(hex_value));
            }
        }
        {
            let bright = table_mut(&mut doc, &["colors", "bright"]);
            for (name, hex_value) in ANSI_NAMES.iter().zip(&theme.palette.ansi[8..16]) {
                bright[*name] = value(hex(hex_value));
            }
        }

        save_doc(&path, &doc)
    }

    fn apply_font(&self, font: &FontSettings) -> Result<(), String> {
        let path = self.config_path()?;
        let mut doc = load_doc(&path)?;

        let normal = table_mut(&mut doc, &["font", "normal"]);
        normal["family"] = value(font.family.clone());

        let font_table = table_mut(&mut doc, &["font"]);
        font_table["size"] = value(rounded(font.size));

        save_doc(&path, &doc)
    }

    fn apply_opacity(&self, opacity: f32) -> Result<(), String> {
        let path = self.config_path()?;
        let mut doc = load_doc(&path)?;
        let window = table_mut(&mut doc, &["window"]);
        window["opacity"] = value(rounded(opacity));
        save_doc(&path, &doc)
    }
}
