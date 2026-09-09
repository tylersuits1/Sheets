mod alacritty;
mod ghostty;
mod kitty;

use crate::theme::{FontSettings, Palette, Theme};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminalApp {
    Ghostty,
    Kitty,
    Alacritty,
}

impl TerminalApp {
    pub const ALL: [TerminalApp; 3] = [TerminalApp::Ghostty, TerminalApp::Kitty, TerminalApp::Alacritty];

    pub fn adapter(self) -> Box<dyn ConfigAdapter> {
        match self {
            TerminalApp::Ghostty => Box::new(ghostty::GhosttyAdapter),
            TerminalApp::Kitty => Box::new(kitty::KittyAdapter),
            TerminalApp::Alacritty => Box::new(alacritty::AlacrittyAdapter),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentConfig {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub opacity: Option<f32>,
}

pub trait ConfigAdapter {
    fn config_path(&self) -> Result<PathBuf, String>;

    fn is_installed(&self) -> bool {
        self.config_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn read_current(&self) -> Result<CurrentConfig, String>;

    /// The colors currently configured, if a full palette (background,
    /// foreground, and all 16 ANSI colors) is set. Used to reverse-match
    /// against known themes; `Ok(None)` means no theme has been applied
    /// yet, or the config only sets some colors.
    fn read_current_palette(&self) -> Result<Option<Palette>, String>;

    fn apply_theme(&self, theme: &Theme) -> Result<(), String>;
    fn apply_font(&self, font: &FontSettings) -> Result<(), String>;
    fn apply_opacity(&self, opacity: f32) -> Result<(), String>;
}
