//! Inline canvas text-edit interaction tests — click-to-place caret,
//! drag-select, Enter-newline, Escape-commit, outside-click commit.
//!
//! Geometry discipline: viewport 1440×900, fixtures placed at doc
//! x ≥ 400 so every screen press lands right of the AI chat float
//! (x ≤ 612) and the floating toolbar (x ≈ 252-300, y ≈ 60-400).
//! Press points derive from the node's LAYOUT-SCENE bounds (not the
//! authored x/y) so layout resolution can't skew the hit targets;
//! offset assertions stay metric-free (edges / line membership) so
//! they hold under any system font.

use super::WidgetHostNative;
use op_editor_core::NodeId;
use op_editor_ui::Rect;

const VIEWPORT_W: f32 = 1440.0;
const VIEWPORT_H: f32 = 900.0;

/// `t1` text node (400, 60, 200×60) with two authored lines.
const TWO_LINES: &str = r#"{"version":"1.0.0","children":[
  {"type":"text","id":"t1","name":"Label","x":400,"y":60,"width":200,"height":60,
   "content":"hello\nworld","fontSize":20,"lineHeight":1}
]}"#;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

/// The node's layout-resolved doc-space bounds.
fn node_bounds(host: &mut WidgetHostNative, id: &str) -> Rect {
    host.layout_scene()
        .active_page()
        .and_then(|p| p.find(id))
        .map(|n| n.bounds)
        .expect("node in scene")
}

/// Screen point for a doc point (zoom 1, pan 0 in a fresh host).
fn screen_at(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let (cx0, cy0, _cw, _ch) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    (cx0 + doc_x, cy0 + doc_y)
}

fn press_doc(host: &mut WidgetHostNative, doc_x: f32, doc_y: f32) {
    let (x, y) = screen_at(host, doc_x, doc_y);
    host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H);
}

fn release(host: &mut WidgetHostNative) {
    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
}

fn start_edit(host: &mut WidgetHostNative, id: &str) {
    assert!(host.editor_state_mut().start_text_edit(NodeId::new(id)));
    host.mark_paint_dirty_for_test();
}

fn caret(host: &WidgetHostNative) -> Option<usize> {
    host.editor_state()
        .ui
        .text_editing
        .as_ref()
        .map(|_| host.editor_state().ui.text_edit_input.caret())
}

fn selection(host: &WidgetHostNative) -> Option<(usize, usize)> {
    host.editor_state().text_edit_effective_selection(
        host.editor_state()
            .text_edit_content()
            .map(str::len)
            .unwrap_or(0),
    )
}

// --- Click-to-place caret ----------------------------------------------

#[test]
fn double_click_enters_edit_then_click_places_caret() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    let b = node_bounds(&mut host, "t1");
    // Node CENTRE — clear of the 8 selection-resize handles (±6 px
    // around the selected bounds edges) the first click activates.
    let (cx, cy) = (b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    // Double-click enters text edit (caret seeds at the content end).
    press_doc(&mut host, cx, cy);
    release(&mut host);
    press_doc(&mut host, cx, cy);
    release(&mut host);
    assert_eq!(host.editor_state().ui.text_editing, Some(NodeId::new("t1")));
    assert_eq!(caret(&host), Some(11));
    // A third press at the node's left edge places the caret at the
    // first line's start WITHOUT committing the session.
    host.set_now_ms(1_000); // step past the double-click window
    press_doc(&mut host, b.origin.x + 1.0, b.origin.y + 5.0);
    release(&mut host);
    assert_eq!(
        host.editor_state().ui.text_editing,
        Some(NodeId::new("t1")),
        "click inside the edited node must not commit"
    );
    assert_eq!(caret(&host), Some(0));
}

#[test]
fn click_right_of_first_line_clamps_to_line_end() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    let b = node_bounds(&mut host, "t1");
    // Far right inside the node, on the FIRST painted line.
    press_doc(&mut host, b.origin.x + b.size.x - 2.0, b.origin.y + 5.0);
    release(&mut host);
    assert_eq!(caret(&host), Some(5), "end of \"hello\"");
    // Same x on the SECOND line (line height 20 → y + 25).
    press_doc(&mut host, b.origin.x + b.size.x - 2.0, b.origin.y + 25.0);
    release(&mut host);
    assert_eq!(caret(&host), Some(11), "end of \"world\"");
}

