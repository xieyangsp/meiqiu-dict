// Small window-positioning helpers shared by capture (floater) and
// commands (popup). The pure `clamp_rect` does the math so it stays
// trivially unit-testable; `clamp_to_monitor` is the thin Tauri adapter.

use tauri::{AppHandle, Monitor, PhysicalPosition, PhysicalSize, Runtime};

/// Clamp a `size`-sized window anchored at `anchor` so it fits inside
/// the monitor whose rect contains the anchor. Falls back to the
/// primary monitor, then to the anchor unchanged.
pub fn clamp_to_monitor<R: Runtime>(
    app: &AppHandle<R>,
    anchor: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let monitor = pick_monitor(app, anchor);
    let Some(monitor) = monitor else {
        return anchor;
    };
    clamp_rect(anchor, size, monitor.position(), monitor.size())
}

fn pick_monitor<R: Runtime>(
    app: &AppHandle<R>,
    anchor: PhysicalPosition<i32>,
) -> Option<Monitor> {
    if let Ok(monitors) = app.available_monitors() {
        if let Some(m) = monitors.into_iter().find(|m| contains(m, anchor)) {
            return Some(m);
        }
    }
    app.primary_monitor().ok().flatten()
}

fn contains(monitor: &Monitor, p: PhysicalPosition<i32>) -> bool {
    let pos = monitor.position();
    let size = monitor.size();
    let x0 = pos.x;
    let y0 = pos.y;
    let x1 = x0 + size.width as i32;
    let y1 = y0 + size.height as i32;
    p.x >= x0 && p.x < x1 && p.y >= y0 && p.y < y1
}

fn clamp_rect(
    anchor: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    mon_pos: &PhysicalPosition<i32>,
    mon_size: &PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let w = size.width as i32;
    let h = size.height as i32;
    let min_x = mon_pos.x;
    let min_y = mon_pos.y;
    // If the window is wider than the monitor, prefer the left edge.
    let max_x = (mon_pos.x + mon_size.width as i32 - w).max(min_x);
    let max_y = (mon_pos.y + mon_size.height as i32 - h).max(min_y);
    PhysicalPosition::new(anchor.x.clamp(min_x, max_x), anchor.y.clamp(min_y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon() -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        (PhysicalPosition::new(0, 0), PhysicalSize::new(1920, 1080))
    }

    #[test]
    fn inside_is_unchanged() {
        let (p, s) = mon();
        let out = clamp_rect(PhysicalPosition::new(100, 200), PhysicalSize::new(360, 260), &p, &s);
        assert_eq!(out, PhysicalPosition::new(100, 200));
    }

    #[test]
    fn right_edge_pulls_in() {
        let (p, s) = mon();
        let out = clamp_rect(PhysicalPosition::new(1800, 100), PhysicalSize::new(360, 260), &p, &s);
        assert_eq!(out, PhysicalPosition::new(1920 - 360, 100));
    }

    #[test]
    fn bottom_edge_pulls_up() {
        let (p, s) = mon();
        let out = clamp_rect(PhysicalPosition::new(100, 1000), PhysicalSize::new(360, 260), &p, &s);
        assert_eq!(out, PhysicalPosition::new(100, 1080 - 260));
    }

    #[test]
    fn negative_anchor_snaps_to_origin() {
        let (p, s) = mon();
        let out = clamp_rect(PhysicalPosition::new(-50, -50), PhysicalSize::new(360, 260), &p, &s);
        assert_eq!(out, PhysicalPosition::new(0, 0));
    }

    #[test]
    fn secondary_monitor_origin_respected() {
        let p = PhysicalPosition::new(1920, 0);
        let s = PhysicalSize::new(1280, 720);
        let out = clamp_rect(PhysicalPosition::new(3100, 100), PhysicalSize::new(360, 260), &p, &s);
        assert_eq!(out, PhysicalPosition::new(1920 + 1280 - 360, 100));
    }
}
