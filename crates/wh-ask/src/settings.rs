use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AskError;

pub const DEFAULT_TOP_K: u32 = 8;
pub const DEFAULT_RESERVE: u32 = 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default)]
    pub bundle: Option<String>,
    #[serde(default)]
    pub chat_model: Option<String>,
    #[serde(default)]
    pub embed_model: Option<String>,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_reserve")]
    pub context_reserve: u32,
}

fn default_top_k() -> u32 {
    DEFAULT_TOP_K
}

fn default_reserve() -> u32 {
    DEFAULT_RESERVE
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            bundle: None,
            chat_model: None,
            embed_model: None,
            top_k: DEFAULT_TOP_K,
            context_reserve: DEFAULT_RESERVE,
        }
    }
}

pub fn default_config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("wh-ask"))
        .unwrap_or_else(|| PathBuf::from("wh-ask"))
}

pub fn load_settings(dir: &Path) -> Result<Settings, AskError> {
    let path = dir.join("settings.json");
    if !path.exists() {
        return Ok(Settings::default());
    }
    let bytes =
        fs::read(&path).map_err(|err| AskError::model(format!("{}: {err}", path.display())))?;
    let mut settings: Settings = serde_json::from_slice(&bytes)
        .map_err(|err| AskError::model(format!("{}: malformed JSON: {err}", path.display())))?;
    if !(1..=32).contains(&settings.top_k) {
        settings.top_k = DEFAULT_TOP_K;
    }
    Ok(settings)
}

pub fn save_settings(dir: &Path, settings: &Settings) -> Result<(), AskError> {
    fs::create_dir_all(dir).map_err(|err| AskError::model(format!("{}: {err}", dir.display())))?;
    let path = dir.join("settings.json");
    let mut bytes = serde_json::to_vec_pretty(settings)
        .map_err(|err| AskError::model(format!("could not write settings: {err}")))?;
    bytes.push(b'\n');
    let temp = dir.join("settings.json.tmp");
    fs::write(&temp, bytes).map_err(|err| AskError::model(format!("{}: {err}", temp.display())))?;
    fs::rename(&temp, &path)
        .map_err(|err| AskError::model(format!("{}: {err}", path.display())))?;
    Ok(())
}

/// Make a user-entered path absolute without requiring the file to exist.
pub fn absolute_path(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    joined.canonicalize().unwrap_or(joined)
}
