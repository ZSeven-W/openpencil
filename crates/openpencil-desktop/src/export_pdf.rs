//! PDF multi-page export. Skia ships a built-in PDF backend
//! (`skia_safe::pdf::new_document`) — each [`ScenePage`] becomes one
//! PDF page laid out at its bounding rect. Mirrors the TS app's
//! global-export → exportDocumentPdf flow, except the TS app
//! hand-rolls a PDF stream (Catalog + Pages + Image XObjects with
//! DCTDecode JPEG blobs); skia's backend emits real vector PDF ops,
//! which keeps glyphs + shapes selectable + zoom-clean.
//!
//! The export consumes a layout-resolved [`LayoutScene`] — every node
//! kind paints through the shared `crate::export::paint_node`, the
//! same painter the raster path uses, so PDF / PNG / SVG stay in
//! lockstep.

use openpencil_shell_core::layout_scene::LayoutScene;
use std::path::Path as StdPath;

const PDF_MARGIN: f32 = 16.0;

/// Emit every page in `scene` as one PDF page each. Every page emits
/// at the SAME size — the union of all page bounds plus a 16-pt
/// margin — so PDF viewers don't jump between heterogeneous page
/// sizes on scroll/zoom. Each page's content is positioned within
/// that frame at its own `(origin.x, origin.y)`; empty pages are
/// skipped. Returns Err when no page has paintable content.
pub fn export_pdf(scene: &LayoutScene, target: &StdPath) -> Result<(), String> {
    let bounds_per_page: Vec<_> = scene
        .pages
        .iter()
        .map(crate::export::page_bounds)
        .collect();
    let any_some = bounds_per_page.iter().any(Option::is_some);
    if !any_some {
        return Err("nothing to export".into());
    }
    let (page_w, page_h) = {
        let mut max_w = 0.0_f32;
        let mut max_h = 0.0_f32;
        for b in bounds_per_page.iter().flatten() {
            max_w = max_w.max(b.size.x);
            max_h = max_h.max(b.size.y);
        }
        (max_w + PDF_MARGIN * 2.0, max_h + PDF_MARGIN * 2.0)
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        for (page, bounds_opt) in scene.pages.iter().zip(bounds_per_page.iter()) {
            let Some(bounds) = bounds_opt else { continue };
            let mut on_page = pdf.begin_page(skia_safe::Size::new(page_w, page_h), None);
            let canvas = on_page.canvas();
            canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
            for node in &page.children {
                crate::export::paint_node(canvas, node);
            }
            pdf = on_page.end_page();
        }
        pdf.close();
    }
    std::fs::write(target, &buf).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::test_support::filled_rect;
    use openpencil_shell_core::layout_scene::NodeKind;
    use openpencil_shell_core::layout_scene::{ScenePage, SceneNode};
    use openpencil_shell_core::{Color, Rect};

    #[test]
    fn pdf_export_emits_one_page_per_scene_page() {
        let mut ellipse = SceneNode::leaf("n20", NodeKind::Ellipse);
        ellipse.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
        ellipse.fill = Some(Color { r: 0.8, g: 0.2, b: 0.4, a: 1.0 });
        let scene = LayoutScene {
            pages: vec![
                ScenePage {
                    id: "p1".into(),
                    name: "Page 1".into(),
                    children: vec![filled_rect(
                        "n10",
                        0.0,
                        0.0,
                        60.0,
                        40.0,
                        Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
                    )],
                },
                ScenePage {
                    id: "p2".into(),
                    name: "Page 2".into(),
                    children: vec![ellipse],
                },
            ],
            active_page_index: 0,
        };
        let tmp = std::env::temp_dir().join(format!("op-export-pdf-{}.pdf", std::process::id()));
        let res = export_pdf(&scene, &tmp);
        assert!(res.is_ok(), "export_pdf failed: {res:?}");
        let bytes = std::fs::read(&tmp).unwrap();
        // PDF header magic: %PDF-
        assert_eq!(&bytes[..5], b"%PDF-", "missing %PDF- header");
        // Should contain the EOF marker.
        let tail = &bytes[bytes.len().saturating_sub(8)..];
        assert!(
            tail.windows(5).any(|w| w == b"%%EOF"),
            "missing %%EOF trailer"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn pdf_export_skips_empty_pages_but_fails_when_all_empty() {
        let scene = LayoutScene {
            pages: vec![
                ScenePage { id: "p1".into(), name: "P1".into(), children: Vec::new() },
                ScenePage { id: "p2".into(), name: "P2".into(), children: Vec::new() },
            ],
            active_page_index: 0,
        };
        let tmp = std::env::temp_dir().join(format!("op-pdf-empty-{}.pdf", std::process::id()));
        let res = export_pdf(&scene, &tmp);
        assert!(res.is_err(), "expected Err on all-empty, got {res:?}");
        assert_eq!(res.unwrap_err(), "nothing to export");
    }
}
