//! Browser Canvas2D text measurement backend for preview hit-testing.
//!
//! This backend intentionally reuses the design canvas's text measurement path
//! (`browserTextCtx.measureText` and family-aware font selection), ensuring that
//! preview's hit-test layout stays pixel-perfect aligned with what the canvas paints.
//! Both paths call the same JS functions: `measureTextFamilyStyled` for per-run
//! measurement and the new `measureParagraph` for full-paragraph wrapping.

use jian_core::layout::measure::{FontStyleKind, MeasureBackend, MeasureRequest, MeasureResult};
use js_sys::Array;
use wasm_bindgen::JsValue;

use super::bindings::OpCk;

/// Browser-backed text measurement for preview layout.
///
/// The web design canvas measures text through `browserTextCtx.measureText`,
/// resolving font families through the browser's own font stack. This backend
/// uses the exact same path for preview, keeping hit-test geometry in sync with
/// painted output. Both measure and draw go through the same font registry and
/// browser text measurement, so wrapping boundaries and baseline alignment match.
pub struct BrowserMeasure {
    op_ck: OpCk,
}

impl BrowserMeasure {
    /// Clone the bridge handle rather than borrowing it: `MeasureBackend` is
    /// held as an `Rc<dyn MeasureBackend>` by the layout engine and outlives
    /// any single call, and `OpCk` is a `#[wasm_bindgen]` import whose clone is
    /// just a JS handle refcount bump.
    pub fn new(op_ck: &OpCk) -> Self {
        Self {
            op_ck: op_ck.clone(),
        }
    }
}

impl MeasureBackend for BrowserMeasure {
    fn measure(&self, request: &MeasureRequest<'_>) -> MeasureResult {
        if request.runs.is_empty() {
            return MeasureResult {
                width: 0.0,
                height: 0.0,
                line_count: 0,
                baseline: 0.0,
            };
        }

        // Build parallel arrays matching the JS `measureParagraph` signature.
        let texts = Array::new();
        let families = Array::new();
        let mut sizes = Vec::with_capacity(request.runs.len());
        let mut weights = Vec::with_capacity(request.runs.len());
        let mut italics = Vec::with_capacity(request.runs.len());
        let mut spacing = Vec::with_capacity(request.runs.len());

        for run in request.runs {
            texts.push(&JsValue::from_str(run.text));
            families.push(&JsValue::from_str(run.font_family.unwrap_or("")));
            sizes.push(run.font_size);
            weights.push(run.font_weight as i32);
            italics.push(u8::from(run.font_style == FontStyleKind::Italic));
            spacing.push(run.letter_spacing);
        }

        // Call the JS bridge to measure the paragraph. Returns [width, height, line_count, baseline].
        let values = self.op_ck.measure_paragraph(
            &texts,
            &families,
            &sizes,
            &weights,
            &italics,
            &spacing,
            request.max_width.unwrap_or(-1.0),
            request.line_height,
        );

        MeasureResult {
            width: values.get_index(0),
            height: values.get_index(1),
            line_count: values.get_index(2).round() as u16,
            baseline: values.get_index(3),
        }
    }
}
