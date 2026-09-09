use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    Dark,
    Light,
    Both,
}

/// Time-of-day bucket for picking a theme, per the spec's "which are best
/// for day/night" list. `Day` accepts Light or Both, `Night` accepts Dark
/// or Both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Day,
    Night,
}

impl Period {
    pub fn accepts(self, variant: ThemeVariant) -> bool {
        matches!(
            (self, variant),
            (Period::Day, ThemeVariant::Light | ThemeVariant::Both)
                | (Period::Night, ThemeVariant::Dark | ThemeVariant::Both)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeSource {
    BuiltIn,
    UserInstalled,
}

/// ANSI colors 0-15 as hex strings without a leading `#`: black, red, green,
/// yellow, blue, magenta, cyan, white, then the bright variants of each.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub background: String,
    pub foreground: String,
    pub cursor: Option<String>,
    pub selection_background: Option<String>,
    pub selection_foreground: Option<String>,
    pub ansi: [String; 16],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub variant: ThemeVariant,
    pub source: ThemeSource,
    /// Present when `source` is `UserInstalled`.
    pub git_url: Option<String>,
    pub palette: Palette,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
}
