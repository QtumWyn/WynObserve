//! Bound saved placement to one display before showing the desktop window.
//!
//! The window-state plugin still owns saving. Its automatic restore is disabled
//! for `main` because it restores size without checking the monitor's work area.

use serde::Deserialize;
#[cfg(any(target_os = "linux", test))]
use tauri::LogicalSize;
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const STATE_FILENAME: &str = "wynobserve-window-state.json";

#[derive(Debug, Deserialize)]
struct SavedWindow {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    #[serde(default)]
    prev_x: Option<i32>,
    #[serde(default)]
    prev_y: Option<i32>,
    #[serde(default)]
    maximized: bool,
}

impl SavedWindow {
    fn bounds(&self) -> Option<Rect> {
        (self.width > 0 && self.height > 0).then(|| Rect {
            x: if self.maximized { self.prev_x } else { None }.unwrap_or(self.x),
            y: if self.maximized { self.prev_y } else { None }.unwrap_or(self.y),
            width: self.width,
            height: self.height,
        })
    }
}

fn saved_window(window: &WebviewWindow) -> Option<SavedWindow> {
    let path = window
        .app_handle()
        .path()
        .app_config_dir()
        .ok()?
        .join(STATE_FILENAME);
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            log::warn!("Could not read saved window placement: {error}");
            return None;
        }
    };
    let states: std::collections::HashMap<String, serde_json::Value> =
        match serde_json::from_reader(std::io::BufReader::new(file)) {
            Ok(states) => states,
            Err(error) => {
                log::warn!("Ignoring invalid saved window placement: {error}");
                return None;
            }
        };
    serde_json::from_value(states.get(window.label())?.clone()).ok()
}

/// Always reveal the window, even if a compositor refuses placement requests.
pub fn restore_and_show(window: &WebviewWindow) -> tauri::Result<()> {
    if let Err(error) = restore(window) {
        log::warn!("Could not restore window placement; using startup bounds: {error}");
    }
    window.show()?;
    if let Err(error) = window.set_focus() {
        log::debug!("Could not focus the restored window: {error}");
    }
    Ok(())
}

