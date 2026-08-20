use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const PRESET_DEFAULT: &str = include_str!("../../presets/probes.default.toml");
pub const PRESET_TK421: &str = include_str!("../../presets/probes.tk421.toml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub height: u32,
    pub half_width: u32,
    pub full_width: u32,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub stale_secs: u64,
    #[serde(default)]
    pub show: ShowCfg,
    pub net: NetCfg,
    pub path: PathCfg,
    #[serde(default)]
    pub cpu: UtilCfg,
    #[serde(default)]
    pub memory: UtilCfg,
    #[serde(default)]
    pub gpu: GpuCfg,
    /// Legacy single Cursor block. Migrated into `process` on load.
    #[serde(default, skip_serializing)]
    pub cursor: Option<CursorCfg>,
    #[serde(default)]
    pub http: Vec<HttpCfg>,
    #[serde(default)]
    pub file: Vec<FileCfg>,
    #[serde(default)]
    pub process: Vec<ProcessCfg>,
    /// Programs Pulse may Start/Stop/Restart. Empty = those spokes stay off.
    #[serde(default)]
    pub launch_allow: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShowCfg {
    #[serde(default = "default_true")]
    pub path: bool,
    #[serde(default = "default_true")]
    pub cpu: bool,
    #[serde(default = "default_true")]
    pub memory: bool,
    #[serde(default = "default_true")]
    pub gpu: bool,
}

impl Default for ShowCfg {
    fn default() -> Self {
        Self {
            path: true,
            cpu: true,
            memory: true,
            gpu: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetCfg {
    pub host: String,
    pub https_url: String,
    pub history: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathCfg {
    pub dns_host: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UtilCfg {
    pub warn_pct: f64,
    pub crit_pct: f64,
}

impl Default for UtilCfg {
    fn default() -> Self {
        Self {
            warn_pct: 80.0,
            crit_pct: 95.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GpuCfg {
    pub warn_pct: f64,
    pub crit_pct: f64,
    #[serde(default = "default_vram_warn")]
    pub vram_warn_pct: f64,
    #[serde(default = "default_vram_crit")]
    pub vram_crit_pct: f64,
}

impl Default for GpuCfg {
    fn default() -> Self {
        Self {
            warn_pct: 80.0,
            crit_pct: 95.0,
            vram_warn_pct: default_vram_warn(),
            vram_crit_pct: default_vram_crit(),
        }
    }
}

fn default_vram_warn() -> f64 {
    80.0
}

fn default_vram_crit() -> f64 {
    95.0
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CursorCfg {
    #[serde(default = "default_cursor_warn_gb")]
    pub warn_gb: f64,
    #[serde(default = "default_cursor_crit_gb")]
    pub crit_gb: f64,
}

fn default_cursor_warn_gb() -> f64 {
    3.0
}

fn default_cursor_crit_gb() -> f64 {
    4.0
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessCfg {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub exe_name: String,
    #[serde(default)]
    pub path_contains: Vec<String>,
    #[serde(default)]
    pub exclude_names: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default = "default_cursor_warn_gb")]
    pub warn_gb: f64,
    #[serde(default = "default_cursor_crit_gb")]
    pub crit_gb: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_exe: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpCfg {
    pub id: String,
    pub label: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub also: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_fallback: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdio_match: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileCfg {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bool_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
}

pub fn user_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pulse")
}

pub fn user_config_path() -> PathBuf {
    user_config_dir().join("probes.toml")
}

pub fn autostart_inited_path() -> PathBuf {
    user_config_dir().join("autostart-inited")
}

pub fn load_config() -> Result<Config, String> {
    let path = user_config_path();
    if !path.is_file() {
        let raw = first_run_preset();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
        fs::write(&path, raw).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    load_from_path(&path)
}

pub fn load_from_path(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse_config(&raw)
}

pub fn parse_config(raw: &str) -> Result<Config, String> {
    let mut cfg: Config = toml::from_str(raw).map_err(|e| format!("parse probes.toml: {e}"))?;
    migrate_legacy_cursor(&mut cfg);
    migrate_launch_allow(&mut cfg);
    Ok(cfg)
}

pub fn save_config(cfg: &Config) -> Result<PathBuf, String> {
    let path = user_config_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let raw = toml::to_string_pretty(cfg).map_err(|e| format!("serialize probes.toml: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

pub fn apply_preset(name: &str) -> Result<Config, String> {
    let raw = match name {
        "tk421" => PRESET_TK421,
        "generic" | "default" => PRESET_DEFAULT,
        _ => return Err(format!("unknown preset {name}")),
    };
    let cfg = parse_config(raw)?;
    save_config(&cfg)?;
    Ok(cfg)
}

fn first_run_preset() -> &'static str {
    if Path::new(r"C:\dev\cam").is_dir() {
        PRESET_TK421
    } else {
        PRESET_DEFAULT
    }
}

fn migrate_launch_allow(cfg: &mut Config) {
    if !cfg.launch_allow.is_empty() {
        return;
    }
    let defaults = ["cam", "ice", "npm"];
    let uses_default = cfg.http.iter().any(|h| {
        h.start_program
            .as_deref()
            .map(program_base)
            .is_some_and(|b| defaults.contains(&b.as_str()))
    });
    if uses_default {
        cfg.launch_allow = defaults.iter().map(|s| (*s).to_string()).collect();
    }
}

pub fn program_base(program: &str) -> String {
    program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .trim_end_matches(".CMD")
        .trim_end_matches(".bat")
        .to_ascii_lowercase()
}

pub fn program_allowed(program: &str, allow: &[String]) -> bool {
    let base = program_base(program);
    allow.iter().any(|a| program_base(a) == base)
}

fn migrate_legacy_cursor(cfg: &mut Config) {
    if !cfg.process.is_empty() {
        cfg.cursor = None;
        return;
    }
    if let Some(cur) = cfg.cursor.take() {
        cfg.process.push(ProcessCfg {
            id: "cursor".into(),
            label: "Cursor".into(),
            exe_name: "cursor".into(),
            path_contains: vec![
                r"\programs\cursor\".into(),
                r"\program files\cursor\".into(),
                r"\program files (x86)\cursor\".into(),
            ],
            exclude_names: vec!["pulse".into(), "code".into()],
            exclude_paths: vec![r"\microsoft vs code".into(), r"\vscode\".into()],
            warn_gb: cur.warn_gb,
            crit_gb: cur.crit_gb,
            open_exe: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_parses() {
        let cfg = parse_config(PRESET_DEFAULT).unwrap();
        assert!(cfg.http.is_empty());
        assert!(cfg.process.is_empty());
        assert!(cfg.show.cpu);
    }

    #[test]
    fn tk421_preset_parses() {
        let cfg = parse_config(PRESET_TK421).unwrap();
        assert!(cfg.http.iter().any(|h| h.id == "cam"));
        assert!(cfg.process.iter().any(|p| p.id == "cursor"));
        assert!(cfg.launch_allow.iter().any(|p| p == "cam"));
    }

    #[test]
    fn generic_preset_has_empty_allowlist() {
        let cfg = parse_config(PRESET_DEFAULT).unwrap();
        assert!(cfg.launch_allow.is_empty());
    }
}
