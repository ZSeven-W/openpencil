//! Toast-banner flow shared by both widget hosts.
//!
//! Mirrors `html_import_diagnostics_flow`: every host-visible transition —
//! placement, press, dismissal, the scheduled expiry wake-up — is
//! `EditorState` mutation plus widget-layer hit-tests, so `op-host-native` and
//! `op-host-web` each keep a thin arm that supplies the viewport and the
//! `mark_dirty()` tail. Both hosts behave identically because there is only
//! one implementation.
//!
//! The banner is non-modal: [`press`] returns `false` for a point outside it,
//! so the canvas underneath keeps receiving presses while a notice is up. It
//! is likewise off the Escape ladder — it takes no focus, and eating a keypress
//! aimed at the canvas to close a self-expiring banner would be a worse trade
//! than leaving it to time out.

use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::EditorState;

use crate::widgets::editor_toast::{EditorToast, EditorToastHit, TOAST_HEIGHT};
use crate::widgets::{AlignToolbar, PaintCx};
use crate::{Point2D, Rect};

/// Where the banner paints this frame, if at all.
///
/// Needs a `PaintCx` because the width is measured in the font the banner
/// paints with — so hosts resolve it inside their paint pass and the press arm
/// re-resolves it against the same state.
pub fn toast_rect<'a>(
    cx: &mut PaintCx<'_>,
    state: &'a EditorState,
    viewport_width: f32,
    viewport_height: f32,
    now_ms: u64,
) -> Option<(EditorToast<'a>, Rect)> {
    let toast = EditorToast::for_editor(state, now_ms)?;
    let mut canvas =
        crate::widgets::host_canvas_geometry::canvas_rect(state, viewport_width, viewport_height);
    if let Some(rect) = crate::widgets::missing_fonts_notice::MissingFontsNotice::for_editor(state)
        .and_then(|notice| notice.rect(viewport_width, viewport_height))
    {
        let occupied = rect.origin.y + rect.size.y + 8.0 - canvas.origin.y;
        canvas.origin.y += occupied;
        canvas.size.y = (canvas.size.y - occupied).max(0.0);
    }
    let width = toast.width(cx);
    // The align toolbar owns the same strip when a multi-selection is up; the
    // banner stacks below it rather than over it.
    let align_visible = AlignToolbar::for_canvas_region(canvas, state).is_some();
    let rect = EditorToast::rect_in_canvas(canvas, width, align_visible)?;
    Some((toast, rect))
}

/// Paint the banner, if one is due. Hosts call this above every panel and
/// below the missing-font modal, matching its press tier.
///
/// Returns the painted rect, for tests and for the hosts' hit-test parity.
pub fn paint(
    cx: &mut PaintCx<'_>,
    state: &EditorState,
    viewport_width: f32,
    viewport_height: f32,
    now_ms: u64,
) -> Option<Rect> {
    let (toast, rect) = toast_rect(cx, state, viewport_width, viewport_height, now_ms)?;
    toast.paint(cx, rect);
    Some(rect)
}

/// Route a press. `true` only when the point landed on the banner, so an
/// outside press falls through to the tiers below.
///
/// Takes the resolved `rect` rather than re-measuring: the width depends on
/// the painted text metrics, which only the paint pass can measure, so the
/// host caches the last painted rect and hands it back here. A host with no
/// cached rect (nothing painted yet) simply has no banner to press.
pub fn press(state: &mut EditorState, rect: Option<Rect>, point: Point2D, now_ms: u64) -> bool {
    let Some(rect) = rect else {
        return false;
    };
    // Re-checked here, not trusted from the cache: the toast may have expired
    // between the last paint and this press, and a stale rect must not eat a
    // click aimed at the canvas.
    if state.editor_ui.visible_toast(now_ms).is_none() {
        return false;
    }
    match EditorToast::hit_test(rect, point) {
        EditorToastHit::Dismiss => {
            state.editor_ui.dismiss_toast();
            true
        }
        EditorToastHit::Inside => true,
        EditorToastHit::Outside => false,
    }
}

/// The instant a visible toast expires, for the hosts' animation scheduler.
///
/// Expiring is not an input event, so without this the banner would stay on
/// screen until the user's next mouse move. `None` once nothing is up — the
/// toast needs exactly one wake-up, not a running clock.
pub fn next_deadline_ms(ui: &EditorUiState, now_ms: u64) -> Option<u64> {
    let expires = ui.visible_toast(now_ms)?.expires_at_ms();
    (now_ms < expires).then_some(expires)
}

/// Rough vertical band the banner occupies, for hosts that need a rect before
/// a paint pass has measured one. Deliberately height-only: the width is
/// text-dependent and any guess would disagree with the painted hit-test.
pub const fn band_height() -> f32 {
    TOAST_HEIGHT
}
