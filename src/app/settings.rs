//! Small persisted preferences (last export choices) in the user config dir.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wvs::export::ExportOptions;
use wvs::i18n::Lang;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub export: ExportOptions,
    /// Last chosen export base folder.
    pub export_base: Option<PathBuf>,
    /// Create `<name>_<timestamp>` sub-folder (default on).
    pub export_subdir: bool,
    /// UI / export language (None = default 繁體中文).
    pub lang: Option<Lang>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { export: ExportOptions::default(), export_base: None, export_subdir: true, lang: None }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

/// `<config>/whitebox-video-storyboard`.
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home_dir().map(|h| h.join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home_dir().map(|h| h.join(".config")))
    }?;
    Some(base.join("whitebox-video-storyboard"))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("settings.json"))
}

/// Folder holding the user's saved scene templates (`*.json`).
pub fn templates_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("templates"))
}

impl Settings {
    pub fn load() -> Settings {
        config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = config_path() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default());
        }
    }
}

/// Open a folder in the platform file manager.
pub fn open_folder(dir: &std::path::Path) {
    let cmd = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}
