use crate::config::{Config, FileCfg, HttpCfg};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use sysinfo::Disks;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Degraded,
    Down,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub id: String,
    pub label: String,
    pub status: Status,
    pub primary: String,
    pub detail: String,
    pub copy_text: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetState {
    pub status: Status,
    pub icmp_ms: Option<f64>,
    pub https_ms: Option<f64>,
    pub loss_pct: f64,
    pub history: Vec<f64>,
    pub detail: String,
    pub copy_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub cells: Vec<Cell>,
    pub net: NetState,
}

pub struct ProbeState {
    pub history: Mutex<Vec<f64>>,
    pub pings: Mutex<Vec<bool>>,
}

impl ProbeState {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            pings: Mutex::new(Vec::new()),
        }
    }
}

pub fn empty_snapshot() -> Snapshot {
    Snapshot {
        cells: Vec::new(),
        net: NetState {
            status: Status::Degraded,
            icmp_ms: None,
            https_ms: None,
            loss_pct: 0.0,
            history: Vec::new(),
            detail: "probing…".into(),
            copy_text: String::new(),
        },
    }
}

pub async fn collect(cfg: &Config, state: &ProbeState, client: &reqwest::Client) -> Snapshot {
    let net_f = probe_net(cfg, state, client);
    let path_f = probe_path(cfg);
    let disk_f = async { probe_disk(cfg) };
    let http_f = probe_http_all(cfg, client);
    let files_f = async { probe_files(cfg) };
    let (net, path, disk, https, files) = tokio::join!(net_f, path_f, disk_f, http_f, files_f);
    let mut cells = Vec::new();
    cells.push(path);
    cells.extend(https);
    cells.push(disk);
    cells.extend(files);
    Snapshot { cells, net }
}

async fn probe_net(cfg: &Config, state: &ProbeState, client: &reqwest::Client) -> NetState {
    let host = cfg.net.host.clone();
    let icmp = tokio::task::spawn_blocking(move || ping_ms(&host)).await.unwrap_or(None);
    let https = https_ms(client, &cfg.net.https_url).await;

    {
        let mut hist = state.history.lock().unwrap();
        if let Some(ms) = icmp.or(https) {
            hist.push(ms);
        }
        let keep = cfg.net.history.max(8);
        if hist.len() > keep {
            let drain = hist.len() - keep;
            hist.drain(0..drain);
        }
    }
    {
        let mut pings = state.pings.lock().unwrap();
        pings.push(icmp.is_some());
        if pings.len() > 20 {
            pings.remove(0);
        }
    }

    let sent = state.pings.lock().unwrap();
    let ok_n = sent.iter().filter(|x| **x).count();
    let loss = if sent.is_empty() {
        100.0
    } else {
        (1.0 - ok_n as f64 / sent.len() as f64) * 100.0
    };
    drop(sent);

    let history = state.history.lock().unwrap().clone();
    let status = if icmp.is_none() && https.is_none() {
        Status::Down
    } else if loss > 20.0 || icmp.map(|m| m > 120.0).unwrap_or(true) {
        Status::Degraded
    } else {
        Status::Ok
    };

    let icmp_s = icmp.map(|m| format!("{m:.0}ms")).unwrap_or_else(|| "timeout".into());
    let https_s = https.map(|m| format!("{m:.0}ms")).unwrap_or_else(|| "fail".into());
    let detail = format!("icmp {icmp_s}  tls {https_s}  loss {loss:.0}%");
    let copy_text = format!(
        "icmp={} https={} loss={:.0}%",
        icmp.map(|m| format!("{m:.1}ms")).unwrap_or_else(|| "timeout".into()),
        https.map(|m| format!("{m:.1}ms")).unwrap_or_else(|| "fail".into()),
        loss
    );

    NetState {
        status,
        icmp_ms: icmp,
        https_ms: https,
        loss_pct: loss,
        history,
        detail,
        copy_text,
    }
}

