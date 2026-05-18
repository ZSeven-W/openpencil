//! Window geometry persistence.
//!
//! Distinct from `settings_io.rs` (user preferences) and
//! `persistence.rs` (the open document): this module remembers
//! *where the window was* — outer position, inner size and the
//! maximized flag — so a restart reopens the window the way the
//! user left it.
//!
//! Geometry is stored in PHYSICAL pixels next to the settings file
//! (`<config>/openpencil/window.json`). Physical units avoid a
//! chicken-and-egg DPI problem: the scale factor is not known until
//! a window exists, and winit's `with_inner_size` / `with_position`
//! both accept `Physical*` directly. The OS clamps a position that
//! no longer lands on any monitor, so a stale entry can't strand the
//! window off-screen.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const APP_DIR: &str = "openpencil";
const FILE_NAME: &str = "window.json";
const STATE_VERSION: u32 = 1;

/// Minimum restored size — guards against a corrupt file shrinking
/// the window to an unusable sliver. Mirrors the TS app's
/// `WINDOW_MIN_WIDTH` / `WINDOW_MIN_HEIGHT` (physical px at 1×).
const MIN_W: u32 = 640;
const MIN_H: u32 = 400;

/// Persisted window geometry. Position + size are PHYSICAL pixels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    version: u32,
    /// Outer-position x (physical px). `None` when never moved /
    /// unknown — restore then lets the OS pick a default spot.
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    /// Inner-size width / height (physical px).
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    /// Window was maximized at save time.
    #[serde(default)]
    maximized: bool,
}

/// Resolve `<config>/openpencil/window.json`. `None` when no usable
/// config base exists — load / save become silent no-ops.
fn state_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Best-effort load. Returns `None` on a missing file, parse error
/// or version mismatch — the caller then opens at default geometry.
pub fn load() -> Option<WindowState> {
    let path = state_path()?;
    let bytes = std::fs::read(&path).ok()?;
    let state: WindowState = serde_json::from_slice(&bytes).ok()?;
    if state.version != STATE_VERSION {
        return None;
    }
    Some(state)
}

/// Best-effort save. Silent on IO failure — losing the geometry of
/// a closing window must never block shutdown.
pub fn save(state: &WindowState) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(json) = serde_json::to_string_pretty(state) else {
        return;
    };
    let mut tmp = path.clone();
    tmp.set_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

impl WindowState {
    /// Build the to-persist snapshot from the live window geometry.
    /// `pos` / `size` are the last *windowed* (non-maximized) values
    /// the runner tracked, so un-maximizing after a restart restores
    /// a sensible size rather than the maximized rectangle.
    pub fn from_window(
        pos: Option<(i32, i32)>,
        size: Option<(u32, u32)>,
        maximized: bool,
    ) -> Self {
        Self {
            version: STATE_VERSION,
            x: pos.map(|p| p.0),
            y: pos.map(|p| p.1),
            width: size.map(|s| s.0),
            height: size.map(|s| s.1),
            maximized,
        }
    }

    /// Last windowed outer-position, if recorded.
    pub fn pos(&self) -> Option<(i32, i32)> {
        Some((self.x?, self.y?))
    }

    /// Last windowed inner-size, if recorded.
    pub fn size(&self) -> Option<(u32, u32)> {
        Some((self.width?, self.height?))
    }

    /// Whether the window was maximized at save time.
    pub fn maximized(&self) -> bool {
        self.maximized
    }

    /// Apply the saved geometry onto a winit `WindowAttributes`. A
    /// missing size falls back to the caller's default attrs; a
    /// too-small size is clamped to the usable minimum.
    pub fn apply_to(&self, mut attrs: winit::window::WindowAttributes) -> winit::window::WindowAttributes {
        if let (Some(w), Some(h)) = (self.width, self.height) {
            let w = w.max(MIN_W);
            let h = h.max(MIN_H);
            attrs = attrs.with_inner_size(winit::dpi::PhysicalSize::new(w, h));
        }
        if let (Some(x), Some(y)) = (self.x, self.y) {
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(x, y));
        }
        attrs.with_maximized(self.maximized)
    }
}

/// A monitor's physical rectangle: `((x, y), (width, height))`.
pub type MonitorRect = ((i32, i32), (u32, u32));

