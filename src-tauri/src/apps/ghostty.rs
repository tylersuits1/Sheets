use super::{ConfigAdapter, CurrentConfig};
use crate::config_dir::xdg_config_home;
use crate::kv_config::{KvConfig, KvSyntax};
use crate::theme::{FontSettings, Palette, Theme};
use std::path::PathBuf;

pub struct GhosttyAdapter;

impl ConfigAdapter for GhosttyAdapter {
    fn config_path(&self) -> Result<PathBuf, String> {
        Ok(xdg_config_home()?.join("ghostty").join("config"))
    }

    fn read_current(&self) -> Result<CurrentConfig, String> {
        let cfg = KvConfig::load(&self.config_path()?, KvSyntax::Equals)?;
        Ok(CurrentConfig {
            font_family: cfg.get("font-family"),
            font_size: cfg.get("font-size").and_then(|v| v.parse().ok()),
            opacity: cfg.get("background-opacity").and_then(|v| v.parse().ok()),
        })
    }

    fn read_current_palette(&self) -> Result<Option<Palette>, String> {
        let cfg = KvConfig::load(&self.config_path()?, KvSyntax::Equals)?;
        let (background, foreground) = match (cfg.get("background"), cfg.get("foreground")) {
            (Some(b), Some(f)) => (b, f),
            _ => return Ok(None),
        };

        let entries = cfg.get_all("palette");
        if entries.len() != 16 {
            return Ok(None);
        }
        let mut ansi: [String; 16] = core::array::from_fn(|_| String::new());
        for entry in entries {
            let (idx, hex) = entry.split_once('=').ok_or_else(|| format!("malformed palette entry: {entry}"))?;
            let idx: usize = idx.trim().parse().map_err(|_| format!("bad palette index in: {entry}"))?;
            if idx >= 16 {
                return Err(format!("palette index out of range: {entry}"));
            }
            ansi[idx] = hex.trim().to_string();
        }

        Ok(Some(Palette {
            background,
            foreground,
            cursor: cfg.get("cursor-color"),
            selection_background: cfg.get("selection-background"),
            selection_foreground: cfg.get("selection-foreground"),
            ansi,
        }))
    }

    fn apply_theme(&self, theme: &Theme) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Equals)?;

        cfg.set("background", &theme.palette.background);
        cfg.set("foreground", &theme.palette.foreground);
        if let Some(cursor) = &theme.palette.cursor {
            cfg.set("cursor-color", cursor);
        }
        if let Some(bg) = &theme.palette.selection_background {
            cfg.set("selection-background", bg);
        }
        if let Some(fg) = &theme.palette.selection_foreground {
            cfg.set("selection-foreground", fg);
        }

        cfg.unset("palette");
        for (i, hex) in theme.palette.ansi.iter().enumerate() {
            cfg.push("palette", &format!("{i}={hex}"));
        }

        cfg.save(&path)
    }

    fn apply_font(&self, font: &FontSettings) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Equals)?;
        cfg.set("font-family", &font.family);
        cfg.set("font-size", &font.size.to_string());
        cfg.save(&path)
    }

    fn apply_opacity(&self, opacity: f32) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Equals)?;
        cfg.set("background-opacity", &opacity.to_string());
        cfg.save(&path)
    }
}
