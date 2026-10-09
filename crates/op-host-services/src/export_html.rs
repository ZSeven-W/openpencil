//! Deck slideshow export — the render + IO behind File ▸ "Export
//! slideshow HTML".
//!
//! Produces ONE `.html` file that presents the deck with no external
//! resources at all: every board is emitted as a tree of absolutely
//! positioned elements by [`crate::export_html_structured`], anything
//! that tree cannot express is embedded as a base64 `data:` PNG, and
//! the player's CSS + JS are inlined. A presenter can carry the file to
//! a machine with no network, no fonts and no OpenPencil installed and
//! still present — and, because the text is real text, still select and
//! copy a quote off a slide.
//!
//! Page order comes from [`active_page_boards`] — the same single source
//! the native slideshow presents from — so the exported deck advances in
//! exactly the order Preview does, and a board the author hid is skipped
//! in both.

use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::EditorState;
use std::path::Path;

use crate::export::ExportError;
use crate::export_html_structured::board_slide_markup;
use crate::export_html_template::{render_slideshow_page, SlideAsset};

/// Fallback browser-tab title for a deck whose document and first board
/// are both unnamed.
const DEFAULT_TITLE: &str = "OpenPencil";

/// What one export produced.
///
/// The fallback counts are not decoration: a deck that came out mostly
/// rastered looks fine but has lost its selectable text, and that is
/// worth being able to see without diffing the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeckHtmlExport {
    /// Slides written — what the host reports back to the user.
    pub slides: usize,
    /// Nodes emitted as real elements across the whole deck.
    pub structured_nodes: usize,
    /// Nodes that had to be embedded as a raster image instead.
    pub raster_fallbacks: usize,
}

/// Render the active page's boards into a self-contained slideshow at
/// `target`.
///
/// A board that fails to render aborts the whole export rather than
/// being dropped. In a batch export the user can count the files in the
/// folder and see a gap; here the artifact is a single file whose page
/// numbering would silently close over the hole, and the presenter would
/// only discover the missing slide on stage.
pub fn export_deck_html(state: &EditorState, target: &Path) -> Result<DeckHtmlExport, ExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_export_layout_scene(state);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;

    let mut slides = Vec::new();
    let mut summary = DeckHtmlExport::default();
    for board_id in active_page_boards(state) {
        // Hidden boards are skipped, not failed: hiding a board is the
        // author saying it is not part of the deck.
        let Some(node) = page.find(&board_id) else {
            return Err(ExportError::NodeNotFoundOnPage {
                node_id: board_id,
                page_id: page.id.clone(),
            });
        };
        if node.hidden {
            continue;
        }
        let markup = board_slide_markup(page, &board_id, board_name(state, &board_id))?;
        summary.structured_nodes += markup.structured_nodes;
        summary.raster_fallbacks += markup.raster_fallbacks();
        slides.push(SlideAsset {
            name: markup.name,
            width: markup.width,
            height: markup.height,
            body: markup.body,
        });
    }
    if slides.is_empty() {
        return Err(ExportError::NothingToExport);
    }
    summary.slides = slides.len();

    let html = render_slideshow_page(&deck_title(state, &slides), &slides);
    std::fs::write(target, html).map_err(|e| ExportError::Write(e.to_string()))?;
    Ok(summary)
}