async fn probe_path(cfg: &Config) -> Cell {
    let host = cfg.path.dns_host.clone();
    let dns = dns_ms(&format!("{host}:443")).await;
    let gw = tokio::task::spawn_blocking(probe_gateway).await.unwrap_or(None);
    let (gw_ip, gw_ms) = gw.unwrap_or((None, None));

    let status = match (dns, gw_ms) {
        (Some(d), Some(g)) if d < 80.0 && g < 40.0 => Status::Ok,
        (Some(_), Some(_)) => Status::Degraded,
        (Some(_), None) | (None, Some(_)) => Status::Degraded,
        (None, None) => Status::Down,
    };
    let dns_s = dns.map(|m| format!("{m:.0}")).unwrap_or_else(|| "—".into());
    let gw_s = gw_ms.map(|m| format!("{m:.0}")).unwrap_or_else(|| "—".into());
    let gw_label = gw_ip.unwrap_or_else(|| "gw".into());
    Cell {
        id: "path".into(),
        label: "Path".into(),
        status,
        primary: format!("{dns_s}/{gw_s}"),
        detail: format!("dns ms / {gw_label} ms"),
        copy_text: format!("dns={}ms gateway={}ms ({})", dns_s, gw_s, gw_label),
        actions: vec![
            Action { id: "open".into(), label: "Open".into(), enabled: false },
            Action { id: "start".into(), label: "Start".into(), enabled: false },
            Action { id: "stop".into(), label: "Stop".into(), enabled: false },
            Action { id: "restart".into(), label: "Restart".into(), enabled: false },
            Action { id: "copy".into(), label: "Copy".into(), enabled: true },
        ],
    }
}

fn probe_disk(cfg: &Config) -> Cell {
    let disks = Disks::new_with_refreshed_list();
    let want = Path::new(&cfg.disk.mount);
    let found = disks.iter().find(|d| same_mount(d.mount_point(), want));
    let Some(disk) = found else {
        return Cell {
            id: "disk".into(),
            label: "Disk".into(),
            status: Status::Down,
            primary: "n/a".into(),
            detail: format!("no {}", cfg.disk.mount),
            copy_text: format!("disk {} missing", cfg.disk.mount),
            actions: copy_only(),
        };
    };
    let total = disk.total_space() as f64;
    let avail = disk.available_space() as f64;
    let pct = if total > 0.0 { (avail / total) * 100.0 } else { 0.0 };
    let gb = avail / 1_073_741_824.0;
    let status = if pct < cfg.disk.crit_pct {
        Status::Down
    } else if pct < cfg.disk.warn_pct {
        Status::Degraded
    } else {
        Status::Ok
    };
    Cell {
        id: "disk".into(),
        label: "Disk".into(),
        status,
        primary: format!("{gb:.0} GB"),
        detail: format!("{pct:.0}% free on {}", cfg.disk.mount),
        copy_text: format!("{} {:.1} GB free ({:.1}%)", cfg.disk.mount, gb, pct),
        actions: copy_only(),
    }
}

async fn probe_http_all(cfg: &Config, client: &reqwest::Client) -> Vec<Cell> {
    let mut out = Vec::new();
    for spec in &cfg.http {
        out.push(probe_http(spec, client).await);
    }
    out
}

