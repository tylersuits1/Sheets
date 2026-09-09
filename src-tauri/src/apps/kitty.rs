use super::{ConfigAdapter, CurrentConfig, TerminalApp};
use crate::config_dir::xdg_config_home;
use crate::config_override;
use crate::kv_config::{KvConfig, KvSyntax};
use crate::theme::{FontSettings, Palette, Theme};
use std::path::PathBuf;

pub struct KittyAdapter;

fn hex(value: &str) -> String {
    format!("#{value}")
}

fn unhex(value: &str) -> String {
    value.trim_start_matches('#').to_string()
}

impl ConfigAdapter for KittyAdapter {
    fn config_path(&self) -> Result<PathBuf, String> {
        if let Some(p) = config_override::get(TerminalApp::Kitty)? {
            return Ok(p);
        }
        Ok(xdg_config_home()?.join("kitty").join("kitty.conf"))
    }

    fn read_current(&self) -> Result<CurrentConfig, String> {
        let cfg = KvConfig::load(&self.config_path()?, KvSyntax::Space)?;
        Ok(CurrentConfig {
            font_family: cfg.get("font_family"),
            font_size: cfg.get("font_size").and_then(|v| v.parse().ok()),
            opacity: cfg.get("background_opacity").and_then(|v| v.parse().ok()),
        })
    }

    fn read_current_palette(&self) -> Result<Option<Palette>, String> {
        let cfg = KvConfig::load(&self.config_path()?, KvSyntax::Space)?;
        let (background, foreground) = match (cfg.get("background"), cfg.get("foreground")) {
            (Some(b), Some(f)) => (unhex(&b), unhex(&f)),
            _ => return Ok(None),
        };

        let mut ansi: [String; 16] = core::array::from_fn(|_| String::new());
        for (i, slot) in ansi.iter_mut().enumerate() {
            match cfg.get(&format!("color{i}")) {
                Some(v) => *slot = unhex(&v),
                None => return Ok(None),
            }
        }

        Ok(Some(Palette {
            background,
            foreground,
            cursor: cfg.get("cursor").map(|v| unhex(&v)),
            selection_background: cfg.get("selection_background").map(|v| unhex(&v)),
            selection_foreground: cfg.get("selection_foreground").map(|v| unhex(&v)),
            ansi,
        }))
    }

    fn apply_theme(&self, theme: &Theme) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Space)?;

        cfg.set("background", &hex(&theme.palette.background));
        cfg.set("foreground", &hex(&theme.palette.foreground));
        if let Some(cursor) = &theme.palette.cursor {
            cfg.set("cursor", &hex(cursor));
        }
        if let Some(bg) = &theme.palette.selection_background {
            cfg.set("selection_background", &hex(bg));
        }
        if let Some(fg) = &theme.palette.selection_foreground {
            cfg.set("selection_foreground", &hex(fg));
        }

        for (i, value) in theme.palette.ansi.iter().enumerate() {
            cfg.set(&format!("color{i}"), &hex(value));
        }

        cfg.save(&path)
    }

    fn apply_font(&self, font: &FontSettings) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Space)?;
        cfg.set("font_family", &font.family);
        cfg.set("font_size", &font.size.to_string());
        cfg.save(&path)
    }

    fn apply_opacity(&self, opacity: f32) -> Result<(), String> {
        let path = self.config_path()?;
        let mut cfg = KvConfig::load(&path, KvSyntax::Space)?;
        cfg.set("background_opacity", &opacity.to_string());
        cfg.save(&path)
    }
}
