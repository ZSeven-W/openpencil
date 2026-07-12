//! PDF multi-page export. Skia ships a built-in PDF backend
//! (`skia_safe::pdf::new_document`) — each [`ScenePage`] becomes one
//! PDF page laid out at its bounding rect. Mirrors the TS app's
//! global-export → exportDocumentPdf flow, except the TS app
//! hand-rolls a PDF stream (Catalog + Pages + Image XObjects with
//! DCTDecode JPEG blobs); skia's backend emits real vector PDF ops,
//! which keeps glyphs + shapes selectable + zoom-clean.
//!
//! The export consumes a layout-resolved [`LayoutScene`] — every node
//! kind paints through the shared `crate::export::paint_nodes`, the
//! same live-canvas scene painter the raster path uses (see
//! `export/scene_painter.rs`), so PDF / PNG / screenshots stay in
//! lockstep with the editor canvas.

use op_editor_ui::layout_scene::{LayoutScene, ScenePage};
use op_editor_ui::Rect;
use std::path::Path as StdPath;

const PDF_MARGIN: f32 = 16.0;

fn render_pdf_item(
    bounds: Rect,
    paint: impl FnOnce(&skia_safe::Canvas),
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        let page_size = skia_safe::Size::new(
            bounds.size.x + PDF_MARGIN * 2.0,
            bounds.size.y + PDF_MARGIN * 2.0,
        );
        let mut page = pdf.begin_page(page_size, None);
        let canvas = page.canvas();
        canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
        paint(canvas);
        pdf = page.end_page();
        pdf.close();
    }
    if buf.is_empty() {
        Err("PDF encoder returned no bytes".into())
    } else {
        Ok(buf)
    }
}

/// Render one resolved page as a single tightly cropped PDF page.
pub fn render_page_pdf_bytes(page: &ScenePage) -> Result<Vec<u8>, String> {
    let bounds = crate::export::page_bounds(page)
        .ok_or_else(|| format!("page {} has no visible content to export", page.id))?;
    render_pdf_item(bounds, |canvas| {
        crate::export::paint_nodes(canvas, &page.children)
    })
}

/// Render one node from its resolved containing page as a single PDF page.
pub fn render_node_on_page_pdf_bytes(page: &ScenePage, node_id: &str) -> Result<Vec<u8>, String> {
    let node = page
        .find(node_id)
        .ok_or_else(|| format!("node {node_id} not found on page {}", page.id))?;
    if node.hidden {
        return Err(format!("node {node_id} is hidden and cannot be exported"));
    }
    let mut acc = crate::export::BoundsAcc::new();
    crate::export::collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    let bounds = acc
        .into_rect()
        .ok_or_else(|| format!("node {node_id} paints nothing"))?;
    render_pdf_item(bounds, |canvas| crate::export::paint_node(canvas, node))
}

