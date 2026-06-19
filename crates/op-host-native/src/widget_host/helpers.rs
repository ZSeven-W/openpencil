//! Free-standing helpers used across the `widget_host` submodules:
//! hex-color parsing, color formatting, rect containment, and the
//! resize-bounds math. Pulled out of `widget_host.rs` to keep the
//! spine file under the 800-line ceiling.

// Shared color / number / resize helpers now live in the wasm-clean UI
// crate so the native + web hosts can't drift. Re-exported here under the
// historical `widget_host` paths so call sites stay unchanged.
#[allow(unused_imports)]
pub(in crate::widget_host) use op_editor_ui::util::{
    color_to_hex, color_to_hex_with_alpha, parse_hex_color, resize_bounds,
};

/// Small breathing room from the canvas corner so the chat pill
/// doesn't visually touch the canvas edge (per 2026-05-10 user
/// note to add just a little vertical spacing).
pub(in crate::widget_host) const AICHAT_INSET_BOTTOM: f32 = 12.0;
pub(in crate::widget_host) const AICHAT_INSET_LEFT: f32 = 12.0;

pub(in crate::widget_host) const TOOLBAR_INSET_X: f32 = 12.0;
pub(in crate::widget_host) const TOOLBAR_INSET_Y: f32 = 12.0;
pub(in crate::widget_host) const STATUS_INSET: f32 = 16.0;

/// Floating Git panel — gap below the TopBar (room for the caret that
/// connects the panel to its toggle button) + the caret's height and
/// half-width. Shared by `overlay_rects::git_panel_rect` (placement)
/// and `paint` (the caret triangle) so they can't drift.
pub(in crate::widget_host) const GIT_PANEL_CARET_GAP: f32 = 9.0;
pub(in crate::widget_host) const GIT_PANEL_CARET_H: f32 = 7.0;
pub(in crate::widget_host) const GIT_PANEL_CARET_HALF: f32 = 8.0;

/// Pixel half-thickness of the resize gutter on each panel edge —
/// click within this distance of the edge to begin a resize drag.
pub(in crate::widget_host) const PANEL_RESIZE_GUTTER: f32 = 4.0;
/// Hard floor / ceiling for resizable panels (TS app uses similar
/// limits — left/right rails can't shrink below ~180 or grow past
/// half the viewport).
pub(in crate::widget_host) const PANEL_MIN_WIDTH: f32 = 180.0;
pub(in crate::widget_host) const PANEL_MAX_WIDTH: f32 = 480.0;