#[test]
fn click_on_second_line_start_maps_past_newline() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    let b = node_bounds(&mut host, "t1");
    press_doc(&mut host, b.origin.x + 1.0, b.origin.y + 25.0);
    release(&mut host);
    assert_eq!(caret(&host), Some(6), "start of \"world\"");
}

#[test]
fn shift_click_extends_selection_from_caret() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1"); // caret at 11
    let b = node_bounds(&mut host, "t1");
    host.set_modifier_shift(true);
    press_doc(&mut host, b.origin.x + 1.0, b.origin.y + 5.0);
    release(&mut host);
    host.set_modifier_shift(false);
    assert_eq!(caret(&host), Some(0));
    assert_eq!(selection(&host), Some((0, 11)));
}

// --- Drag select ---------------------------------------------------------

#[test]
fn drag_across_first_line_selects_it() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    let b = node_bounds(&mut host, "t1");
    press_doc(&mut host, b.origin.x + 1.0, b.origin.y + 5.0);
    assert_eq!(caret(&host), Some(0));
    // Drag to the node's right edge on the same line.
    let (mx, my) = screen_at(&host, b.origin.x + b.size.x - 2.0, b.origin.y + 5.0);
    host.apply_cursor_move(mx, my);
    assert_eq!(selection(&host), Some((0, 5)), "first line selected");
    release(&mut host);
    // Selection survives the release; the drag state is cleared.
    assert_eq!(selection(&host), Some((0, 5)));
    let (mx2, my2) = screen_at(&host, b.origin.x + 1.0, b.origin.y + 25.0);
    host.apply_cursor_move(mx2, my2);
    assert_eq!(
        selection(&host),
        Some((0, 5)),
        "cursor moves after release must not keep extending"
    );
}

#[test]
fn drag_below_node_clamps_to_last_line() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    let b = node_bounds(&mut host, "t1");
    press_doc(&mut host, b.origin.x + 1.0, b.origin.y + 5.0);
    // Drag far below the node — line clamps to the last painted line,
    // x past the line end clamps to the content end.
    let (mx, my) = screen_at(&host, b.origin.x + b.size.x + 300.0, b.origin.y + 400.0);
    host.apply_cursor_move(mx, my);
    assert_eq!(selection(&host), Some((0, 11)), "select to content end");
    release(&mut host);
}

// --- Enter / Escape / outside click --------------------------------------

#[test]
fn enter_inserts_newline_instead_of_committing() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1"); // caret at end (11)
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().ui.text_editing,
        Some(NodeId::new("t1")),
        "Enter must keep the session open"
    );
    assert_eq!(
        host.editor_state().text_edit_content(),
        Some("hello\nworld\n")
    );
    assert_eq!(caret(&host), Some(12));
    // And typing after the newline lands on the new line.
    assert!(host.apply_text('!'));
    assert_eq!(
        host.editor_state().text_edit_content(),
        Some("hello\nworld\n!")
    );
}

#[test]
fn escape_commits_text_edit() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    assert!(host.apply_escape());
    assert_eq!(host.editor_state().ui.text_editing, None);
    assert_eq!(caret(&host), None);
}

#[test]
fn outside_click_commits_text_edit() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    // Blank canvas far from the node (and from every floating panel).
    press_doc(&mut host, 900.0, 500.0);
    release(&mut host);
    assert_eq!(
        host.editor_state().ui.text_editing,
        None,
        "outside click blurs + commits (TS textarea parity)"
    );
}

// --- Keyboard caret movement ---------------------------------------------

#[test]
fn horizontal_arrows_move_caret_and_never_nudge_node() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1"); // caret 11
    assert!(host.apply_text_edit_caret(false));
    assert_eq!(caret(&host), Some(10));
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(caret(&host), Some(11));
    // Clamped at the end — still consumed.
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(caret(&host), Some(11));
    // Not editing → not consumed (falls through to nudge handlers).
    assert!(host.editor_state_mut().text_edit_commit());
    assert!(!host.apply_text_edit_caret(true));
}

#[test]
fn vertical_arrows_move_by_visual_line() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1"); // caret 11 = line 1 col 5
    assert!(host.apply_text_edit_vertical(false));
    assert_eq!(caret(&host), Some(5), "line 0 keeps the char column");
    assert!(host.apply_text_edit_vertical(true));
    assert_eq!(caret(&host), Some(11));
    // Down past the last line clamps to the content end.
    assert!(host.apply_text_edit_vertical(true));
    assert_eq!(caret(&host), Some(11));
}

