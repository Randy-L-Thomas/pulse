use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub height: u32,
    pub half_width: u32,
    pub full_width: u32,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub stale_secs: u64,
    pub net: NetCfg,
    pub path: PathCfg,
    pub disk: DiskCfg,
    pub http: Vec<HttpCfg>,
    pub file: Vec<FileCfg>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetCfg {
    pub host: String,
    pub https_url: String,
    pub history: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathCfg {
    pub dns_host: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiskCfg {
    pub mount: String,
    pub warn_pct: f64,
    pub crit_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpCfg {
    pub id: String,
    pub label: String,
    pub url: String,
    pub also: Option<String>,
    pub open: Option<String>,
    pub open_fallback: Option<String>,
    pub start_program: Option<String>,
    pub start_args: Option<Vec<String>>,
    pub start_cwd: Option<String>,
    pub stop_program: Option<String>,
    pub stop_args: Option<Vec<String>>,
    pub restart_program: Option<String>,
    pub restart_args: Option<Vec<String>>,
    pub task: Option<String>,
    pub stdio_match: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileCfg {
    pub id: String,
    pub label: String,
    pub path: String,
    pub bool_field: Option<String>,
    pub string_field: Option<String>,
    pub ok_value: Option<String>,
    pub open: Option<String>,
}

pub fn load_config() -> Result<Config, String> {
    let path = find_probes_toml().ok_or("probes.toml not found")?;
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&raw).map_err(|e| format!("parse probes.toml: {e}"))
}

fn find_probes_toml() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("probes.toml"));
        candidates.push(cwd.join("..").join("probes.toml"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("probes.toml"));
            candidates.push(dir.join("..").join("probes.toml"));
            candidates.push(dir.join("resources").join("probes.toml"));
        }
    }
    candidates.push(PathBuf::from(r"C:\dev\pulse\probes.toml"));
    candidates.into_iter().find(|p| Path::new(p).is_file())
}