fn restore(window: &WebviewWindow) -> tauri::Result<()> {
    let saved = saved_window(window);
    let inner = window.inner_size()?;
    let position = window
        .outer_position()
        .unwrap_or(PhysicalPosition::new(0, 0));
    let saved_bounds = saved.as_ref().and_then(SavedWindow::bounds);
    let bounds = saved_bounds.unwrap_or(Rect {
        x: position.x,
        y: position.y,
        width: inner.width.max(1),
        height: inner.height.max(1),
    });
    let monitors = window.available_monitors()?;
    let areas: Vec<_> = monitors
        .iter()
        .map(|monitor| {
            let bounds = Rect::new(*monitor.position(), *monitor.size());
            let work = monitor.work_area();
            // Some backends report a desktop-wide work area. Never use the union of
            // displays as a size limit, and fall back when work-area data is absent.
            bounds
                .intersection(Rect::new(work.position, work.size))
                .unwrap_or(bounds)
        })
        .collect();
    let preferred = window.current_monitor()?.or(window.primary_monitor()?);
    let preferred_index = preferred
        .as_ref()
        .and_then(|preferred| {
            monitors.iter().position(|monitor| {
                monitor.position() == preferred.position() && monitor.size() == preferred.size()
            })
        })
        .unwrap_or(0);
    let Some(index) = pick_monitor(bounds, &areas, preferred_index) else {
        // No trustworthy monitor data: keep the configured 800 x 600 startup
        // size instead of applying potentially unbounded saved dimensions.
        return Ok(());
    };
    let scale = monitors[index].scale_factor();
    let outer = window.outer_size().unwrap_or(inner);
    let current_scale = window.scale_factor()?;
    // Hidden windows may not have decorations yet. Reserve titlebar/border room
    // in that case; don't confuse physical saved pixels with logical DPI units.
    let frame = if window.is_decorated()? {
        PhysicalSize::new(
            decoration_pixels(
                outer.width.saturating_sub(inner.width),
                16.0,
                current_scale,
                scale,
            ),
            decoration_pixels(
                outer.height.saturating_sub(inner.height),
                48.0,
                current_scale,
                scale,
            ),
        )
    } else {
        PhysicalSize::new(0, 0)
    };
    // `inner` describes the configured default using the window's current
    // scale. An unmapped Wayland window still reports 1 even on a 2x display.
    let default_size = inner
        .to_logical::<f64>(current_scale)
        .to_physical::<u32>(scale);
    let requested = saved_bounds.unwrap_or(Rect {
        width: default_size.width,
        height: default_size.height,
        ..bounds
    });
    let fitted = fit_restored_bounds(requested, default_size, areas[index], frame);
    if fitted != bounds {
        log::info!(
            "Adjusted saved window placement from {bounds:?} to {fitted:?} to fit one monitor"
        );
    }
    // Tao converts PhysicalSize to GTK logical units immediately. Before a
    // Wayland window is mapped, its scale is still 1 while its monitor may be
    // 2x, so a physical request here doubles on first show. Supply GTK logical
    // units using the target monitor's scale instead. Keep native physical
    // coordinates on Windows/macOS. Wayland may ignore position requests.
    let position = PhysicalPosition::new(fitted.x, fitted.y);
    #[cfg(target_os = "linux")]
    let position = position.to_logical::<f64>(scale);
    if let Err(error) = window.set_position(position) {
        log::debug!("Window manager did not accept the saved position: {error}");
    }
    let size = PhysicalSize::new(fitted.width, fitted.height);
    #[cfg(target_os = "linux")]
    let size = restore_logical_size(size, scale);
    window.set_size(size)?;
    if saved.as_ref().is_some_and(|state| state.maximized) {
        window.maximize()?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn restore_logical_size(size: PhysicalSize<u32>, monitor_scale: f64) -> LogicalSize<f64> {
    size.to_logical(monitor_scale)
}

fn fit_restored_bounds(
    bounds: Rect,
    default_size: PhysicalSize<u32>,
    area: Rect,
    frame: PhysicalSize<u32>,
) -> Rect {
    // An old doubled size or a layout from a removed larger monitor can fit
    // poorly even after clipping one dimension. Recover to the configured
    // default; ordinary saved placement and maximized state remain intact.
    let bounds = if bounds.width > area.width.saturating_sub(frame.width)
        || bounds.height > area.height.saturating_sub(frame.height)
    {
        Rect {
            width: default_size.width,
            height: default_size.height,
            ..bounds
        }
    } else {
        bounds
    };
    fit_to_area(bounds, area, frame)
}

fn decoration_pixels(measured: u32, fallback: f64, current_scale: f64, target_scale: f64) -> u32 {
    let logical = if measured == 0 {
        fallback
    } else {
        measured as f64 / current_scale
    };
    (logical * target_scale).ceil() as u32
}

/// All geometry uses physical pixels; i64 arithmetic avoids overflow when a
/// stale/corrupt state contains extreme coordinates or dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Rect {
    fn new(position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Self {
        Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }
    }

    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }
    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    fn contains_origin(self, other: Self) -> bool {
        i64::from(other.x) >= i64::from(self.x)
            && i64::from(other.x) < self.right()
            && i64::from(other.y) >= i64::from(self.y)
            && i64::from(other.y) < self.bottom()
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > i64::from(x) && bottom > i64::from(y)).then(|| Self {
            x,
            y,
            width: (right - i64::from(x)) as u32,
            height: (bottom - i64::from(y)) as u32,
        })
    }

    fn overlap(self, other: Self) -> u64 {
        self.intersection(other)
            .map_or(0, |r| u64::from(r.width) * u64::from(r.height))
    }
}

fn pick_monitor(bounds: Rect, areas: &[Rect], preferred: usize) -> Option<usize> {
    let valid = |area: &Rect| area.width > 0 && area.height > 0;
    // Keep the display containing the saved titlebar, even if an oversized
    // window covers more of another display below it.
    if let Some(index) = areas
        .iter()
        .position(|area| valid(area) && area.contains_origin(bounds))
    {
        return Some(index);
    }
    areas
        .iter()
        .enumerate()
        .filter(|(_, area)| valid(area))
        .max_by_key(|(index, area)| (area.overlap(bounds), *index == preferred))
        .map(|(index, _)| index)
}

