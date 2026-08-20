use crate::config::{Config, HttpCfg, ProcessCfg};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x00000008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

pub fn run_action(cfg: &Config, cell_id: &str, action: &str) -> Result<String, String> {
    if matches!(cell_id, "path" | "cpu" | "ram" | "gpu" | "net") {
        return Err("no launch action".into());
    }
    if let Some(spec) = cfg.process.iter().find(|p| p.id == cell_id) {
        return process_action(spec, action);
    }

    if let Some(spec) = cfg.http.iter().find(|h| h.id == cell_id) {
        return http_action(spec, action, &cfg.launch_allow);
    }
    if let Some(spec) = cfg.file.iter().find(|f| f.id == cell_id) {
        return match action {
            "open" => open_url(spec.open.as_deref().ok_or("no URL")?),
            _ => Err("no launch action".into()),
        };
    }
    Err(format!("unknown cell {cell_id}"))
}

fn process_action(spec: &ProcessCfg, action: &str) -> Result<String, String> {
    match action {
        "open" => {
            let path = crate::probes::process_open_path(spec).ok_or("exe not found")?;
            open::that(&path).map_err(|e| e.to_string())?;
            Ok(format!("opened {}", path.display()))
        }
        "stop" | "restart" => Err("won't stop this process from Pulse".into()),
        _ => Err("no launch action".into()),
    }
}

fn http_action(spec: &HttpCfg, action: &str, allow: &[String]) -> Result<String, String> {
    match action {
        "open" => {
            if let Some(url) = &spec.open {
                if let Some(fallback) = &spec.open_fallback {
                    if url_up(url) {
                        return open_url(url);
                    }
                    return open_url(fallback);
                }
                return open_url(url);
            }
            Err("no URL".into())
        }
        "start" => spawn_cli(
            spec.start_program.as_deref().ok_or("no start")?,
            spec.start_args.as_deref().unwrap_or(&[]),
            spec.start_cwd.as_deref(),
            spec.task.as_deref(),
            true,
            allow,
        ),
        "stop" => spawn_cli(
            spec.stop_program.as_deref().ok_or("no stop")?,
            spec.stop_args.as_deref().unwrap_or(&[]),
            None,
            spec.task.as_deref(),
            false,
            allow,
        ),
        "restart" => spawn_cli(
            spec.restart_program.as_deref().ok_or("no restart")?,
            spec.restart_args.as_deref().unwrap_or(&[]),
            spec.start_cwd.as_deref(),
            spec.task.as_deref(),
            false,
            allow,
        ),
        _ => Err(format!("unknown action {action}")),
    }
}

fn url_up(url: &str) -> bool {
    let rest = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let hostport = rest.split('/').next().unwrap_or(rest);
    let addrs = match hostport.to_socket_addrs() {
        Ok(a) => a,
        Err(_) => return false,
    };
    addrs.filter(|a: &SocketAddr| matches!(a, SocketAddr::V4(_))).take(1).any(|addr| {
        TcpStream::connect_timeout(&addr, Duration::from_millis(400)).is_ok()
    })
}

fn open_url(url: &str) -> Result<String, String> {
    open::that(url).map_err(|e| e.to_string())?;
    Ok(format!("opened {url}"))
}

fn spawn_cli(
    program: &str,
    args: &[String],
    cwd: Option<&str>,
    task: Option<&str>,
    detach: bool,
    allow: &[String],
) -> Result<String, String> {
    allow_program(program, allow)?;
    let resolved = resolve_program(program);
    let mut cmd = Command::new(&resolved);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut flags = CREATE_NO_WINDOW;
        if detach {
            flags |= DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP;
        }
        cmd.creation_flags(flags);
    }

    if detach {
        cmd.spawn().map_err(|e| {
            if let Some(name) = task {
                format!("could not run {resolved} ({e}). Copy: Start-ScheduledTask -TaskName {name}")
            } else {
                format!("could not run {resolved}: {e}")
            }
        })?;
        return Ok(format!("started {resolved} {}", args.join(" ")));
    }

    match cmd.output() {
        Ok(out) => {
            if out.status.success() {
                Ok(format!("{program} {}", args.join(" ")))
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                let msg = err.trim();
                if let Some(name) = task {
                    return Err(format!(
                        "{program} failed. If this needs admin, copy: Start-ScheduledTask -TaskName {name}"
                    ));
                }
                Err(if msg.is_empty() {
                    format!("{program} exited {}", out.status)
                } else {
                    msg.chars().take(180).collect()
                })
            }
        }
        Err(e) => {
            if let Some(name) = task {
                return Err(format!(
                    "could not run {program} ({e}). Copy: Start-ScheduledTask -TaskName {name}"
                ));
            }
            Err(format!("could not run {program}: {e}"))
        }
    }
}

fn resolve_program(program: &str) -> String {
    #[cfg(windows)]
    {
        if program.eq_ignore_ascii_case("npm") {
            return "npm.cmd".into();
        }
    }
    program.to_string()
}

fn allow_program(program: &str, allow: &[String]) -> Result<(), String> {
    if crate::config::program_allowed(program, allow) {
        Ok(())
    } else {
        Err(format!("program {program} is not on the allow list"))
    }
}