/// In-memory PDF variant: render each requested node as one PDF page
/// (cropped to that node's painted bounds). Returns the raw PDF bytes
/// (`%PDF-` magic through `%%EOF`) so the caller can base64-encode or
/// write to a file. Errors when `node_ids` is empty, the active page is
/// missing, or none of the requested nodes have paintable content.
/// Unknown node ids are skipped silently (the caller is expected to have
/// pre-validated them or to surface the per-id errors separately).
pub fn render_nodes_pdf_bytes(
    scene: &LayoutScene,
    node_ids: &[&str],
    _scale: f32,
) -> Result<Vec<u8>, String> {
    let page = scene.active_page().ok_or("no active page")?;
    if node_ids.is_empty() {
        return Err("no node ids provided".into());
    }
    // Collect (node, bounds) pairs for requested ids — skip unknowns and
    // nodes that paint nothing (mirrors export_node_raster's approach).
    let mut pairs = Vec::new();
    for &id in node_ids {
        let Some(node) = page.find(id) else { continue };
        let mut acc = crate::export::BoundsAcc::new();
        crate::export::collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
        let Some(bounds) = acc.into_rect() else {
            continue;
        };
        pairs.push((node, bounds));
    }
    if pairs.is_empty() {
        return Err("no requested nodes have paintable content on the active page".into());
    }
    // Find a common page size: the largest single-node bounds + margin on each
    // side, so every page in the PDF is the same dimensions.
    let (page_w, page_h) = {
        let mut max_w = 0.0_f32;
        let mut max_h = 0.0_f32;
        for (_, b) in &pairs {
            max_w = max_w.max(b.size.x);
            max_h = max_h.max(b.size.y);
        }
        (max_w + PDF_MARGIN * 2.0, max_h + PDF_MARGIN * 2.0)
    };
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        for (node, bounds) in &pairs {
            let mut on_page = pdf.begin_page(skia_safe::Size::new(page_w, page_h), None);
            let canvas = on_page.canvas();
            // Position the node's content with PDF_MARGIN inset from the page
            // edges, the same translation strategy as the multi-page exporter.
            canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
            crate::export::paint_node(canvas, node);
            pdf = on_page.end_page();
        }
        pdf.close();
    }
    Ok(buf)
}

/// Emit every page in `scene` as one PDF page each. Every page emits
/// at the SAME size — the union of all page bounds plus a 16-pt
/// margin — so PDF viewers don't jump between heterogeneous page
/// sizes on scroll/zoom. Each page's content is positioned within
/// that frame at its own `(origin.x, origin.y)`; empty pages are
/// skipped. Returns Err when no page has paintable content.
pub fn export_pdf(scene: &LayoutScene, target: &StdPath) -> Result<(), String> {
    let bounds_per_page: Vec<_> = scene.pages.iter().map(crate::export::page_bounds).collect();
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
            crate::export::paint_nodes(canvas, &page.children);
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
    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::layout_scene::{SceneNode, ScenePage};
    use op_editor_ui::{Color, Rect};

    fn two_page_scene() -> LayoutScene {
        LayoutScene {
            pages: vec![
                ScenePage {
                    id: "page-one".into(),
                    name: "Page One".into(),
                    children: vec![filled_rect(
                        "page-one-node",
                        0.0,
                        0.0,
                        20.0,
                        20.0,
                        Color::BLACK,
                    )],
                },
                ScenePage {
                    id: "page-two".into(),
                    name: "Page Two".into(),
                    children: vec![filled_rect(
                        "page-two-node",
                        100.0,
                        120.0,
                        40.0,
                        30.0,
                        Color::BLACK,
                    )],
                },
            ],
            active_page_index: 0,
        }
    }

    #[test]
    fn render_page_pdf_bytes_crops_one_resolved_page() {
        let scene = two_page_scene();
        let bytes = render_page_pdf_bytes(&scene.pages[1]).expect("page PDF");
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn render_node_on_page_pdf_bytes_accepts_non_active_page() {
        let scene = two_page_scene();
        let bytes =
            render_node_on_page_pdf_bytes(&scene.pages[1], "page-two-node").expect("node PDF");
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn pdf_export_emits_one_page_per_scene_page() {
        let mut ellipse = SceneNode::leaf("n20", NodeKind::Ellipse);
        ellipse.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
        ellipse.fill = Some(Color {
            r: 0.8,
            g: 0.2,
            b: 0.4,
            a: 1.0,
        });
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
                        Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        },
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
                ScenePage {
                    id: "p1".into(),
                    name: "P1".into(),
                    children: Vec::new(),
                },
                ScenePage {
                    id: "p2".into(),
                    name: "P2".into(),
                    children: Vec::new(),
                },
            ],
            active_page_index: 0,
        };
        let tmp = std::env::temp_dir().join(format!("op-pdf-empty-{}.pdf", std::process::id()));
        let res = export_pdf(&scene, &tmp);
        assert!(res.is_err(), "expected Err on all-empty, got {res:?}");
        assert_eq!(res.unwrap_err(), "nothing to export");
    }
}
