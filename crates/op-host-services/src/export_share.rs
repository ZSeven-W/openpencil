//! Share package — the render + IO behind File ▸ "Share…" and the
//! Studio workspace's 分享 button.
//!
//! # Format choice
//!
//! There is no hosted service to upload to, so a share is ONE file the
//! author sends however they like. Two shapes were measured:
//!
//! - **Live viewer** — the `.op` plus `packages/op-web-sdk`'s wasm viewer
//!   inlined as base64. Measured 2026-09-25: the viewer wasm is 15.9 MB raw
//!   / 5.3 MB gzip and needs CanvasKit (7.1 MB raw / 2.8 MB gzip) beside
//!   it — 31 MB of inline base64, or 10.9 MB gzip-then-base64 behind an
//!   in-page `DecompressionStream`. Even the small form needs WebGL and a
//!   multi-second decode, and — decisive — the desktop binary does not
//!   have the viewer bundle: it is a separate wasm build the desktop
//!   pipeline never runs.
//! - **Resolved-scene page** (this module) — every board emitted by the
//!   same structured-markup exporter the slideshow uses: real, selectable
//!   text pinned at jian's layout rects, `data:` rasters only for what
//!   markup cannot express. The 6-slide sample deck shares as 67 KB; it
//!   opens in any browser (including in-app mobile browsers without
//!   WebGL) and needs nothing the desktop does not already ship.
//!
//! The page carries the document itself as a JSON script block, and its
//! "Make one like this" button downloads that `.op`: opened in
//! OpenPencil, the file's share recipe (brief, task, options, style
//! guide) offers the same generation to the recipient.
//!
//! # What leaves the machine
//!
//! The recipe is re-sanitized against host-known identifying terms and
//! the document's local file references are inlined or emptied (see
//! [`crate::export_share_sanitize`]). The page never names the file it
//! was saved from.

use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::{EditorState, Locale, ShareRecipe};
use std::path::Path;

use crate::export::ExportError;
use crate::export_html::board_name;
use crate::export_html_structured::board_slide_markup;
use crate::export_share_sanitize::{host_redact_terms, sanitize_document_value};
use crate::export_share_template::{render_share_page, ShareLabels, ShareSlide};

/// What one share export produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharePackage {
    /// Boards on the page, across every page of the document.
    pub boards: usize,
    /// Nodes that had to be embedded as a raster image.
    pub raster_fallbacks: usize,
    /// Size of the written page in bytes.
    pub bytes: usize,
}

/// Knobs a host passes; [`ShareOptions::for_state`] is the usual start.
#[derive(Debug, Clone)]
pub struct ShareOptions {
    /// Identifying terms redacted from the recipe (the OS account name).
    pub redact_terms: Vec<String>,
    /// Directory relative asset references resolve against.
    pub document_dir: Option<std::path::PathBuf>,
    /// Locale of the page chrome — the author's.
    pub locale: Locale,
}

impl ShareOptions {
    pub fn for_state(state: &EditorState, document_path: Option<&Path>) -> Self {
        Self {
            redact_terms: host_redact_terms(),
            document_dir: document_path.and_then(Path::parent).map(Path::to_path_buf),
            locale: state.editor_ui.effective_locale(),
        }
    }
}

/// Render the share page for `state` and write it to `target`.
pub fn export_share_html(
    state: &EditorState,
    target: &Path,
    options: &ShareOptions,
) -> Result<SharePackage, ExportError> {
    let (html, mut package) = render_share_html(state, options)?;
    std::fs::write(target, &html).map_err(|e| ExportError::Write(e.to_string()))?;
    package.bytes = html.len();
    Ok(package)
}

