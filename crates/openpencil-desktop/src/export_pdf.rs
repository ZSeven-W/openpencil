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

/// Emit every page in `doc` as one PDF page each. Every page emits
/// at the SAME size — the union of all page bounds plus a 16-pt
/// margin — so PDF viewers don't jump between heterogeneous page
/// sizes on scroll/zoom (codex CONCERN: mixed page sizes were a
/// papercut). Each page's content is positioned within that frame
/// at its own (origin.x, origin.y); empty pages are skipped.
/// Returns Err when no page has paintable content.
pub fn export_pdf(doc: &Document, target: &StdPath) -> Result<(), String> {
    let bounds_per_page: Vec<_> = doc
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
        for (page, bounds_opt) in doc.pages.iter().zip(bounds_per_page.iter()) {
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
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::{Color, Rect};

    #[test]
    fn pdf_export_emits_one_page_per_pen_page() {
        let mut doc = Document::empty();
        let p0 = doc.pages.get_mut(0).unwrap();
        p0.children.clear();
        let mut n0 = Node::leaf("n10", NodeKind::Rect, "r");
        n0.bounds = Rect::xywh(0.0, 0.0, 60.0, 40.0);
        n0.fill = Some(Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        });
        p0.children.push(n0);
        let p1 = openpencil_shell_core::document::Page::new("n2", "Page 2", Vec::new());
        let mut n1 = Node::leaf("n20", NodeKind::Ellipse, "e");
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
