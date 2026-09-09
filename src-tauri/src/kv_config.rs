use std::fs;
use std::path::Path;

const MANAGED_MARKER: &str = "# --- Managed by Sheets: everything below this line is regenerated on each apply ---";

/// The two flat config syntaxes used by Ghostty (`key = value`) and Kitty
/// (`key value`).
#[derive(Clone, Copy)]
pub enum KvSyntax {
    Equals,
    Space,
}

impl KvSyntax {
    fn parse(self, line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let (key, value) = match self {
            KvSyntax::Equals => trimmed.split_once('=')?,
            KvSyntax::Space => trimmed.split_once(char::is_whitespace)?,
        };
        Some((key.trim().to_string(), value.trim().to_string()))
    }

    fn format(self, key: &str, value: &str) -> String {
        match self {
            KvSyntax::Equals => format!("{key} = {value}"),
            KvSyntax::Space => format!("{key} {value}"),
        }
    }
}

/// Edits a flat `key = value` / `key value` config file (Ghostty, Kitty)
/// without disturbing anything the user wrote by hand: everything Sheets
/// sets lives in one "managed" block appended at the end of the file, which
/// is fully regenerated on every apply. The user's own lines above that
/// block, including any pre-existing settings Sheets doesn't manage, are
/// left untouched.
pub struct KvConfig {
    prefix_lines: Vec<String>,
    managed_lines: Vec<String>,
    syntax: KvSyntax,
}

impl KvConfig {
    pub fn load(path: &Path, syntax: KvSyntax) -> Result<Self, String> {
        let contents = if path.exists() {
            fs::read_to_string(path).map_err(|e| e.to_string())?
        } else {
            String::new()
        };
        let mut lines: Vec<String> = contents.lines().map(str::to_string).collect();

        let managed_lines = match lines.iter().position(|l| l == MANAGED_MARKER) {
            Some(pos) => {
                let managed = lines.split_off(pos + 1);
                lines.pop(); // drop the marker line itself, now at the end of `lines`
                while lines.last().is_some_and(|l| l.trim().is_empty()) {
                    lines.pop();
                }
                managed
            }
            None => Vec::new(),
        };

        Ok(Self { prefix_lines: lines, managed_lines, syntax })
    }

    /// Removes every managed line for `key` (there may be more than one,
    /// e.g. Ghostty's repeated `palette` key).
    pub fn unset(&mut self, key: &str) {
        let syntax = self.syntax;
        self.managed_lines
            .retain(|l| syntax.parse(l).map(|(k, _)| k).as_deref() != Some(key));
    }

    /// Replaces every managed occurrence of `key` with a single new value.
    pub fn set(&mut self, key: &str, value: &str) {
        self.unset(key);
        self.managed_lines.push(self.syntax.format(key, value));
    }

    /// Appends another managed line for `key` without removing prior ones.
    /// Use with `unset` for keys that repeat, like Ghostty's `palette`.
    pub fn push(&mut self, key: &str, value: &str) {
        self.managed_lines.push(self.syntax.format(key, value));
    }

    /// Looks up `key`, preferring the value Sheets last applied and falling
    /// back to whatever the user had configured before that.
    pub fn get(&self, key: &str) -> Option<String> {
        self.managed_lines
            .iter()
            .rev()
            .find_map(|l| self.syntax.parse(l).and_then(|(k, v)| (k == key).then_some(v)))
            .or_else(|| {
                self.prefix_lines
                    .iter()
                    .rev()
                    .find_map(|l| self.syntax.parse(l).and_then(|(k, v)| (k == key).then_some(v)))
            })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut out = self.prefix_lines.clone();
        if !self.managed_lines.is_empty() {
            out.push(String::new());
            out.push(MANAGED_MARKER.to_string());
            out.extend(self.managed_lines.iter().cloned());
        }
        let mut contents = out.join("\n");
        contents.push('\n');

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, contents).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(path: &Path) -> KvConfig {
        KvConfig::load(path, KvSyntax::Equals).unwrap()
    }

    #[test]
    fn set_then_get_returns_the_new_value() {
        let mut cfg = KvConfig { prefix_lines: vec![], managed_lines: vec![], syntax: KvSyntax::Equals };
        cfg.set("background", "1a1b26");
        assert_eq!(cfg.get("background"), Some("1a1b26".to_string()));
    }

    #[test]
    fn set_replaces_rather_than_duplicates() {
        let mut cfg = KvConfig { prefix_lines: vec![], managed_lines: vec![], syntax: KvSyntax::Equals };
        cfg.set("background", "111111");
        cfg.set("background", "222222");
        assert_eq!(cfg.managed_lines.len(), 1);
        assert_eq!(cfg.get("background"), Some("222222".to_string()));
    }

    #[test]
    fn push_allows_repeated_keys_like_ghostty_palette() {
        let mut cfg = KvConfig { prefix_lines: vec![], managed_lines: vec![], syntax: KvSyntax::Equals };
        cfg.unset("palette");
        cfg.push("palette", "0=000000");
        cfg.push("palette", "1=ff0000");
        assert_eq!(cfg.managed_lines.len(), 2);
    }

    #[test]
    fn save_then_load_preserves_user_lines_and_managed_values() {
        let dir = std::env::temp_dir().join(format!("kv-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "keybind = ctrl+c=copy\n# a user comment\n").unwrap();

        let mut cfg = roundtrip(&path);
        cfg.set("font-size", "13");
        cfg.save(&path).unwrap();

        let reloaded = roundtrip(&path);
        assert_eq!(reloaded.get("font-size"), Some("13".to_string()));
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("keybind = ctrl+c=copy"));
        assert!(contents.contains("# a user comment"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