/// The board's authored name, or its id when it has none — a slide with
/// no accessible label at all would be invisible to a screen reader.
///
/// Read from the document rather than the scene: the resolved
/// `SceneNode` carries geometry and paint, not the authored layer name.
///
/// Shared with `export_hyperframes`: a board must be labelled the same
/// in the slideshow a presenter opens and in the composition a renderer
/// walks, or the two artifacts disagree about what a slide is called.
pub(crate) fn board_name(state: &EditorState, board_id: &str) -> String {
    state
        .active_children()
        .iter()
        .find(|node| node.id_str() == board_id)
        .and_then(|node| node.base().name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(board_id)
        .to_string()
}

/// Browser-tab title: the document name when the file has one, else the
/// cover slide's name (which for a generated deck is the deck's own
/// title), else a neutral constant.
fn deck_title(state: &EditorState, slides: &[SlideAsset]) -> String {
    state
        .doc
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| slides.first().map(|slide| slide.name.clone()))
        .unwrap_or_else(|| DEFAULT_TITLE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::scene_template_catalog::TemplateScene;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!(
            "openpencil-deck-html-{tag}-{}-{nanos}.html",
            std::process::id()
        ));
        p
    }

    fn deck_state(source: &str) -> EditorState {
        let doc = jian_ops_schema::load_str(source)
            .expect("fixture JSON parses")
            .value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.scenario = Some(TemplateScene::Slides);
        state
    }

    /// Two boards of DIFFERENT sizes, so page order can be verified from
    /// the emitted slide dimensions rather than from names alone.
    fn two_board_deck() -> EditorState {
        deck_state(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"封面","x":0,"y":0,"width":40,"height":20,
                 "fill":[{"type":"solid","color":"#ff0000"}]},
                {"type":"frame","id":"f2","name":"步骤 1","x":100,"y":0,"width":30,"height":30,
                 "fill":[{"type":"solid","color":"#00ff00"}]}
            ]}"##,
        )
    }

    /// The `(width, height)` each slide container declares, in document
    /// order — the structured-markup equivalent of decoding each PNG's
    /// IHDR, which is how this used to be checked.
    fn slide_sizes(html: &str) -> Vec<(String, String)> {
        html.split("data-w=\"")
            .skip(1)
            .map(|rest| {
                let width = rest.split('"').next().expect("closing quote").to_string();
                let height = rest
                    .split("data-h=\"")
                    .nth(1)
                    .and_then(|h| h.split('"').next())
                    .expect("data-h follows data-w")
                    .to_string();
                (width, height)
            })
            .collect()
    }

    #[test]
    fn the_deck_lands_as_one_file_with_one_slide_container_per_board() {
        let state = two_board_deck();
        let path = temp_path("single-file");

        let written = export_deck_html(&state, &path).expect("deck exports");

        assert_eq!(written.slides, 2);
        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert_eq!(html.matches("class=\"slide").count(), 2);
        // Self-contained means no fetch of any kind can be pending.
        assert!(!html.contains("http://"), "{html}");
        assert!(!html.contains("https://"), "{html}");
        assert!(!html.contains("<link"), "{html}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_plain_frame_deck_needs_no_raster_fallback_at_all() {
        let state = two_board_deck();
        let path = temp_path("all-structured");

        let written = export_deck_html(&state, &path).expect("deck exports");

        assert_eq!(written.raster_fallbacks, 0);
        assert_eq!(written.structured_nodes, 2, "one element per board");
        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert!(
            !html.contains("data:image/png;base64,"),
            "solid frames must not be rastered: {html}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn slide_order_follows_document_child_order() {
        let state = two_board_deck();
        let path = temp_path("order");

        export_deck_html(&state, &path).expect("deck exports");

        let html = std::fs::read_to_string(&path).expect("export file exists");
        // 40x20 authored first, 30x30 second — the export must not sort
        // by canvas position (f2 sits to the right) or by size.
        assert_eq!(
            slide_sizes(&html),
            vec![
                ("40".to_string(), "20".to_string()),
                ("30".to_string(), "30".to_string())
            ]
        );
        // The board fills identify the same two slides independently of
        // their sizes, so a swap could not pass on dimensions alone.
        let red = html.find("rgb(255,0,0)").expect("first board fill");
        let green = html.find("rgb(0,255,0)").expect("second board fill");
        assert!(red < green, "{html}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_page_carries_its_own_player() {
        let state = two_board_deck();
        let path = temp_path("player");

        export_deck_html(&state, &path).expect("deck exports");

        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert!(html.contains("PageDown"), "no forward key binding");
        assert!(html.contains("PageUp"), "no backward key binding");
        assert!(html.contains("Home"), "no jump-to-first binding");
        assert!(html.contains("End"), "no jump-to-last binding");
        assert!(
            html.contains("addEventListener('click'"),
            "no click advance"
        );
        assert!(html.contains("1 / 2"), "no page counter");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn slide_text_lands_as_real_selectable_text_not_pixels() {
        let state = deck_state(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":400,"height":300,
                 "fill":[{"type":"solid","color":"#ffffff"}],"children":[
                   {"type":"text","id":"t1","x":20,"y":20,"width":300,"height":48,
                    "content":"Quarterly Review","fontSize":32,"fontWeight":"700",
                    "fill":[{"type":"solid","color":"#101828"}]}
                 ]}
            ]}"##,
        );
        let path = temp_path("real-text");

        let written = export_deck_html(&state, &path).expect("deck exports");

        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert_eq!(written.raster_fallbacks, 0);
        assert!(html.contains(">Quarterly Review<"), "{html}");
        assert!(html.contains("font-size:32px"), "{html}");
        assert!(html.contains("font-weight:700"), "{html}");
        assert!(html.contains("color:rgb(16,24,40)"), "{html}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_hidden_board_is_skipped_rather_than_failing_the_export() {
        let state = deck_state(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"one","x":0,"y":0,"width":40,"height":20,
                 "fill":[{"type":"solid","color":"#ff0000"}]},
                {"type":"frame","id":"f2","name":"skipped","x":100,"y":0,"width":30,"height":30,
                 "visible":false,"fill":[{"type":"solid","color":"#00ff00"}]},
                {"type":"frame","id":"f3","name":"two","x":200,"y":0,"width":25,"height":25,
                 "fill":[{"type":"solid","color":"#0000ff"}]}
            ]}"##,
        );
        let path = temp_path("hidden");

        let written = export_deck_html(&state, &path).expect("deck exports");

        assert_eq!(
            written.slides, 2,
            "the hidden board must not become a slide"
        );
        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert_eq!(html.matches("class=\"slide").count(), 2);
        assert!(!html.contains("aria-label=\"skipped\""), "{html}");
        assert!(
            html.contains("1 / 2"),
            "counter must exclude the hidden board"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_deck_with_no_visible_board_refuses_to_write_a_file() {
        let state = deck_state(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"gone","x":0,"y":0,"width":40,"height":20,
                 "visible":false,"fill":[{"type":"solid","color":"#ff0000"}]}
            ]}"##,
        );
        let path = temp_path("empty");

        let result = export_deck_html(&state, &path);

        assert_eq!(result, Err(ExportError::NothingToExport));
        assert!(!path.exists(), "a refused export must leave no file behind");
    }

    #[test]
    fn board_names_are_escaped_into_the_page() {
        let state = deck_state(
            r##"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"<script> & \"quotes\"","x":0,"y":0,
                 "width":40,"height":20,"fill":[{"type":"solid","color":"#ff0000"}]}
            ]}"##,
        );
        let path = temp_path("escaping");

        export_deck_html(&state, &path).expect("deck exports");

        let html = std::fs::read_to_string(&path).expect("export file exists");
        assert!(
            html.contains("aria-label=\"&lt;script&gt; &amp; &quot;quotes&quot;\""),
            "{html}"
        );
        // The board name doubles as the tab title when the document is
        // unnamed, so both sites have to be escaped.
        assert!(
            html.contains("<title>&lt;script&gt; &amp; &quot;quotes&quot;</title>"),
            "{html}"
        );
        assert!(!html.contains("<script> &"), "raw board name leaked");
        let _ = std::fs::remove_file(&path);
    }
}
