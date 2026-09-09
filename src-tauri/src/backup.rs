use crate::apps::TerminalApp;
use crate::config_dir::sheets_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One-level undo per app: each call to `snapshot` overwrites the previous
/// backup with whatever was on disk right before the write about to happen,
/// so `undo` always reverts exactly the last change, not a deeper history.
#[derive(Serialize, Deserialize)]
struct Backup {
    /// Whether the config file existed before the change being backed up.
    existed: bool,
    content: Option<String>,
}

fn backup_path(app: TerminalApp) -> Result<PathBuf, String> {
    Ok(sheets_data_dir()?.join("backups").join(format!("{}.json", app.as_str())))
}

/// Records the current on-disk state of `config_path` as the undo point for
/// `app`, replacing any earlier one. Call this immediately before writing a
/// new theme/font/opacity to that app's config.
pub fn snapshot(app: TerminalApp, config_path: &Path) -> Result<(), String> {
    let backup = if config_path.exists() {
        Backup {
            existed: true,
            content: Some(fs::read_to_string(config_path).map_err(|e| e.to_string())?),
        }
    } else {
        Backup { existed: false, content: None }
    };

    let path = backup_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, serde_json::to_string(&backup).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn has_backup(app: TerminalApp) -> bool {
    backup_path(app).map(|p| p.exists()).unwrap_or(false)
}

/// Restores `config_path` to the state captured by the last `snapshot` for
/// `app` (removing the file if it didn't exist before that change), then
/// consumes the backup so a second `undo` in a row has nothing to do.
pub fn undo(app: TerminalApp, config_path: &Path) -> Result<(), String> {
    let path = backup_path(app)?;
    let contents = fs::read_to_string(&path).map_err(|_| "nothing to undo".to_string())?;
    let backup: Backup = serde_json::from_str(&contents).map_err(|e| e.to_string())?;

    if backup.existed {
        fs::write(config_path, backup.content.unwrap_or_default()).map_err(|e| e.to_string())?;
    } else if config_path.exists() {
        fs::remove_file(config_path).map_err(|e| e.to_string())?;
    }

    fs::remove_file(&path).map_err(|e| e.to_string())
}
