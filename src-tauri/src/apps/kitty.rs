use super::{ConfigAdapter, CurrentConfig};
use crate::config_dir::xdg_config_home;
use crate::kv_config::{KvConfig, KvSyntax};
use crate::theme::{FontSettings, Theme};
use std::path::PathBuf;

pub struct KittyAdapter;

fn hex(value: &str) -> String {
    format!("#{value}")
}

impl ConfigAdapter for KittyAdapter {
    fn config_path(&self) -> Result<PathBuf, String> {
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
