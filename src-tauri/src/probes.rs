use crate::config::{Config, FileCfg, GpuCfg, HttpCfg, ProcessCfg, UtilCfg};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, Serialize)]
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

#[derive(Clone, Copy)]
enum GpuKind {
    Unknown,
    Nvidia,
    Pdh,
    Missing,
}

pub struct ProbeState {
    pub history: Mutex<Vec<f64>>,
    pub pings: Mutex<Vec<bool>>,
    sys: Mutex<sysinfo::System>,
    cpu_ready: Mutex<bool>,
    gpu_kind: Mutex<GpuKind>,
}

impl ProbeState {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            pings: Mutex::new(Vec::new()),
            sys: Mutex::new(sysinfo::System::new()),
            cpu_ready: Mutex::new(false),
            gpu_kind: Mutex::new(GpuKind::Unknown),
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
    let host_f = async {
        let (cpu, ram) = probe_cpu_ram(cfg, state);
        let procs = probe_processes(cfg, state);
        (cpu, ram, procs)
    };
    let gpu_f = async { probe_gpu(cfg, state) };
    let http_f = probe_http_all(cfg, client);
    let files_f = async { probe_files(cfg) };
    let (net, path, host, gpu, https, files) = tokio::join!(net_f, path_f, host_f, gpu_f, http_f, files_f);
    let mut cells = Vec::new();
    if cfg.show.path {
        cells.push(path);
    }
    cells.extend(https);
    cells.extend(host.2);
    if cfg.show.cpu {
        cells.push(host.0);
    }
    if cfg.show.memory {
        cells.push(host.1);
    }
    if cfg.show.gpu {
        cells.push(gpu);
    }
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
    let gw_ip = gw_ip.unwrap_or_else(|| "unknown".into());
    let status_s = status_word(&status);
    Cell {
        id: "path".into(),
        label: "Path".into(),
        status,
        primary: format!("{dns_s}/{gw_s}"),
        detail: format!("{host} · {gw_ip}"),
        copy_text: format!("Path {status_s}\nDNS {host}: {dns_s} ms\ngateway {gw_ip}: {gw_s} ms"),
        actions: radial_actions(false, false, false, false),
    }
}

const GIB: f64 = 1_073_741_824.0;

fn util_status(pct: f64, cfg: &UtilCfg) -> Status {
    if pct >= cfg.crit_pct {
        Status::Down
    } else if pct >= cfg.warn_pct {
        Status::Degraded
    } else {
        Status::Ok
    }
}

