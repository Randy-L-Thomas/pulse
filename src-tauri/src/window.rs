use crate::config::Config;
use tauri::{PhysicalPosition, PhysicalSize, Size, WebviewWindow};

#[derive(Clone, Copy)]
pub enum WidthMode {
    Half,
    Full,
}

pub fn dock_and_size(window: &WebviewWindow, cfg: &Config, mode: WidthMode) -> Result<(), String> {
    let (origin, max_w) = strip_origin(window, cfg);
    let width = match mode {
        WidthMode::Half => cfg.half_width.min(max_w),
        WidthMode::Full => cfg.full_width.min(max_w).max(cfg.half_width),
    };
    window
        .set_position(tauri::Position::Physical(PhysicalPosition { x: origin.0, y: origin.1 }))
        .map_err(|e| e.to_string())?;
    window
        .set_size(Size::Physical(PhysicalSize {
            width,
            height: cfg.height,
        }))
        .map_err(|e| e.to_string())?;
    let _ = window.set_max_size(Some(Size::Physical(PhysicalSize {
        width: max_w,
        height: cfg.height,
    })));
    let _ = window.set_min_size(Some(Size::Physical(PhysicalSize {
        width: cfg.half_width.min(max_w),
        height: cfg.height,
    })));
    let _ = window.set_always_on_top(true);
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

pub fn strip_origin(window: &WebviewWindow, cfg: &Config) -> ((i32, i32), u32) {
    let monitors = window.available_monitors().unwrap_or_default();
    // Prefer the short 1920-wide panel (Windows may report 440 as 450).
    let mut best: Option<(i32, i32, u32, u32)> = None;
    for mon in &monitors {
        let size = mon.size();
        if size.width != cfg.monitor_width || size.height > 500 {
            continue;
        }
        let pos = mon.position();
        let better = match best {
            None => true,
            Some((_, _, _, h)) => size.height <= h,
        };
        if better {
            best = Some((pos.x, pos.y, size.width, size.height));
        }
    }
    if let Some((x, y, w, _)) = best {
        return ((x, y), w);
    }
    if let Ok(Some(mon)) = window.primary_monitor() {
        let pos = mon.position();
        return ((pos.x, pos.y), mon.size().width.max(cfg.half_width));
    }
    ((0, 0), cfg.full_width)
}

pub fn clamp_resize(window: &WebviewWindow, cfg: &Config) {
    let Ok(size) = window.inner_size() else { return };
    let (_, max_w) = strip_origin(window, cfg);
    let w = size.width.clamp(cfg.half_width.min(max_w), max_w);
    let h = cfg.height;
    if w != size.width || size.height != h {
        let _ = window.set_size(Size::Physical(PhysicalSize { width: w, height: h }));
    }
}