//! Iconify SVG-body ingestion — the widget-boundary-sanctioned seam.
//!
//! `op_editor_ui::widgets` may only be referenced from `widget_host.rs`
//! and its sibling submodules (`tools/check-widget-boundary.sh` F4).
//! The remote-icon fetcher (`iconify_web.rs`) is transport plumbing,
//! so its parse step routes through this wrapper instead of calling
//! `icon_catalog` directly.

use op_editor_ui::widgets::icon_catalog::{parse_iconify_body, IconRenderStyle};

/// Boundary-flattened [`parse_iconify_body`] result: path data, the
/// resolved viewBox, and the render style as the wire string the
/// picker rows persist (`"stroke"` / `"fill"`).
pub(crate) struct IngestedIcon {
    pub d: String,
    pub width: f32,
    pub height: f32,
    pub style: &'static str,
}

/// Parse one Iconify icon `body` into the picker's row shape.
/// `None` mirrors the catalog parser's rejection (unsupported body).
pub(crate) fn ingest_iconify_body(svg_body: &str, w: f32, h: f32) -> Option<IngestedIcon> {
    let parsed = parse_iconify_body(svg_body, w, h)?;
    Some(IngestedIcon {
        d: parsed.d,
        width: parsed.width,
        height: parsed.height,
        style: match parsed.style {
            IconRenderStyle::Stroke => "stroke",
            IconRenderStyle::Fill => "fill",
        },
    })
}
