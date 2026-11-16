//! Default file names for the single-shot export actions (the
//! property panel's Export block and the File menu's Export row).
//!
//! Both hosts call [`default_export_file_name`]: desktop feeds it to
//! the rfd save dialog's pre-filled name, web feeds it to the browser
//! download. One function means the two can never drift into naming
//! the same export differently.
//!
//! The batch "export every frame" flow has its own planner
//! ([`crate::export_batch`]) because it names N files with an index
//! prefix; it shares this module's [`sanitize_name_component`] so the
//! character rules stay in one place.

use crate::pen_node_ext::PenNodeExt;
use crate::walkers::find_node;
use crate::EditorState;
use jian_ops_schema::node::PenNode;

/// Characters no mainstream filesystem accepts in a name (the Windows
/// set is the strict superset, so honoring it keeps exports portable).
const ILLEGAL_NAME_CHARS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Cap on one name component (the document stem, the node stem), in
/// characters. Two components plus a separator therefore stay well
/// inside every per-path limit, and a CJK node name — where one
/// character is three UTF-8 bytes — cannot blow past a byte-counted
/// limit either.
pub const MAX_NAME_COMPONENT_CHARS: usize = 60;

/// Stem used when the document has never been named or saved. Matches
/// the `untitled.op` convention the Save paths already use.
const UNTITLED_STEM: &str = "untitled";

/// Strip the characters a path cannot carry. Illegal characters and
/// control codes become `-`, runs of `-` collapse, and leading /
/// trailing separators, whitespace and dots are trimmed (a trailing
/// dot is itself illegal on Windows). The result is truncated to
/// `max_chars` **characters**, not bytes, so CJK names keep their
/// meaning instead of being cut mid-codepoint-budget.
///
/// Returns an empty string when nothing printable survives — every
/// caller substitutes its own fallback.
pub fn sanitize_name_component(name: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let replace = ILLEGAL_NAME_CHARS.contains(&ch) || ch.is_control();
        if replace {
            if !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(|c: char| c == '-' || c == '.' || c.is_whitespace());
    let capped: String = trimmed.chars().take(max_chars).collect();
    // Truncation can re-expose a trailing separator ("a-b-" cut at 4).
    capped
        .trim_end_matches(|c: char| c == '-' || c == '.' || c.is_whitespace())
        .to_string()
}

/// The document half of an export name: the bound file's name without
/// its extension, else the document's own name, else `untitled`.
///
/// `file_name_display` is the same string the TopBar shows, so the
/// exported file is named after what the user believes they are
/// editing.
pub fn document_export_stem(state: &EditorState) -> String {
    let from_file = state
        .editor_ui
        .file_name_display
        .as_deref()
        .map(strip_extension)
        .map(|stem| sanitize_name_component(stem, MAX_NAME_COMPONENT_CHARS))
        .filter(|stem| !stem.is_empty());
    if let Some(stem) = from_file {
        return stem;
    }
    let from_doc = state
        .doc
        .name
        .as_deref()
        .map(|name| sanitize_name_component(name, MAX_NAME_COMPONENT_CHARS))
        .filter(|stem| !stem.is_empty());
    from_doc.unwrap_or_else(|| UNTITLED_STEM.to_string())
}

/// Drop a trailing `.op` (or whatever extension the bound file
/// carries) from a display file name. Names without an extension, and
/// dotfile-shaped names, come back unchanged.
fn strip_extension(file_name: &str) -> &str {
    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => file_name,
    }
}

/// The node half of an export name, or `None` when the export is not
/// scoped to a single node.
///
/// The scope rule is the exporters' own: exactly one selected, real
/// node narrows a raster/SVG export to that subtree. A multi-selection
/// exports the whole page, so it deliberately gets no node suffix —
/// naming it after one arbitrary member would misdescribe the file.
pub fn selected_export_node_stem(state: &EditorState) -> Option<String> {
    if state.selection_count() != 1 || !state.selection.anchor.is_real() {
        return None;
    }
    let node = find_node(state.active_children(), &state.selection.anchor)?;
    Some(export_node_stem(node))
}