/// Render the share page in memory.
pub fn render_share_html(
    state: &EditorState,
    options: &ShareOptions,
) -> Result<(String, SharePackage), ExportError> {
    let mut package = SharePackage::default();
    let slides = collect_slides(state, &mut package)?;
    if slides.is_empty() {
        return Err(ExportError::NothingToExport);
    }
    package.boards = slides.len();

    let terms: Vec<&str> = options.redact_terms.iter().map(String::as_str).collect();
    let recipe = state
        .editor_ui
        .share_recipe()
        .map(|recipe| recipe.sanitized(&terms));
    let document_json = share_document_json(state, recipe.clone(), options)?;
    let title = op_editor_core::sanitize_share_text(&share_title(state, &slides), &terms);
    let labels = ShareLabels::for_locale(options.locale, recipe.as_ref());
    let html = render_share_page(&title, &slides, &document_json, &labels);
    package.bytes = html.len();
    Ok((html, package))
}

/// Every visible board of every page, pages in document order.
fn collect_slides(
    state: &EditorState,
    package: &mut SharePackage,
) -> Result<Vec<ShareSlide>, ExportError> {
    let page_count = state.doc.pages.as_ref().map_or(0, Vec::len);
    let mut slides = Vec::new();
    if page_count <= 1 {
        push_page_slides(state, None, &mut slides, package)?;
        return Ok(slides);
    }
    let mut page_state = state.clone();
    for index in 0..page_count {
        page_state.ui.active_page_index = index;
        let page_name = state
            .doc
            .pages
            .as_ref()
            .and_then(|pages| pages.get(index))
            .map(|page| page.name.trim().to_string())
            .filter(|name| !name.is_empty());
        push_page_slides(&page_state, page_name, &mut slides, package)?;
    }
    Ok(slides)
}

fn push_page_slides(
    state: &EditorState,
    page_name: Option<String>,
    slides: &mut Vec<ShareSlide>,
    package: &mut SharePackage,
) -> Result<(), ExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let Some(page) = scene.active_page() else {
        return Ok(());
    };
    for board_id in active_page_boards(state) {
        // A hidden board is the author saying it is not part of the work.
        match page.find(&board_id) {
            Some(node) if !node.hidden => {}
            _ => continue,
        }
        let markup = match board_slide_markup(page, &board_id, board_name(state, &board_id)) {
            Ok(markup) => markup,
            // An empty frame is not worth failing a whole share over.
            Err(ExportError::NodePaintsNothing { .. }) => continue,
            Err(error) => return Err(error),
        };
        package.raster_fallbacks += markup.raster_fallbacks();
        slides.push(ShareSlide {
            name: markup.name,
            page: page_name.clone(),
            width: markup.width,
            height: markup.height,
            body: markup.body,
        });
    }
    Ok(())
}

/// The embedded `.op`: canonical bytes with the sanitized recipe in its
/// `editorMeta`, local references rewritten, compact JSON.
fn share_document_json(
    state: &EditorState,
    recipe: Option<ShareRecipe>,
    options: &ShareOptions,
) -> Result<String, ExportError> {
    let meta = op_pen_loader::EditorMeta {
        share_recipe: recipe,
        ..op_pen_loader::EditorMeta::from_state(state)
    };
    let bytes = crate::doc_io::canonical_document_bytes(&state.doc, meta)
        .map_err(|e| ExportError::Write(e.to_string()))?;
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| ExportError::Write(e.to_string()))?;
    sanitize_document_value(&mut value, options.document_dir.as_deref());
    serde_json::to_string(&value).map_err(|e| ExportError::Write(e.to_string()))
}

/// The document name, else the first board's name, else the product.
fn share_title(state: &EditorState, slides: &[ShareSlide]) -> String {
    state
        .doc
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| {
            state
                .editor_ui
                .share_recipe()
                .map(|recipe| recipe.brief.chars().take(24).collect::<String>())
                .filter(|brief| !brief.trim().is_empty())
        })
        .or_else(|| slides.first().map(|slide| slide.name.clone()))
        .unwrap_or_else(|| "OpenPencil".to_string())
}

/// The save picker's suggested file name: the document name made safe
/// for every filesystem, as `.html`.
pub fn share_file_name(state: &EditorState) -> String {
    let base = state
        .doc
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("openpencil-share");
    let safe: String = base
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect();
    format!("{safe}.html")
}

#[cfg(test)]
#[path = "export_share_tests.rs"]
mod tests;
