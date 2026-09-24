//! The generation workspace's chat DRAWER for narrow windows.
//!
//! The docked chat is a 280–440 px column. Below
//! [`WORKSPACE_DRAWER_BREAKPOINT`] a column that wide leaves the design a
//! sliver, so the chat stops docking: the canvas takes the full width and
//! the conversation slides in over it as a drawer — opened by the
//! toolbar's chat toggle, closed by the toggle, Escape, or any press
//! outside it. Wide windows keep the column and its own open flag
//! (`sidebar_open`); the drawer's flag is separate so crossing the
//! breakpoint never rewrites the user's docked preference.

use super::EditorUiState;
use super::WorkspaceState;

/// Window width (logical px) below which the chat becomes a drawer. The
/// widest dock (440) plus the narrowest useful canvas (~460 px, a phone
/// board framed with its margins) — under that the column squeezes the
/// design more than the conversation is worth.
pub const WORKSPACE_DRAWER_BREAKPOINT: f32 = 900.0;

/// Slide duration of the drawer, shared by paint and the frame scheduler.
pub const WORKSPACE_DRAWER_SLIDE_MS: u64 = 220;

/// Gutter the drawer always leaves uncovered on its right, so there is a
/// visible strip of canvas to press to close it.
pub const WORKSPACE_DRAWER_MIN_GUTTER: f32 = 56.0;

impl WorkspaceState {
    /// Re-derive drawer mode from the window width. Entering drawer mode
    /// starts with the drawer shut — the reason to be in it is to give the
    /// design the width. Returns whether the mode changed.
    pub fn sync_drawer_mode(&mut self, viewport_w: f32) -> bool {
        let narrow = viewport_w > 0.0 && viewport_w < WORKSPACE_DRAWER_BREAKPOINT;
        if narrow == self.drawer_mode {
            return false;
        }
        self.drawer_mode = narrow;
        self.drawer_open = false;
        self.drawer_moved_at_ms = 0;
        true
    }

    /// Open or shut the drawer, stamping the slide at `now_ms`. Returns
    /// whether anything changed. A `now_ms` of 0 lands settled.
    pub fn set_drawer_open(&mut self, open: bool, now_ms: u64) -> bool {
        if self.drawer_open == open {
            return false;
        }
        self.drawer_open = open;
        self.drawer_moved_at_ms = now_ms;
        true
    }

    /// How far open the drawer is at `now_ms` (0 = shut, 1 = open), eased.
    /// `moved_at_ms` is the slide's stamp as the chrome sees it (0 under
    /// reduced motion — see [`EditorUiState::motion_stamp`]).
    pub fn drawer_progress(&self, moved_at_ms: u64, now_ms: u64) -> f32 {
        let target = if self.drawer_open { 1.0 } else { 0.0 };
        if moved_at_ms == 0 {
            return target;
        }
        let t = (now_ms.saturating_sub(moved_at_ms) as f32 / WORKSPACE_DRAWER_SLIDE_MS as f32)
            .clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        if self.drawer_open {
            eased
        } else {
            1.0 - eased
        }
    }

    /// The next frame the slide still needs, or `None` once it settled.
    pub fn drawer_deadline_ms(&self, moved_at_ms: u64, now_ms: u64) -> Option<u64> {
        if !self.visible || !self.drawer_mode || moved_at_ms == 0 {
            return None;
        }
        let end = moved_at_ms.saturating_add(WORKSPACE_DRAWER_SLIDE_MS);
        (now_ms < end).then_some(end)
    }
}

impl EditorUiState {
    /// The workspace is up on a desktop window narrow enough that its chat
    /// is a drawer rather than a docked column.
    pub fn workspace_drawer_active(&self) -> bool {
        self.workspace.visible && !self.touch_chrome() && self.workspace.drawer_mode
    }

    /// Width of the open drawer: the dock's own width, clamped so a strip
    /// of canvas stays visible beside it.
    pub fn workspace_drawer_width(&self, viewport_w: f32) -> f32 {
        self.layer_panel_width
            .min((viewport_w - WORKSPACE_DRAWER_MIN_GUTTER).max(0.0))
    }

    /// The drawer's slide progress at `now_ms`, honouring reduced motion.
    pub fn workspace_drawer_progress(&self, now_ms: u64) -> f32 {
        let stamp = self.motion_stamp(self.workspace.drawer_moved_at_ms);
        self.workspace.drawer_progress(stamp, now_ms)
    }

    /// The next frame the drawer's slide needs, honouring reduced motion.
    pub fn workspace_drawer_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        let stamp = self.motion_stamp(self.workspace.drawer_moved_at_ms);
        self.workspace.drawer_deadline_ms(stamp, now_ms)
    }
}

#[cfg(test)]
#[path = "workspace_drawer_tests.rs"]
mod tests;