async fn probe_http(spec: &HttpCfg, client: &reqwest::Client) -> Cell {
    let t = Instant::now();
    let res = client.get(&spec.url).send().await;
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let also_ok = if let Some(also) = &spec.also {
        client.get(also).send().await.map(|r| r.status().is_success()).unwrap_or(false)
    } else {
        true
    };

    let mut primary = format!("{ms:.0} ms");
    let mut detail;
    let mut status = Status::Down;
    let mut stdio_only = false;

    match res {
        Ok(resp) => {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if code.is_success() {
                status = if also_ok { Status::Ok } else { Status::Degraded };
                if let Ok(v) = serde_json::from_str::<Value>(&body) {
                    let restart_required = v.get("restart_required").and_then(|x| x.as_bool()) == Some(true);
                    if restart_required {
                        status = Status::Degraded;
                    }
                    let version = v.get("version").and_then(|x| x.as_str());
                    if spec.id == "ollama" {
                        let n = v.get("models").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                        primary = format!("{n}");
                        detail = format!("models · {ms:.0} ms");
                    } else if spec.id == "cam-mcp" {
                        let ver = v.get("mcp_version").and_then(|x| x.as_str()).unwrap_or("");
                        primary = if ver.is_empty() { "http".into() } else { ver.into() };
                        detail = format!("http · {ms:.0} ms");
                    } else if let Some(ver) = version {
                        primary = ver.to_string();
                        detail = format!("{ms:.0} ms");
                    } else {
                        detail = format!("{ms:.0} ms");
                    }
                    if !also_ok {
                        detail = format!("{detail} · ui down");
                    }
                    if restart_required {
                        detail = format!("{detail} · restart required");
                    }
                } else {
                    detail = format!("http {code} · {ms:.0} ms");
                    if !also_ok {
                        detail = format!("{detail} · ui down");
                    }
                }
            } else {
                detail = format!("http {code}");
            }
        }
        Err(e) => {
            detail = short_err(&e);
            if spec.stdio_match.is_some() && process_matches(spec.stdio_match.as_deref().unwrap()) {
                stdio_only = true;
                status = Status::Degraded;
                primary = "stdio".into();
                detail = "no HTTP — stdio only".into();
            }
        }
    }

    if matches!(status, Status::Down) && !stdio_only {
        primary = "down".into();
    }

    if let Some(task) = &spec.task {
        match task_last_result(task) {
            Some(0) | Some(267009) => {
                detail = format!("{detail} · task ok");
            }
            Some(code) => {
                if !matches!(status, Status::Down) {
                    status = Status::Degraded;
                }
                detail = format!("{detail} · task {code}");
            }
            None => {
                detail = format!("{detail} · task ?");
            }
        }
    }

    let has_open = spec.open.is_some();
    let has_start = spec.start_program.is_some();
    let has_stop = spec.stop_program.is_some();
    let has_restart = spec.restart_program.is_some();
    Cell {
        id: spec.id.clone(),
        label: spec.label.clone(),
        status,
        primary,
        detail: detail.clone(),
        copy_text: format!("{} {} {}", spec.label, spec.url, detail),
        actions: vec![
            Action { id: "open".into(), label: "Open".into(), enabled: has_open },
            Action { id: "start".into(), label: "Start".into(), enabled: has_start },
            Action { id: "stop".into(), label: "Stop".into(), enabled: has_stop },
            Action { id: "restart".into(), label: "Restart".into(), enabled: has_restart },
            Action { id: "copy".into(), label: "Copy".into(), enabled: true },
        ],
    }
}

fn probe_files(cfg: &Config) -> Vec<Cell> {
    cfg.file.iter().map(|f| probe_file(cfg, f)).collect()
}

fn probe_file(cfg: &Config, spec: &FileCfg) -> Cell {
    let path = Path::new(&spec.path);
    let actions = vec![
        Action { id: "open".into(), label: "Open".into(), enabled: spec.open.is_some() },
        Action { id: "start".into(), label: "Start".into(), enabled: false },
        Action { id: "stop".into(), label: "Stop".into(), enabled: false },
        Action { id: "restart".into(), label: "Restart".into(), enabled: false },
        Action { id: "copy".into(), label: "Copy".into(), enabled: true },
    ];

    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => {
            return Cell {
                id: spec.id.clone(),
                label: spec.label.clone(),
                status: Status::Down,
                primary: "missing".into(),
                detail: spec.path.clone(),
                copy_text: format!("{} missing {}", spec.label, spec.path),
                actions,
            };
        }
    };
    let age = meta
        .modified()
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .unwrap_or(Duration::from_secs(0));
    let stale = age.as_secs() > cfg.stale_secs;
    let age_s = format_age(age);
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let json: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

    let mut status = Status::Ok;
    let mut primary = age_s.clone();
    let mut detail = age_s.clone();

    if let Some(field) = &spec.bool_field {
        let ok = json.get(field).and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            status = Status::Down;
            primary = "fail".into();
            if let Some(checks) = json.get("checks").and_then(|c| c.as_array()) {
                if let Some(bad) = checks.iter().find(|c| c.get("status").and_then(|s| s.as_str()) != Some("ok")) {
                    detail = bad
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("check failed")
                        .to_string();
                }
            }
        } else {
            primary = "ok".into();
            detail = format!("age {age_s}");
        }
    }
    if let Some(field) = &spec.string_field {
        let val = json.get(field).and_then(|v| v.as_str()).unwrap_or("unknown");
        let ok_val = spec.ok_value.as_deref().unwrap_or("healthy");
        primary = val.to_string();
        if val != ok_val {
            status = Status::Degraded;
        }
        if let Some(sum) = json.get("operator_summary").and_then(|v| v.as_str()) {
            detail = sum.chars().take(42).collect();
        } else {
            detail = format!("age {age_s}");
        }
    }
    if stale && !matches!(status, Status::Down) {
        status = Status::Degraded;
        detail = format!("stale {age_s}");
    }

    Cell {
        id: spec.id.clone(),
        label: spec.label.clone(),
        status,
        primary,
        detail: detail.clone(),
        copy_text: format!("{} {} {}", spec.label, spec.path, detail),
        actions,
    }
}

