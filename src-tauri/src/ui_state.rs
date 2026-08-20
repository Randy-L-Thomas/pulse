use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default = "default_module")]
    pub last_module: String,
    #[serde(default = "default_wa")]
    pub wa_title: String,
    #[serde(default = "default_from")]
    pub mt_from: String,
    #[serde(default = "default_to")]
    pub mt_to: String,
    #[serde(default)]
    pub mt_enrich: bool,
    #[serde(default)]
    pub ollama_model: String,
    #[serde(default = "default_ollama")]
    pub ollama_url: String,
}

fn default_module() -> String {
    "translate".into()
}
fn default_wa() -> String {
    "WhatsApp".into()
}
fn default_from() -> String {
    "es".into()
}
fn default_to() -> String {
    "en".into()
}
fn default_ollama() -> String {
    "http://127.0.0.1:11434".into()
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            last_module: default_module(),
            wa_title: default_wa(),
            mt_from: default_from(),
            mt_to: default_to(),
            mt_enrich: false,
            ollama_model: String::new(),
            ollama_url: default_ollama(),
        }
    }
}

fn path() -> PathBuf {
    crate::config::user_config_dir().join("ui.json")
}

pub fn load() -> UiState {
    let p = path();
    let Ok(raw) = fs::read_to_string(&p) else {
        return UiState::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(state: &UiState) -> Result<(), String> {
    let dir = crate::config::user_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path(), raw).map_err(|e| e.to_string())
}