fn fit_to_area(bounds: Rect, area: Rect, frame: PhysicalSize<u32>) -> Rect {
    let width = bounds
        .width
        .max(1)
        .min(area.width.saturating_sub(frame.width).max(1));
    let height = bounds
        .height
        .max(1)
        .min(area.height.saturating_sub(frame.height).max(1));
    let outer_width = u64::from(width) + u64::from(frame.width);
    let outer_height = u64::from(height) + u64::from(frame.height);
    let max_x = (area.right() - outer_width as i64).max(i64::from(area.x));
    let max_y = (area.bottom() - outer_height as i64).max(i64::from(area.y));
    let (x, y) = if area.overlap(bounds) == 0 {
        // A removed/rearranged monitor should not leave the titlebar offscreen.
        (
            (i64::from(area.x) + max_x) / 2,
            (i64::from(area.y) + max_y) / 2,
        )
    } else {
        (
            i64::from(bounds.x).clamp(i64::from(area.x), max_x),
            i64::from(bounds.y).clamp(i64::from(area.y), max_y),
        )
    };
    Rect {
        x: x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        y: y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    fn frame() -> PhysicalSize<u32> {
        PhysicalSize::new(16, 48)
    }

    #[test]
    fn hidden_wayland_window_does_not_double_size_when_mapped() {
        let saved = PhysicalSize::new(1600, 1200);
        let gtk_request = restore_logical_size(saved, 2.0);
        assert_eq!(gtk_request, LogicalSize::new(800.0, 600.0));
        assert_eq!(gtk_request.to_physical::<u32>(2.0), saved);
        // The old request used the hidden window's provisional scale of 1.
        assert_eq!(
            saved.to_logical::<f64>(1.0).to_physical::<u32>(2.0),
            PhysicalSize::new(3200, 2400)
        );
    }

    #[test]
    fn corrupted_doubled_save_recovers_to_default_size() {
        let area = rect(0, 0, 8192, 2304);
        let default_size = PhysicalSize::new(1600, 1200);
        let restored = fit_restored_bounds(
            rect(0, 0, 6388, 4280),
            default_size,
            area,
            PhysicalSize::new(32, 96),
        );
        assert_eq!(restored, rect(0, 0, 1600, 1200));
        assert_eq!(
            fit_restored_bounds(restored, default_size, area, PhysicalSize::new(32, 96)),
            restored
        );
    }

    #[test]
    fn small_monitor_also_bounds_the_default_recovery_size() {
        let restored = fit_restored_bounds(
            rect(0, 0, 6388, 4280),
            PhysicalSize::new(1600, 1200),
            rect(0, 0, 1280, 720),
            frame(),
        );
        assert_eq!(restored, rect(0, 0, 1264, 672));
    }

    #[test]
    fn valid_saved_geometry_is_unchanged() {
        let bounds = rect(180, 120, 1280, 800);
        assert_eq!(fit_to_area(bounds, rect(0, 0, 3440, 1400), frame()), bounds);
    }

    #[test]
    fn stacked_ultrawides_do_not_form_one_giant_window() {
        let areas = [rect(0, 0, 5120, 1440), rect(0, 1440, 5120, 1392)];
        let saved = rect(0, 0, 3194, 2140);
        let index = pick_monitor(saved, &areas, 1).unwrap();
        assert_eq!(index, 0);
        assert_eq!(
            fit_to_area(saved, areas[index], frame()),
            rect(0, 0, 3194, 1392)
        );
    }

    #[test]
    fn remembers_lower_monitor_and_respects_taskbar() {
        let areas = [rect(0, 0, 5120, 1440), rect(0, 1440, 5120, 1392)];
        let saved = rect(100, 1500, 2200, 1300);
        let index = pick_monitor(saved, &areas, 0).unwrap();
        assert_eq!(index, 1);
        assert_eq!(
            fit_to_area(saved, areas[index], frame()),
            rect(100, 1484, 2200, 1300)
        );
    }

    #[test]
    fn supports_monitors_above_and_left_of_primary() {
        let area = rect(-3440, -1440, 3440, 1400);
        let saved = rect(-3400, -1420, 1600, 900);
        assert_eq!(
            pick_monitor(saved, &[rect(0, 0, 1920, 1040), area], 0),
            Some(1)
        );
        assert_eq!(fit_to_area(saved, area, frame()), saved);
    }

    #[test]
    fn missing_monitor_recovers_to_preferred_monitor() {
        let areas = [rect(-1920, 0, 1920, 1040), rect(0, 0, 3440, 1400)];
        let saved = rect(9000, 9000, 1280, 800);
        let index = pick_monitor(saved, &areas, 1).unwrap();
        assert_eq!(index, 1);
        assert_eq!(
            fit_to_area(saved, areas[index], frame()),
            rect(1072, 276, 1280, 800)
        );
    }

    #[test]
    fn partially_offscreen_window_uses_largest_overlap() {
        let areas = [rect(0, 0, 1920, 1040), rect(1920, 0, 3440, 1400)];
        let saved = rect(1600, -30, 1280, 800);
        assert_eq!(pick_monitor(saved, &areas, 0), Some(1));
        assert_eq!(
            fit_to_area(saved, areas[1], frame()),
            rect(1920, 0, 1280, 800)
        );
    }

    #[test]
    fn physical_pixels_are_not_scaled_twice_on_high_dpi_monitor() {
        let saved = rect(2000, 100, 1600, 1200);
        let area = rect(1920, 0, 3840, 2080);
        let frame = PhysicalSize::new(
            decoration_pixels(16, 16.0, 1.0, 2.0),
            decoration_pixels(48, 48.0, 1.0, 2.0),
        );
        assert_eq!(frame, PhysicalSize::new(32, 96));
        assert_eq!(fit_to_area(saved, area, frame), saved);
        assert_eq!(
            fit_to_area(rect(1920, 0, 9000, 9000), area, frame),
            rect(1920, 0, 3808, 1984)
        );
    }

    #[test]
    fn desktop_wide_work_area_is_intersected_with_single_monitor() {
        let monitor = rect(0, 1440, 5120, 1440);
        let work_area = rect(0, 0, 5120, 2832);
        assert_eq!(
            monitor.intersection(work_area),
            Some(rect(0, 1440, 5120, 1392))
        );
    }

    #[test]
    fn corrupt_extreme_dimensions_do_not_overflow() {
        let saved = rect(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
        let area = rect(-1920, 0, 1920, 1040);
        assert_eq!(pick_monitor(saved, &[area], 0), Some(0));
        assert_eq!(fit_to_area(saved, area, frame()), rect(-1920, 0, 1904, 992));
    }

    #[test]
    fn invalid_monitors_do_not_receive_saved_dimensions() {
        assert_eq!(pick_monitor(rect(0, 0, 9000, 9000), &[], 0), None);
        assert_eq!(
            pick_monitor(rect(0, 0, 9000, 9000), &[rect(0, 0, 0, 0)], 0),
            None
        );
    }

    #[test]
    fn reads_existing_plugin_format_and_previous_maximized_position() {
        let saved: SavedWindow = serde_json::from_str(
            r#"{
            "width": 1280, "height": 800, "x": 0, "y": 1440,
            "prev_x": 100, "prev_y": 1500, "maximized": true,
            "visible": true, "decorated": true, "fullscreen": false
        }"#,
        )
        .unwrap();
        assert_eq!(saved.bounds(), Some(rect(100, 1500, 1280, 800)));
        assert!(saved.maximized);
    }

    #[test]
    fn missing_previous_position_falls_back_and_zero_size_is_ignored() {
        let saved: SavedWindow = serde_json::from_str(
            r#"{
            "width": 1280, "height": 800, "x": 100, "y": 1500, "maximized": true
        }"#,
        )
        .unwrap();
        assert_eq!(saved.bounds(), Some(rect(100, 1500, 1280, 800)));
        assert_eq!(SavedWindow { width: 0, ..saved }.bounds(), None);
    }
}
