//! Preview session construction over the browser Canvas2D measurement backend.
//!
//! The web host can instantiate a `PreviewSession` by:
//! 1. Using the browser's Canvas2D text measurement (same backend the design canvas uses)
//! 2. Resolving the active document through the editor state
//! 3. Passing the theme and page index to enter preview mode
//!
//! This keeps preview's hit-test layout aligned with the canvas's painted output,
//! since both measure text through the same browser Canvas2D `measureText` path
//! and font resolution stack.

use std::rc::Rc;

use crate::canvaskit::BrowserMeasure;
use op_preview_core::PreviewSession;

/// Build a preview session against the browser Canvas2D measurement backend —
/// the same backend the web design canvas measures with, so preview's
/// hit-test layout stays pixel-perfect aligned with what the canvas paints.
///
/// # Arguments
///
/// * `doc` — The canvas design's `PenDocument`, resolved from the editor state
/// * `canvas_size` — The logical (CSS) viewport dimensions
/// * `active_theme` — The current design token override map
/// * `active_page_index` — The active page's index in the document
/// * `op_ck` — The CanvasKit bridge object from the design canvas rendering path
/// * `presenting` — Whether preview is in presentation mode (app-mode routed)
///
/// # Returns
///
/// A live `PreviewSession` ready to receive input and render through the
/// shared layout engine, or an error if the preview root (`screen` component)
/// is not found or layout initialization fails.
///
/// # Panics
///
/// This function does not panic — all errors are returned as `PreviewEnterError`.
pub fn enter_preview(
    doc: &jian_ops_schema::PenDocument,
    canvas_size: (f32, f32),
    active_theme: &std::collections::BTreeMap<String, String>,
    active_page_index: usize,
    op_ck: &crate::canvaskit::OpCk,
    presenting: bool,
) -> Result<PreviewSession, op_preview_core::PreviewEnterError> {
    // Construct the browser-backed text measurement backend.
    // It uses the same browser Canvas2D measureText + font stack the design canvas does,
    // so this session's layout stays pixel-perfect aligned with the paint.
    let measure_backend = Rc::new(BrowserMeasure::new(op_ck));

    // Enter preview with the measure backend wired. The session takes ownership
    // of the `Rc` and holds it for the lifetime of hit-test queries.
    PreviewSession::enter(
        doc,
        canvas_size,
        active_theme,
        active_page_index,
        false, // preserve_authored_geometry: web preview does not preserve hand-drawn bounds
        presenting,
        measure_backend,
    )
}
