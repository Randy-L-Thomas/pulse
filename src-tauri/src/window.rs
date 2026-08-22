use crate::config::Config;
use crate::ui_state::{UiState, WIN_MIN_H, WIN_MIN_W};
use tauri::{PhysicalPosition, PhysicalSize, Size, WebviewWindow};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidthMode {
    Half,
    Full,
    Custom,
}

impl WidthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            WidthMode::Half => "half",
            WidthMode::Full => "full",
            WidthMode::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "full" => WidthMode::Full,
            "custom" => WidthMode::Custom,
            _ => WidthMode::Half,
        }
    }

    pub fn show_modules(self) -> bool {
        !matches!(self, WidthMode::Half)
    }
}

pub fn dock_and_size(window: &WebviewWindow, cfg: &Config, mode: WidthMode) -> Result<(), String> {
    let (origin, max_w) = strip_origin(window, cfg);
    let width = match mode {
        WidthMode::Half => cfg.half_width.min(max_w),
        WidthMode::Full | WidthMode::Custom => cfg.full_width.min(max_w).max(cfg.half_width),
    };
    let _ = window.set_min_size(Some(Size::Physical(PhysicalSize {
        width: WIN_MIN_W.min(max_w),
        height: WIN_MIN_H,
    })));
    let _ = window.set_max_size(None::<Size>);
    window
        .set_position(tauri::Position::Physical(PhysicalPosition {
            x: origin.0,
            y: origin.1,
        }))
        .map_err(|e| e.to_string())?;
    window
        .set_size(Size::Physical(PhysicalSize {
            width,
            height: cfg.height,
        }))
        .map_err(|e| e.to_string())?;
    let _ = window.set_always_on_top(true);
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

pub fn apply_saved(window: &WebviewWindow, cfg: &Config, ui: &UiState) -> Result<WidthMode, String> {
    let mode = WidthMode::parse(&ui.win_mode);
    if mode == WidthMode::Custom && frame_is_sane(ui.win_w, ui.win_h) {
        apply_frame(window, ui.win_x, ui.win_y, ui.win_w, ui.win_h)?;
        return Ok(WidthMode::Custom);
    }
    dock_and_size(window, cfg, mode)?;
    Ok(mode)
}

pub fn apply_frame(window: &WebviewWindow, x: i32, y: i32, w: u32, h: u32) -> Result<(), String> {
    let w = w.max(WIN_MIN_W);
    let h = h.max(WIN_MIN_H);
    let _ = window.set_min_size(Some(Size::Physical(PhysicalSize {
        width: WIN_MIN_W,
        height: WIN_MIN_H,
    })));
    let _ = window.set_max_size(None::<Size>);
    window
        .set_position(tauri::Position::Physical(PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    window
        .set_size(Size::Physical(PhysicalSize { width: w, height: h }))
        .map_err(|e| e.to_string())?;
    let _ = window.set_always_on_top(true);
    Ok(())
}

pub fn remember_frame(ui: &mut UiState, mode: WidthMode, x: i32, y: i32, w: u32, h: u32) {
    ui.win_mode = mode.as_str().into();
    ui.win_x = x;
    ui.win_y = y;
    ui.win_w = w;
    ui.win_h = h;
}

pub fn read_frame(window: &WebviewWindow) -> Option<(i32, i32, u32, u32)> {
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some((pos.x, pos.y, size.width, size.height))
}

pub fn frame_is_sane(w: u32, h: u32) -> bool {
    w >= WIN_MIN_W && h >= WIN_MIN_H && w <= 8000 && h <= 5000
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

#[cfg(test)]
mod tests {
    use super::{frame_is_sane, remember_frame, WidthMode};
    use crate::ui_state::UiState;

    #[test]
    fn custom_mode_shows_modules_half_does_not() {
        assert!(!WidthMode::Half.show_modules());
        assert!(WidthMode::Full.show_modules());
        assert!(WidthMode::Custom.show_modules());
        assert_eq!(WidthMode::parse("custom").as_str(), "custom");
    }

    #[test]
    fn remembers_custom_frame() {
        let mut ui = UiState::default();
        remember_frame(&mut ui, WidthMode::Custom, 10, 20, 800, 360);
        assert_eq!(ui.win_mode, "custom");
        assert_eq!((ui.win_x, ui.win_y, ui.win_w, ui.win_h), (10, 20, 800, 360));
        assert!(frame_is_sane(ui.win_w, ui.win_h));
        assert!(!frame_is_sane(100, 50));
    }
}