#[test]
fn shift_vertical_extends_selection() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1"); // caret 11
    host.set_modifier_shift(true);
    assert!(host.apply_text_edit_vertical(false));
    host.set_modifier_shift(false);
    assert_eq!(selection(&host), Some((5, 11)));
}

#[test]
fn line_edge_jumps_within_current_line() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    assert!(host.editor_state_mut().text_edit_set_caret(8, false, 0)); // line 1 col 2
    assert!(host.apply_text_edit_line_edge(false));
    assert_eq!(caret(&host), Some(6));
    assert!(host.apply_text_edit_line_edge(true));
    assert_eq!(caret(&host), Some(11));
}

#[test]
fn select_all_then_typing_replaces_content() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    assert!(host.apply_select_all());
    assert_eq!(selection(&host), Some((0, 11)));
    assert!(host.apply_text('X'));
    assert_eq!(host.editor_state().text_edit_content(), Some("X"));
    assert_eq!(caret(&host), Some(1));
}

// --- Deep-nesting drill chain (reported: generated mobile design) -------

/// 4-level fixture mirroring generated documents: screen frame >
/// header row > search pill (135×40 r20) > single-line text. Layout
/// attributes match what the generation pipeline authors, so the
/// resolved scene — not hand-placed x/y — decides where the text lands.
const DEEP_PILL: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"screen","name":"Screen","x":400,"y":60,"width":375,"height":300,
   "layout":"vertical","children":[
    {"type":"frame","id":"header","name":"Header","width":"fill_container","height":72,
     "layout":"horizontal","padding":[16,16],"gap":8,"alignItems":"center","children":[
      {"type":"frame","id":"pill","name":"SearchPill","width":135,"height":40,"cornerRadius":20,
       "layout":"horizontal","alignItems":"center","padding":[0,12],"children":[
        {"type":"text","id":"pill_text","name":"Placeholder",
         "content":"Search exhibitions","fontSize":16,"lineHeight":1}
      ]}
    ]}
  ]}
]}"#;

/// A single double-click on deeply nested text jumps straight into the
/// edit session — the pointer names the exact node, so the level-by-level
/// drill (which used to cost one double-click per nesting level) must not
/// gate text editing.
#[test]
fn double_click_on_deep_nested_text_jumps_straight_to_edit() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, DEEP_PILL);
    let b = node_bounds(&mut host, "pill_text");
    let (cx, cy) = (b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    press_doc(&mut host, cx, cy);
    release(&mut host);
    host.set_now_ms(100);
    press_doc(&mut host, cx, cy);
    release(&mut host);
    assert_eq!(
        host.editor_state().ui.text_editing,
        Some(NodeId::new("pill_text")),
        "first double-click must open the session regardless of nesting depth"
    );
    assert_eq!(
        host.editor_state().selection.anchor.as_str(),
        "pill_text",
        "selection jumps with the session"
    );
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("pill")),
        "scope lands on the text's parent so exiting the session is sane"
    );
}

/// Double-click over a container (no text at the deepest hit) keeps the
/// level-by-level drill — the fast path is text-only.
#[test]
fn double_click_on_container_still_drills_one_level() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, DEEP_PILL);
    // A point inside the header row but right of the pill: deepest hit
    // is the header frame, not a text leaf.
    let header = node_bounds(&mut host, "header");
    let pill = node_bounds(&mut host, "pill");
    let cx = pill.origin.x + pill.size.x + 40.0;
    let cy = header.origin.y + header.size.y / 2.0;
    press_doc(&mut host, cx, cy);
    release(&mut host);
    host.set_now_ms(100);
    press_doc(&mut host, cx, cy);
    release(&mut host);
    assert_eq!(
        host.editor_state().ui.text_editing,
        None,
        "a container double-click must not open a text session"
    );
}

#[test]
fn backspace_and_delete_are_caret_aware() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, TWO_LINES);
    start_edit(&mut host, "t1");
    assert!(host.editor_state_mut().text_edit_set_caret(6, false, 0)); // before 'w'
    assert!(host.apply_backspace()); // removes the '\n'
    assert_eq!(host.editor_state().text_edit_content(), Some("helloworld"));
    assert_eq!(caret(&host), Some(5));
    assert!(host.apply_delete()); // forward delete removes 'w'
    assert_eq!(host.editor_state().text_edit_content(), Some("helloorld"));
    assert_eq!(caret(&host), Some(5));
}