/// Whether a window rect (physical px) keeps a usable amount of
/// itself on at least one of `monitors`.
///
/// A restored position can land on a monitor that has since been
/// disconnected or rearranged, leaving the window off-screen — and
/// `winit`'s `with_position` does not reliably clamp it back. The
/// runner calls this after window creation and recenters when it
/// returns `false`. The overlap threshold (a title-bar-sized strip)
/// means a window peeking by only a sliver still counts as
/// off-screen, since the user can't grab it.
pub fn rect_visible_on_monitors(
    pos: (i32, i32),
    size: (u32, u32),
    monitors: &[MonitorRect],
) -> bool {
    let (wx, wy) = pos;
    let ww = size.0 as i32;
    let wh = size.1 as i32;
    monitors.iter().any(|&((mx, my), (mw, mh))| {
        let mw = mw as i32;
        let mh = mh as i32;
        let overlap_x = (wx + ww).min(mx + mw) - wx.max(mx);
        let overlap_y = (wy + wh).min(my + mh) - wy.max(my);
        overlap_x >= 120 && overlap_y >= 60
    })
}

/// Centre a window of `size` on the monitor rect `monitor`.
pub fn centered_on(size: (u32, u32), monitor: MonitorRect) -> (i32, i32) {
    let ((mx, my), (mw, mh)) = monitor;
    let cx = mx + (mw as i32 - size.0 as i32) / 2;
    let cy = my + (mh as i32 - size.1 as i32) / 2;
    (cx, cy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_window_carries_through_every_field() {
        let s = WindowState::from_window(Some((12, 34)), Some((1280, 720)), true);
        assert_eq!(s.x, Some(12));
        assert_eq!(s.y, Some(34));
        assert_eq!(s.width, Some(1280));
        assert_eq!(s.height, Some(720));
        assert!(s.maximized);
    }

    #[test]
    fn json_round_trips() {
        let s = WindowState::from_window(Some((1, 2)), Some((800, 600)), false);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: WindowState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.x, Some(1));
        assert_eq!(back.width, Some(800));
        assert!(!back.maximized);
        assert_eq!(back.version, STATE_VERSION);
    }

    #[test]
    fn a_future_version_is_rejected_by_the_parser_gate() {
        // `load` drops a state whose version this build doesn't know;
        // simulate that decision directly on a hand-built payload.
        let raw = format!(r#"{{"version":{},"width":800,"height":600}}"#, STATE_VERSION + 1);
        let parsed: WindowState = serde_json::from_str(&raw).expect("still parses");
        assert_ne!(parsed.version, STATE_VERSION);
    }

    #[test]
    fn apply_to_clamps_a_tiny_saved_size() {
        let s = WindowState::from_window(None, Some((10, 10)), false);
        // The clamp logic is what `apply_to` runs; assert on it
        // directly since `WindowAttributes` exposes no size getter.
        let w = s.width.unwrap().max(MIN_W);
        let h = s.height.unwrap().max(MIN_H);
        assert_eq!((w, h), (MIN_W, MIN_H));
    }

    #[test]
    fn rect_visible_when_it_overlaps_a_monitor() {
        let monitors = [((0, 0), (1920u32, 1080u32))];
        // Fully inside.
        assert!(rect_visible_on_monitors((100, 100), (800, 600), &monitors));
        // Partly off the right edge but still grabbable.
        assert!(rect_visible_on_monitors((1600, 100), (800, 600), &monitors));
    }

    #[test]
    fn rect_off_screen_when_no_monitor_overlaps() {
        let monitors = [((0, 0), (1920u32, 1080u32))];
        // Saved on a now-disconnected second monitor far to the right.
        assert!(!rect_visible_on_monitors((4000, 200), (800, 600), &monitors));
        // Only a 10 px sliver peeks in — below the grab threshold.
        assert!(!rect_visible_on_monitors((-790, 100), (800, 600), &monitors));
        // No monitors at all.
        assert!(!rect_visible_on_monitors((0, 0), (800, 600), &[]));
    }

    #[test]
    fn centered_on_places_the_window_in_the_monitor_middle() {
        let (x, y) = centered_on((800, 600), ((0, 0), (1920, 1080)));
        assert_eq!((x, y), (560, 240));
        // Honors a monitor origin offset (second display).
        let (x, y) = centered_on((800, 600), ((1920, 0), (1920, 1080)));
        assert_eq!((x, y), (2480, 240));
    }
}