fn export_node_stem(node: &PenNode) -> String {
    let from_name = node
        .base()
        .name
        .as_deref()
        .map(|name| sanitize_name_component(name, MAX_NAME_COMPONENT_CHARS))
        .filter(|stem| !stem.is_empty());
    if let Some(stem) = from_name {
        return stem;
    }
    // Unnamed node: the kind reads better than a generated id, and the
    // id only has to carry the name when even that is unavailable.
    let from_id = sanitize_name_component(node.id_str(), MAX_NAME_COMPONENT_CHARS);
    if from_id.is_empty() {
        kind_stem(node).to_string()
    } else {
        format!("{}-{from_id}", kind_stem(node))
    }
}

/// Name a motion export after its captured root even when no layer was selected.
pub fn motion_export_file_name(
    state: &EditorState,
    request: &crate::motion_export::MotionExportRequest,
) -> String {
    let document = document_export_stem(state);
    let node = crate::NodeId::new_opt(request.root_id.clone())
        .and_then(|id| find_node(state.active_children(), &id))
        .map(export_node_stem);
    let extension = request.format.extension();
    match node {
        Some(node) if node != document => format!("{document}-{node}.{extension}"),
        _ => format!("{document}.{extension}"),
    }
}

/// File-name stem for the current export, without an extension.
///
/// `<document>-<node>` when the export is scoped to one node,
/// `<document>` otherwise (no selection, a multi-selection, or a
/// page-level format).
pub fn default_export_stem(state: &EditorState) -> String {
    let document = document_export_stem(state);
    if !node_scoped_format(state) {
        return document;
    }
    match selected_export_node_stem(state) {
        // A node named after the document would only stutter.
        Some(node) if node != document => format!("{document}-{node}"),
        _ => document,
    }
}

/// Whether the configured export format follows the single-node
/// selection. PDF is page-level (a deck exports its boards), so it
/// keeps the plain document name.
fn node_scoped_format(state: &EditorState) -> bool {
    state.editor_ui.export_format != crate::property_panel_state::ExportFormat::Pdf
}

/// The default file name both hosts offer for the current export:
/// [`default_export_stem`] plus the configured format's extension.
pub fn default_export_file_name(state: &EditorState) -> String {
    let extension = state.editor_ui.export_format.extension();
    format!("{}.{extension}", default_export_stem(state))
}

