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
    #[serde(default = "default_font_px")]
    pub font_px: u32,
    #[serde(default)]
    pub cell_order: Vec<String>,
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
fn default_font_px() -> u32 {
    13
}

pub const FONT_PX_MIN: u32 = 11;
pub const FONT_PX_MAX: u32 = 22;

pub fn clamp_font_px(px: u32) -> u32 {
    px.clamp(FONT_PX_MIN, FONT_PX_MAX)
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
            font_px: default_font_px(),
            cell_order: Vec::new(),
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
    let mut state: UiState = serde_json::from_str(&raw).unwrap_or_default();
    state.font_px = clamp_font_px(state.font_px);
    state
}

pub fn save(state: &UiState) -> Result<(), String> {
    let dir = crate::config::user_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path(), raw).map_err(|e| e.to_string())
}
