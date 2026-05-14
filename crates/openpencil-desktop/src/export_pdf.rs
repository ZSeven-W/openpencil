//! PDF multi-page export. Skia ships a built-in PDF backend
//! (`skia_safe::pdf::new_document`) — each `PenPage` becomes one
//! PDF page laid out at its bounding rect. Mirrors the TS app's
//! global-export → exportDocumentPdf flow, except the TS app
//! hand-rolls a PDF stream (Catalog + Pages + Image XObjects with
//! DCTDecode JPEG blobs); skia's backend emits real vector PDF
//! ops, which keeps glyphs + shapes selectable + zoom-clean.

use openpencil_shell_core::document::Document;
use std::path::Path as StdPath;

const PDF_MARGIN: f32 = 16.0;

/// Emit every page in `doc` as one PDF page each. Pages are sized
/// to their content bounding box (`page_bounds` + 16-pt margin).
/// Empty pages are skipped. Returns Err when no page has paintable
/// content — same convention as `export_raster`.
pub fn export_pdf(doc: &Document, target: &StdPath) -> Result<(), String> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut pdf = skia_safe::pdf::new_document(&mut buf, None);
        let mut emitted = 0usize;
        for page in &doc.pages {
            let Some(bounds) = crate::export::page_bounds(page) else {
                continue;
            };
            let w = bounds.size.x + PDF_MARGIN * 2.0;
            let h = bounds.size.y + PDF_MARGIN * 2.0;
            if w <= 0.0 || h <= 0.0 {
                continue;
            }
            let mut on_page = pdf.begin_page(skia_safe::Size::new(w, h), None);
            let canvas = on_page.canvas();
            canvas.translate((PDF_MARGIN - bounds.origin.x, PDF_MARGIN - bounds.origin.y));
            for node in &page.children {
                crate::export::paint_node(canvas, node);
            }
            pdf = on_page.end_page();
            emitted += 1;
        }
        pdf.close();
        if emitted == 0 {
            return Err("nothing to export".into());
        }
    }
    std::fs::write(target, &buf).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::{Color, Rect};

    #[test]
    fn pdf_export_emits_one_page_per_pen_page() {
        let mut doc = Document::empty();
        let p0 = doc.pages.get_mut(0).unwrap();
        p0.children.clear();
        let mut n0 = Node::leaf(10, NodeKind::Rect, "r");
        n0.bounds = Rect::xywh(0.0, 0.0, 60.0, 40.0);
        n0.fill = Some(Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        });
        p0.children.push(n0);
        let p1 = openpencil_shell_core::document::Page::new(2, "Page 2", Vec::new());
        let mut n1 = Node::leaf(20, NodeKind::Ellipse, "e");
        n1.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
        n1.fill = Some(Color {
            r: 0.8,
            g: 0.2,
            b: 0.4,
            a: 1.0,
        });
        let mut p1 = p1;
        p1.children.push(n1);
        doc.pages.push(p1);
        let tmp = std::env::temp_dir().join(format!("op-export-pdf-{}.pdf", std::process::id()));
        let res = export_pdf(&doc, &tmp);
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
        let mut doc = Document::empty();
        // All pages empty.
        for p in &mut doc.pages {
            p.children.clear();
        }
        let tmp = std::env::temp_dir().join(format!("op-pdf-empty-{}.pdf", std::process::id()));
        let res = export_pdf(&doc, &tmp);
        assert!(res.is_err(), "expected Err on all-empty, got {res:?}");
        assert_eq!(res.unwrap_err(), "nothing to export");
    }
}