fn probe_cpu_ram(cfg: &Config, state: &ProbeState) -> (Cell, Cell) {
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let mut ready = state.cpu_ready.lock().unwrap();
    if !*ready {
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_usage();
        *ready = true;
    }
    let cpu_pct = (sys.global_cpu_usage() as f64).clamp(0.0, 100.0);
    let cores = sys.cpus().len();
    let used = sys.used_memory() as f64;
    let total = sys.total_memory() as f64;
    drop(ready);
    drop(sys);

    let cpu_status = util_status(cpu_pct, &cfg.cpu);
    let cpu = Cell {
        id: "cpu".into(),
        label: "CPU".into(),
        status: cpu_status,
        primary: format!("{cpu_pct:.0}%"),
        detail: if cores == 0 {
            "sysinfo".into()
        } else {
            format!("{cores} cores")
        },
        copy_text: format!("CPU {cpu_pct:.1}% ({cores} cores)"),
        actions: copy_only(),
    };

    let mem_pct = if total > 0.0 {
        (used / total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let mem_status = util_status(mem_pct, &cfg.memory);
    let ram = Cell {
        id: "ram".into(),
        label: "RAM".into(),
        status: mem_status,
        primary: format!("{mem_pct:.0}%"),
        detail: format!("{:.0}/{:.0} GB", used / GIB, total / GIB),
        copy_text: format!(
            "RAM {mem_pct:.1}% ({:.1}/{:.1} GB)",
            used / GIB,
            total / GIB
        ),
        actions: copy_only(),
    };
    (cpu, ram)
}

fn probe_processes(cfg: &Config, state: &ProbeState) -> Vec<Cell> {
    if cfg.process.is_empty() {
        return Vec::new();
    }
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_memory();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
    );
    let total_ram = sys.total_memory();
    let self_pid = sysinfo::Pid::from_u32(std::process::id());
    let mut samples: Vec<(u64, usize)> = vec![(0, 0); cfg.process.len()];
    for p in sys.processes().values() {
        if p.pid() == self_pid {
            continue;
        }
        let name = p.name().to_string_lossy();
        let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
        for (i, spec) in cfg.process.iter().enumerate() {
            if matches_process(spec, &name, exe.as_deref()) {
                samples[i].0 = samples[i].0.saturating_add(p.memory());
                samples[i].1 += 1;
            }
        }
    }
    drop(sys);

    cfg.process
        .iter()
        .zip(samples)
        .map(|(spec, (bytes, count))| process_cell(spec, bytes, count, total_ram))
        .collect()
}

fn process_cell(spec: &ProcessCfg, bytes: u64, count: usize, total_ram: u64) -> Cell {
    let open_enabled = process_open_path(spec).is_some();
    let restart = is_cursor_spec(spec) && count > 0;
    let actions = radial_actions(open_enabled, false, false, restart);
    if count == 0 {
        return Cell {
            id: spec.id.clone(),
            label: spec.label.clone(),
            status: Status::Down,
            primary: "not running".into(),
            detail: "0 procs".into(),
            copy_text: format!(
                "{} red\nnot running (0 procs)\nwarn {:.1} GB, crit {:.1} GB",
                spec.label, spec.warn_gb, spec.crit_gb
            ),
            actions,
        };
    }
    let used_gb = bytes as f64 / GIB;
    let ram_pct = if total_ram > 0 {
        (bytes as f64 / total_ram as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let warn_bytes = (spec.warn_gb * GIB).max(0.0) as u64;
    let crit_bytes = (spec.crit_gb * GIB).max(0.0) as u64;
    let status = if bytes >= crit_bytes {
        Status::Down
    } else if bytes >= warn_bytes {
        Status::Degraded
    } else {
        Status::Ok
    };
    let proc_word = if count == 1 { "proc" } else { "procs" };
    let primary = format!("{used_gb:.1} GB");
    let detail = format!("{count} {proc_word} · {ram_pct:.0}% ram");
    Cell {
        id: spec.id.clone(),
        label: spec.label.clone(),
        status,
        primary: primary.clone(),
        detail: detail.clone(),
        copy_text: format!(
            "{} {}\n{primary} ({detail})\nwarn {:.1} GB, crit {:.1} GB",
            spec.label,
            status_word(&status),
            spec.warn_gb,
            spec.crit_gb
        ),
        actions,
    }
}

pub(crate) fn is_cursor_spec(spec: &ProcessCfg) -> bool {
    spec.id.eq_ignore_ascii_case("cursor") || spec.exe_name.eq_ignore_ascii_case("cursor")
}

pub(crate) fn process_open_path(spec: &ProcessCfg) -> Option<PathBuf> {
    if let Some(p) = spec.open_exe.as_deref().filter(|s| !s.is_empty()) {
        let expanded = expand_env_path(p);
        if PathBuf::from(&expanded).is_file() {
            return Some(PathBuf::from(expanded));
        }
    }
    if spec.id.eq_ignore_ascii_case("cursor") || spec.exe_name.eq_ignore_ascii_case("cursor") {
        return cursor_exe_path();
    }
    None
}

fn expand_env_path(p: &str) -> String {
    let mut out = p.to_string();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out = out.replace("%LOCALAPPDATA%", &local);
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        out = out.replace("%ProgramFiles%", &pf);
    }
    out
}

/// Well-known Cursor Desktop install. Used for Open and to enable the radial.
pub(crate) fn cursor_exe_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join(r"Programs\cursor\Cursor.exe"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(&pf).join(r"Cursor\Cursor.exe"));
        candidates.push(PathBuf::from(&pf).join(r"cursor\Cursor.exe"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(pf86).join(r"Cursor\Cursor.exe"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn matches_process(spec: &ProcessCfg, name: &str, exe: Option<&str>) -> bool {
    let name_n = normalize_proc_name(name);
    for ex in &spec.exclude_names {
        if name_n == normalize_proc_name(ex) {
            return false;
        }
    }
    if let Some(exe) = exe {
        let path = normalize_match_text(exe);
        for ex in &spec.exclude_paths {
            if path.contains(&normalize_match_text(ex)) {
                return false;
            }
        }
        for needle in &spec.path_contains {
            if path.contains(&normalize_match_text(needle)) {
                return true;
            }
        }
    }
    let want = normalize_proc_name(&spec.exe_name);
    !want.is_empty() && name_n == want
}

fn normalize_proc_name(name: &str) -> String {
    name.to_ascii_lowercase().trim_end_matches(".exe").to_string()
}

#[cfg(test)]
fn is_cursor_desktop_process(name: &str, exe: Option<&str>) -> bool {
    matches_process(&cursor_test_spec(), name, exe)
}

#[cfg(test)]
fn cursor_test_spec() -> ProcessCfg {
    ProcessCfg {
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
        warn_gb: 3.0,
        crit_gb: 4.0,
        open_exe: None,
    }
}

fn status_word(status: &Status) -> &'static str {
    match status {
        Status::Ok => "ok",
        Status::Degraded => "yellow",
        Status::Down => "red",
    }
}

fn spoke(id: &str, enabled: bool) -> Action {
    let label = match id {
        "open" => "Open",
        "start" => "Start",
        "stop" => "Stop",
        "restart" => "Restart",
        "info" => "Info",
        "copy" => "Copy",
        _ => id,
    };
    Action {
        id: id.into(),
        label: label.into(),
        enabled,
    }
}

/// Five radial spokes. Unused Restart becomes Info (toast + copy a longer readout).
fn radial_actions(open: bool, start: bool, stop: bool, restart: bool) -> Vec<Action> {
    vec![
        spoke("open", open),
        spoke("start", start),
        spoke("stop", stop),
        if restart {
            spoke("restart", true)
        } else {
            spoke("info", true)
        },
        spoke("copy", true),
    ]
}

struct GpuSample {
    util: f64,
    mem_used_mib: f64,
    mem_total_mib: f64,
    name: String,
}

fn probe_gpu(cfg: &Config, state: &ProbeState) -> Cell {
    let mut kind = state.gpu_kind.lock().unwrap();
    match *kind {
        GpuKind::Nvidia => match gpu_nvidia_smi() {
            NvProbe::Ok(sample) => gpu_cell(sample, &cfg.gpu),
            _ => gpu_unavailable("nvidia-smi fail"),
        },
        GpuKind::Pdh => {
            if let Some(sample) = gpu_pdh() {
                gpu_cell(sample, &cfg.gpu)
            } else {
                gpu_unavailable("gpu counter")
            }
        }
        GpuKind::Missing => match gpu_nvidia_smi() {
            NvProbe::Ok(sample) => {
                *kind = GpuKind::Nvidia;
                gpu_cell(sample, &cfg.gpu)
            }
            _ => gpu_unavailable("no gpu counter"),
        },
        GpuKind::Unknown => match gpu_nvidia_smi() {
            NvProbe::Ok(sample) => {
                *kind = GpuKind::Nvidia;
                gpu_cell(sample, &cfg.gpu)
            }
            NvProbe::Error => {
                *kind = GpuKind::Nvidia;
                gpu_unavailable("nvidia-smi fail")
            }
            NvProbe::NotFound => {
                if let Some(sample) = gpu_pdh() {
                    *kind = GpuKind::Pdh;
                    gpu_cell(sample, &cfg.gpu)
                } else {
                    *kind = GpuKind::Missing;
                    gpu_unavailable("no gpu counter")
                }
            }
        },
    }
}

fn worse(a: Status, b: Status) -> Status {
    match (a, b) {
        (Status::Down, _) | (_, Status::Down) => Status::Down,
        (Status::Degraded, _) | (_, Status::Degraded) => Status::Degraded,
        _ => Status::Ok,
    }
}

fn gpu_cell(sample: GpuSample, cfg: &GpuCfg) -> Cell {
    let pct = sample.util.clamp(0.0, 100.0);
    let vram_pct = if sample.mem_total_mib > 0.0 {
        (sample.mem_used_mib / sample.mem_total_mib * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let util_st = util_status(
        pct,
        &UtilCfg {
            warn_pct: cfg.warn_pct,
            crit_pct: cfg.crit_pct,
        },
    );
    let vram_st = util_status(
        vram_pct,
        &UtilCfg {
            warn_pct: cfg.vram_warn_pct,
            crit_pct: cfg.vram_crit_pct,
        },
    );
    let status = worse(util_st, vram_st);
    let vram = if sample.mem_total_mib > 0.0 {
        format!(
            "{:.1}/{:.0} GB",
            sample.mem_used_mib / 1024.0,
            sample.mem_total_mib / 1024.0
        )
    } else {
        "vram n/a".into()
    };
    let primary = format!("{pct:.0}%\n{vram}");
    let detail = if sample.name.is_empty() {
        format!("gpu {pct:.0}% · vram {vram_pct:.0}%")
    } else {
        format!("{} · vram {vram_pct:.0}%", sample.name.chars().take(18).collect::<String>())
    };
    Cell {
        id: "gpu".into(),
        label: "GPU".into(),
        status,
        primary,
        detail: detail.clone(),
        copy_text: if sample.name.is_empty() {
            format!("GPU util {pct:.1}%  vram {vram} ({vram_pct:.0}%)")
        } else {
            format!("GPU {} util {pct:.1}%  vram {vram} ({vram_pct:.0}%)", sample.name)
        },
        actions: copy_only(),
    }
}

fn gpu_unavailable(reason: &str) -> Cell {
    Cell {
        id: "gpu".into(),
        label: "GPU".into(),
        status: Status::Degraded,
        primary: "n/a".into(),
        detail: reason.into(),
        copy_text: format!("GPU unavailable ({reason})"),
        actions: copy_only(),
    }
}

enum NvProbe {
    Ok(GpuSample),
    Error,
    NotFound,
}

fn gpu_nvidia_smi() -> NvProbe {
    let output = match nvidia_smi_output() {
        Some(out) => out,
        None => return NvProbe::NotFound,
    };
    if !output.status.success() {
        return NvProbe::Error;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut best: Option<GpuSample> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(4, ',');
        let Some(util) = parts.next().and_then(parse_nv_num) else { continue };
        let mem_used = parts.next().and_then(parse_nv_num).unwrap_or(0.0);
        let mem_total = parts.next().and_then(parse_nv_num).unwrap_or(0.0);
        let name = parts.next().unwrap_or("").trim().to_string();
        let sample = GpuSample {
            util,
            mem_used_mib: mem_used,
            mem_total_mib: mem_total,
            name,
        };
        let take = match &best {
            None => true,
            Some(cur) => {
                sample.util > cur.util || (sample.util == cur.util && sample.mem_total_mib > cur.mem_total_mib)
            }
        };
        if take {
            best = Some(sample);
        }
    }
    match best {
        Some(sample) => NvProbe::Ok(sample),
        None => NvProbe::Error,
    }
}

fn parse_nv_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("[n/a]") || t.eq_ignore_ascii_case("n/a") {
        return None;
    }
    t.parse().ok()
}

fn nvidia_smi_output() -> Option<std::process::Output> {
    const ARGS: [&str; 2] = [
        "--query-gpu=utilization.gpu,memory.used,memory.total,name",
        "--format=csv,noheader,nounits",
    ];
    const PATHS: [&str; 3] = [
        "nvidia-smi",
        r"C:\Windows\System32\nvidia-smi.exe",
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
    ];
    for path in PATHS {
        let mut cmd = Command::new(path);
        cmd.args(ARGS);
        hide_window(&mut cmd);
        if let Ok(out) = cmd.output() {
            return Some(out);
        }
    }
    None
}

fn gpu_pdh() -> Option<GpuSample> {
    let mut cmd = Command::new("typeperf");
    cmd.args([r"\GPU Engine(*)\Utilization Percentage", "-sc", "1"]);
    hide_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_gpu_typeperf(&String::from_utf8_lossy(&out.stdout))
}

fn parse_gpu_typeperf(stdout: &str) -> Option<GpuSample> {
    let mut lines = stdout.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next()?;
    let data = lines.next()?;
    let headers = parse_csv_line(header);
    let values = parse_csv_line(data);
    let mut three_d: Option<f64> = None;
    let mut other: Option<f64> = None;
    for (i, h) in headers.iter().enumerate() {
        let hl = h.to_ascii_lowercase();
        if !hl.contains("gpu engine") {
            continue;
        }
        let Some(raw) = values.get(i) else { continue };
        let Ok(v) = raw.trim().parse::<f64>() else { continue };
        if hl.contains("engtype_3d") || hl.contains("engtype_compute") || hl.contains("engtype_cuda") {
            three_d = Some(three_d.map_or(v, |x| x.max(v)));
        } else {
            other = Some(other.map_or(v, |x| x.max(v)));
        }
    }
    let util = three_d.or(other)?.clamp(0.0, 100.0);
    Some(GpuSample {
        util,
        mem_used_mib: 0.0,
        mem_total_mib: 0.0,
        name: String::new(),
    })
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

async fn probe_http_all(cfg: &Config, client: &reqwest::Client) -> Vec<Cell> {
    let mut out = Vec::new();
    for spec in &cfg.http {
        out.push(probe_http(spec, client, &cfg.launch_allow).await);
    }
    out
}

async fn probe_http(spec: &HttpCfg, client: &reqwest::Client, allow: &[String]) -> Cell {
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
    let has_start = spec
        .start_program
        .as_deref()
        .is_some_and(|p| crate::config::program_allowed(p, allow));
    let has_stop = spec
        .stop_program
        .as_deref()
        .is_some_and(|p| crate::config::program_allowed(p, allow));
    let has_restart = spec
        .restart_program
        .as_deref()
        .is_some_and(|p| crate::config::program_allowed(p, allow));
    Cell {
        id: spec.id.clone(),
        label: spec.label.clone(),
        status,
        primary,
        detail: detail.clone(),
        copy_text: format!("{} {} {}", spec.label, spec.url, detail),
        actions: radial_actions(has_open, has_start, has_stop, has_restart),
    }
}

fn probe_files(cfg: &Config) -> Vec<Cell> {
    cfg.file.iter().map(|f| probe_file(cfg, f)).collect()
}

fn probe_file(cfg: &Config, spec: &FileCfg) -> Cell {
    let path = Path::new(&spec.path);
    let actions = radial_actions(spec.open.is_some(), false, false, false);

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
                if let Some(bad) = checks.iter().find(|c| {
                    !matches!(c.get("status").and_then(|s| s.as_str()), Some("ok"))
                }) {
                    primary = bad
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("fail")
                        .to_string();
                    detail = bad
                        .get("detail")
                        .and_then(|n| n.as_str())
                        .or_else(|| bad.get("name").and_then(|n| n.as_str()))
                        .unwrap_or("check failed")
                        .chars()
                        .take(48)
                        .collect();
                }
            }
        } else {
            primary = "ok".into();
            detail = format!("age {age_s}");
        }
    }
    if let Some(field) = &spec.string_field {
        let val = json.get(field).and_then(|v| v.as_str()).unwrap_or("unknown");
        let ok_val = spec.ok_value.as_deref().unwrap_or("ok");
        primary = val.to_string();
        status = overall_status(val, ok_val);
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

fn overall_status(val: &str, ok_val: &str) -> Status {
    let v = val.trim().to_ascii_lowercase();
    let ok = ok_val.trim().to_ascii_lowercase();
    let happy = v == ok || matches!(v.as_str(), "ok" | "healthy" | "pass" | "true");
    if happy {
        Status::Ok
    } else if matches!(v.as_str(), "failed" | "fail" | "down" | "error" | "crit") {
        Status::Down
    } else {
        Status::Degraded
    }
}

fn copy_only() -> Vec<Action> {
    radial_actions(false, false, false, false)
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
    let needle = normalize_match_text(needle);
    if needle.is_empty() {
        return false;
    }
    let mut sys = sysinfo::System::new();
    // sysinfo 0.33's default refresh_processes() skips cmd/cwd, so a running
    // `node C:\dev\cam-mcp\server.js` looked dead (HTTP down → status "down").
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing()
            .with_cmd(sysinfo::UpdateKind::Always)
            .with_cwd(sysinfo::UpdateKind::Always)
            .with_exe(sysinfo::UpdateKind::Always),
    );
    sys.processes().values().any(|p| process_match_text(p).contains(&needle))
}

fn normalize_match_text(s: &str) -> String {
    s.to_ascii_lowercase().replace('/', "\\")
}

fn process_match_text(p: &sysinfo::Process) -> String {
    let mut parts = Vec::new();
    parts.push(p.name().to_string_lossy().into_owned());
    for arg in p.cmd() {
        parts.push(arg.to_string_lossy().into_owned());
    }
    if let Some(cwd) = p.cwd() {
        parts.push(cwd.to_string_lossy().into_owned());
    }
    if let Some(exe) = p.exe() {
        parts.push(exe.to_string_lossy().into_owned());
    }
    normalize_match_text(&parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::is_cursor_desktop_process;

    #[test]
    fn matches_cursor_exe_by_name() {
        assert!(is_cursor_desktop_process(
            "Cursor.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\cursor\Cursor.exe"),
        ));
        assert!(is_cursor_desktop_process("cursor.exe", None));
        assert!(is_cursor_desktop_process("Cursor", None));
    }

    #[test]
    fn matches_helpers_under_cursor_install() {
        assert!(is_cursor_desktop_process(
            "crashpad_handler.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\cursor\crashpad_handler.exe"),
        ));
        assert!(is_cursor_desktop_process(
            "node.exe",
            Some(r"c:\Users\Randy\AppData\Local\Programs\cursor\resources\app\resources\helpers\node.exe"),
        ));
        assert!(is_cursor_desktop_process(
            "OpenConsole.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\cursor\resources\app\node_modules\node-pty\build\Release\conpty\OpenConsole.exe"),
        ));
        assert!(is_cursor_desktop_process(
            "code-tunnel.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\cursor\resources\app\bin\code-tunnel.exe"),
        ));
    }

    #[test]
    fn rejects_vscode_pulse_and_unrelated_helpers() {
        assert!(!is_cursor_desktop_process(
            "Code.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\Microsoft VS Code\Code.exe"),
        ));
        assert!(!is_cursor_desktop_process(
            "Code.exe",
            Some(r"C:\Users\Randy\AppData\Local\Programs\cursor\Code.exe"),
        ));
        assert!(!is_cursor_desktop_process("pulse.exe", None));
        assert!(!is_cursor_desktop_process(
            "crashpad_handler.exe",
            Some(r"C:\Program Files\Google\Drive File Stream\130.0.2.0\crashpad_handler.exe"),
        ));
        assert!(!is_cursor_desktop_process(
            "pwsh.exe",
            Some(r"C:\Program Files\PowerShell\7\pwsh.exe"),
        ));
    }

    #[test]
    fn radial_swaps_unused_restart_for_info() {
        let path = super::radial_actions(false, false, false, false);
        let ids: Vec<&str> = path.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["open", "start", "stop", "info", "copy"]);
        assert!(path.iter().find(|a| a.id == "info").unwrap().enabled);
        assert!(!path.iter().find(|a| a.id == "open").unwrap().enabled);

        let ice = super::radial_actions(true, true, true, true);
        let ice_ids: Vec<&str> = ice.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ice_ids, ["open", "start", "stop", "restart", "copy"]);
        assert!(!ice.iter().any(|a| a.id == "info"));
    }

    #[test]
    fn file_probe_treats_overall_ok_as_ok_even_if_config_says_healthy() {
        let dir = std::env::temp_dir().join(format!("pulse-fp-ok-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ws.json");
        std::fs::write(&path, r#"{"overall":"ok","operator_summary":"path ok"}"#).unwrap();
        let mut cfg = crate::config::parse_config(crate::config::PRESET_DEFAULT).unwrap();
        cfg.stale_secs = 60 * 60 * 24 * 365;
        cfg.file = vec![crate::config::FileCfg {
            id: "ws".into(),
            label: "ws-ops".into(),
            path: path.to_string_lossy().into_owned(),
            bool_field: None,
            string_field: Some("overall".into()),
            ok_value: Some("healthy".into()),
            open: None,
        }];
        let cell = super::probe_file(&cfg, &cfg.file[0]);
        assert!(matches!(cell.status, super::Status::Ok), "{:?}", cell.primary);
        assert_eq!(cell.primary, "ok");
    }

    #[test]
    fn file_probe_maps_failed_overall_to_down() {
        let dir = std::env::temp_dir().join(format!("pulse-fp-fail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ws.json");
        std::fs::write(&path, r#"{"overall":"failed"}"#).unwrap();
        let mut cfg = crate::config::parse_config(crate::config::PRESET_DEFAULT).unwrap();
        cfg.stale_secs = 60 * 60 * 24 * 365;
        cfg.file = vec![crate::config::FileCfg {
            id: "ws".into(),
            label: "ws-ops".into(),
            path: path.to_string_lossy().into_owned(),
            bool_field: None,
            string_field: Some("overall".into()),
            ok_value: Some("ok".into()),
            open: None,
        }];
        let cell = super::probe_file(&cfg, &cfg.file[0]);
        assert!(matches!(cell.status, super::Status::Down));
    }

    #[test]
    fn file_probe_shows_failed_check_name() {
        let dir = std::env::temp_dir().join(format!("pulse-fp-xsiam-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x.json");
        std::fs::write(
            &path,
            r#"{"checks_ok":false,"checks":[{"name":"auth_selftest","status":"failed","detail":"op read failed"}]}"#,
        )
        .unwrap();
        let mut cfg = crate::config::parse_config(crate::config::PRESET_DEFAULT).unwrap();
        cfg.stale_secs = 60 * 60 * 24 * 365;
        cfg.file = vec![crate::config::FileCfg {
            id: "xsiam".into(),
            label: "xsiam-ops".into(),
            path: path.to_string_lossy().into_owned(),
            bool_field: Some("checks_ok".into()),
            string_field: None,
            ok_value: None,
            open: None,
        }];
        let cell = super::probe_file(&cfg, &cfg.file[0]);
        assert!(matches!(cell.status, super::Status::Down));
        assert_eq!(cell.primary, "auth_selftest");
        assert!(cell.detail.contains("op read"));
    }
}