/// Lowercase kind word used to name an unnamed node.
fn kind_stem(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text-input",
        PenNode::TextArea(_) => "text-area",
        PenNode::Select(_) => "select",
        PenNode::Switch(_) => "switch",
        PenNode::Checkbox(_) => "checkbox",
        PenNode::Slider(_) => "slider",
        PenNode::RadioGroup(_) => "radio-group",
        PenNode::NumberInput(_) => "number-input",
        PenNode::Progress(_) => "progress",
        PenNode::Tabs(_) => "tabs",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon",
        PenNode::Ref(_) => "component",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_id::NodeId;
    use crate::property_panel_state::ExportFormat;

    fn state_from(json: &str) -> EditorState {
        let doc = jian_ops_schema::load_str(json)
            .expect("fixture JSON parses")
            .value;
        EditorState::from_document(doc)
    }

    fn sample_state() -> EditorState {
        let mut state = state_from(
            r#"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"星图","width":100,"height":100,"children":[
                    {"type":"text","id":"t1","name":"标题","content":"hi"}
                ]},
                {"type":"frame","id":"f2","width":100,"height":100}
            ]}"#,
        );
        state.editor_ui.file_name_display = Some("0808-k3-2.op".to_string());
        state
    }

    fn select(state: &mut EditorState, id: &str) {
        state.selection.set = vec![NodeId::new(id)];
        state.selection.anchor = NodeId::new(id);
    }

    #[test]
    fn a_single_selected_node_appends_its_name_to_the_document_name() {
        let mut state = sample_state();
        select(&mut state, "f1");
        assert_eq!(default_export_file_name(&state), "0808-k3-2-星图.png");
    }

    #[test]
    fn nested_selections_resolve_too() {
        let mut state = sample_state();
        select(&mut state, "t1");
        assert_eq!(default_export_file_name(&state), "0808-k3-2-标题.png");
    }

    #[test]
    fn no_selection_names_the_file_after_the_document_alone() {
        let state = sample_state();
        assert_eq!(default_export_file_name(&state), "0808-k3-2.png");
    }

    #[test]
    fn a_multi_selection_exports_the_page_and_keeps_the_document_name() {
        let mut state = sample_state();
        state.selection.set = vec![NodeId::new("f1"), NodeId::new("f2")];
        state.selection.anchor = NodeId::new("f1");
        assert_eq!(default_export_file_name(&state), "0808-k3-2.png");
    }

    #[test]
    fn the_format_drives_the_extension_and_pdf_stays_page_level() {
        let mut state = sample_state();
        select(&mut state, "f1");
        for (format, expected) in [
            (ExportFormat::Png, "0808-k3-2-星图.png"),
            (ExportFormat::Jpeg, "0808-k3-2-星图.jpg"),
            (ExportFormat::Webp, "0808-k3-2-星图.webp"),
            (ExportFormat::Svg, "0808-k3-2-星图.svg"),
            // PDF renders the page, never the selected subtree.
            (ExportFormat::Pdf, "0808-k3-2.pdf"),
        ] {
            state.editor_ui.export_format = format;
            assert_eq!(default_export_file_name(&state), expected);
        }
    }

    #[test]
    fn an_unnamed_node_falls_back_to_its_kind_and_id() {
        let mut state = sample_state();
        select(&mut state, "f2");
        assert_eq!(default_export_file_name(&state), "0808-k3-2-frame-f2.png");
    }

    #[test]
    fn a_missing_or_unreal_selection_anchor_names_the_document_alone() {
        let mut state = sample_state();
        select(&mut state, "gone");
        assert_eq!(default_export_file_name(&state), "0808-k3-2.png");
    }

    #[test]
    fn an_unsaved_document_uses_the_untitled_convention() {
        let mut state = sample_state();
        state.editor_ui.file_name_display = None;
        state.doc.name = None;
        select(&mut state, "f1");
        assert_eq!(default_export_file_name(&state), "untitled-星图.png");

        // A document name stands in for a file name that was never bound.
        state.doc.name = Some("Weekly Report".to_string());
        assert_eq!(default_export_file_name(&state), "Weekly Report-星图.png");
    }

    #[test]
    fn dangerous_characters_in_either_half_are_replaced() {
        let mut state = state_from(
            r#"{"version":"1.0.0","children":[
                {"type":"frame","id":"f1","name":"a/b:c*d?e\"f<g>h|i","width":10,"height":10}
            ]}"#,
        );
        state.editor_ui.file_name_display = Some("re:port/v2.op".to_string());
        select(&mut state, "f1");
        assert_eq!(
            default_export_file_name(&state),
            "re-port-v2-a-b-c-d-e-f-g-h-i.png"
        );
    }

    #[test]
    fn long_cjk_names_are_capped_per_component() {
        let long_node: String = "星".repeat(200);
        let long_doc: String = "月".repeat(200);
        let mut state = state_from(&format!(
            r#"{{"version":"1.0.0","children":[
                {{"type":"frame","id":"f1","name":"{long_node}","width":10,"height":10}}
            ]}}"#
        ));
        state.editor_ui.file_name_display = Some(format!("{long_doc}.op"));
        select(&mut state, "f1");

        let name = default_export_file_name(&state);
        let stem = name.trim_end_matches(".png");
        let (document, node) = stem.split_once('-').expect("both halves are present");
        assert_eq!(document.chars().count(), MAX_NAME_COMPONENT_CHARS);
        assert_eq!(node.chars().count(), MAX_NAME_COMPONENT_CHARS);
    }

    #[test]
    fn a_node_named_like_the_document_does_not_stutter() {
        let mut state = sample_state();
        state.editor_ui.file_name_display = Some("星图.op".to_string());
        select(&mut state, "f1");
        assert_eq!(default_export_file_name(&state), "星图.png");
    }

    #[test]
    fn sanitizing_matches_the_batch_planner_rules() {
        assert_eq!(sanitize_name_component("a/b:c*d", 60), "a-b-c-d");
        assert_eq!(sanitize_name_component("a//b", 60), "a-b");
        assert_eq!(sanitize_name_component("  spaced  ", 60), "spaced");
        assert_eq!(sanitize_name_component("trailing.", 60), "trailing");
        assert_eq!(sanitize_name_component("with\nnewline", 60), "with-newline");
        assert_eq!(sanitize_name_component("///", 60), "");
        // Truncation must not leave the cut edge on a separator.
        assert_eq!(sanitize_name_component("ab/cd", 3), "ab");
    }
}