fn copy_only() -> Vec<Action> {
    vec![
        Action { id: "open".into(), label: "Open".into(), enabled: false },
        Action { id: "start".into(), label: "Start".into(), enabled: false },
        Action { id: "stop".into(), label: "Stop".into(), enabled: false },
        Action { id: "restart".into(), label: "Restart".into(), enabled: false },
        Action { id: "copy".into(), label: "Copy".into(), enabled: true },
    ]
}

fn format_age(age: Duration) -> String {
    let s = age.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else {
        format!("{}h", s / 3600)
    }
}

fn short_err(e: &reqwest::Error) -> String {
    if e.is_connect() {
        "refused".into()
    } else if e.is_timeout() {
        "timeout".into()
    } else {
        "error".into()
    }
}

async fn https_ms(client: &reqwest::Client, url: &str) -> Option<f64> {
    let t = Instant::now();
    match client.get(url).send().await {
        Ok(_) => Some(t.elapsed().as_secs_f64() * 1000.0),
        Err(_) => None,
    }
}

async fn dns_ms(host_port: &str) -> Option<f64> {
    let t = Instant::now();
    match tokio::net::lookup_host(host_port).await {
        Ok(mut addrs) => {
            let _ = addrs.next();
            Some(t.elapsed().as_secs_f64() * 1000.0)
        }
        Err(_) => None,
    }
}

fn ping_ms(host: &str) -> Option<f64> {
    let mut cmd = Command::new("ping");
    #[cfg(windows)]
    {
        cmd.args(["-n", "1", "-w", "2000", host]);
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        cmd.args(["-c", "1", "-W", "2", host]);
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    parse_ping_ms(&text)
}

fn parse_ping_ms(text: &str) -> Option<f64> {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.split("time=").nth(1) {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(v) = num.parse::<f64>() {
                return Some(v);
            }
        }
        if let Some(rest) = lower.split("time<").nth(1) {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(v) = num.parse::<f64>() {
                return Some(v.max(0.5));
            }
        }
        if lower.contains("average") {
            if let Some(eq) = lower.rsplit('=').next() {
                let num: String = eq.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                if let Ok(v) = num.parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn probe_gateway() -> Option<(Option<String>, Option<f64>)> {
    let ip = default_gateway()?;
    let ms = ping_ms(&ip);
    Some((Some(ip), ms))
}

fn default_gateway() -> Option<String> {
    let mut cmd = Command::new("route");
    cmd.args(["print", "-4"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "0.0.0.0" && parts[1] == "0.0.0.0" {
            let gw = parts[2];
            if gw != "On-link" && gw != "0.0.0.0" {
                return Some(gw.to_string());
            }
        }
    }
    None
}

/// Last Task Result from `schtasks`. 0 = success, 267009 = still running (ok).
fn task_last_result(name: &str) -> Option<u32> {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/Query", "/TN", name, "/FO", "LIST", "/V"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let Some((key, val)) = line.split_once(':') else { continue };
        if key.trim().eq_ignore_ascii_case("Last Result") {
            let v = val.trim();
            if let Ok(n) = v.parse::<u32>() {
                return Some(n);
            }
            if let Some(hex) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
                return u32::from_str_radix(hex, 16).ok();
            }
        }
    }
    None
}

fn process_matches(needle: &str) -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.processes().values().any(|p| {
        let cmd = p
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        cmd.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
    })
}

fn same_mount(a: &Path, b: &Path) -> bool {
    let na = a.to_string_lossy().trim_end_matches(['\\', '/']).to_ascii_uppercase();
    let nb = b.to_string_lossy().trim_end_matches(['\\', '/']).to_ascii_uppercase();
    na == nb
}
