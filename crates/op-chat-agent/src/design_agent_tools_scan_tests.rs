//! Scan-specific design-agent tests (icon issues, duplicate status bars,
//! ring issues, duplicate roots, decorative empty shells). Split out of
//! `design_agent_tools.rs` to keep that file under the 800-line cap; the
//! five former per-topic `mod`s are flattened into this one child module,
//! so `super::` still names `design_agent_tools`.

use super::*;

#[test]
fn family_name_as_glyph_and_missing_glyph_are_echoed() {
    // test0711-1.op regression: every icon shipped as
    // iconFontName:"lucide" (family in the glyph field) → fallback dots.
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "R", "width": 100, "height": 100,
          "children": [
            { "type": "icon_font", "id": "bad1", "iconFontName": "lucide",
              "width": 20, "height": 20 },
            { "type": "icon_font", "id": "bad2", "iconFontName": "",
              "width": 20, "height": 20 },
            { "type": "icon_font", "id": "bad3", "iconFontName": "material symbols rounded",
              "width": 20, "height": 20 },
            { "type": "icon_font", "id": "ok", "iconFontName": "compass",
              "width": 20, "height": 20 },
            { "type": "icon_font", "id": "feather-glyph", "iconFontFamily": "lucide",
              "iconFontName": "feather",
              "width": 20, "height": 20 }
          ] }
    ]))
    .expect("nodes");
    let issues = scan_icon_issues(&nodes);
    assert_eq!(issues.len(), 3, "{issues:?}");
    assert!(issues[0].contains("bad1") && issues[0].contains("lucide"));
    assert!(issues[1].contains("bad2") && issues[1].contains("missing"));
    assert!(
        issues[2].contains("bad3") && issues[2].contains("material symbols rounded"),
        "{issues:?}"
    );
    assert!(!issues.iter().any(|i| i.contains("\"ok\"")), "{issues:?}");
    assert!(
        !issues.iter().any(|i| i.contains("feather-glyph")),
        "real Lucide feather glyph must remain valid: {issues:?}"
    );
}

#[test]
fn standard_brand_and_icon_actions_header_is_not_reported() {
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "Home", "layout": "vertical",
          "width": 390, "height": 844, "children": [
            { "type": "frame", "id": "header", "name": "Header",
              "layout": "horizontal", "justifyContent": "space_between",
              "width": "fill_container", "height": 64, "children": [
                { "type": "text", "id": "brand", "name": "Brand Title",
                  "content": "NOVA", "width": 100, "height": 28 },
                { "type": "frame", "id": "actions", "name": "Header Actions",
                  "layout": "horizontal", "width": 72, "height": 24, "children": [
                    { "type": "icon_font", "id": "search", "iconFontName": "search",
                      "width": 20, "height": 20 },
                    { "type": "icon_font", "id": "cart", "iconFontName": "shopping-cart",
                      "width": 20, "height": 20 }
                  ] }
              ] }
          ] }
    ]))
    .expect("nodes");

    let issues = scan_header_icon_row_issues(&nodes);

    assert!(
        issues.is_empty(),
        "standard [brand, actions] header: {issues:?}"
    );
}

#[test]
fn title_outside_a_nonstandard_header_icon_row_is_still_reported() {
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "Home", "layout": "vertical",
          "width": 390, "height": 844, "children": [
            { "type": "frame", "id": "header", "name": "Header",
              "layout": "vertical", "justifyContent": "space_between",
              "width": "fill_container", "height": 88, "children": [
                { "type": "text", "id": "title", "name": "Greeting",
                  "content": "Good evening", "width": 180, "height": 28 },
                { "type": "frame", "id": "icons", "name": "Header Icon Row",
                  "layout": "horizontal", "width": 72, "height": 24, "children": [
                    { "type": "icon_font", "id": "bell", "iconFontName": "bell",
                      "width": 20, "height": 20 }
                  ] }
              ] }
          ] }
    ]))
    .expect("nodes");

    let issues = scan_header_icon_row_issues(&nodes);

    assert_eq!(
        issues.len(),
        1,
        "nonstandard header remains actionable: {issues:?}"
    );
    assert!(issues[0].contains("Header Icon Row"), "{issues:?}");
    assert!(
        issues[0].contains("M() the title INTO this row"),
        "{issues:?}"
    );
}

#[test]
fn nested_hand_built_status_bar_is_removed_once_canonical_exists() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "root", "name": "Music Home",
                "width": 402, "height": 874, "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
                      "width": "fill_container", "height": 62,
                      "children": [
                        { "type": "text", "id": "time", "name": "Time", "content": "9:41",
                          "width": 54, "height": 22 },
                        { "type": "frame", "id": "lv", "name": "Levels", "width": 70, "height": 22 }
                      ] },
                    { "type": "frame", "id": "hdr", "name": "Header",
                      "width": "fill_container", "height": "fit_content",
                      "children": [
                        { "type": "frame", "id": "fake", "name": "Status Bar 2",
                          "width": "fill_container", "height": 44 },
                        { "type": "text", "id": "greet", "name": "Greeting",
                          "content": "Good evening", "width": 200, "height": 30 }
                      ] }
                ]
            }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let removed = remove_nested_duplicate_status_bars(&mut state);
    assert_eq!(removed, 1, "the nested hand-built bar is swept");
    let root = &state.active_children()[0];
    fn find<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find(c, id))
    }
    assert!(find(root, "sb").is_some(), "canonical bar survives");
    assert!(find(root, "fake").is_none(), "nested duplicate removed");
    assert!(find(root, "greet").is_some(), "siblings untouched");
}

#[test]
fn hairline_ring_cluster_is_echoed_and_thick_rings_are_not() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "root", "name": "Screen",
                "width": 390, "height": 844,
                "children": [
                    { "type": "frame", "id": "ring", "name": "ActivityRing",
                      "width": 140, "height": 140, "layout": "none",
                      "children": [
                        { "type": "ellipse", "id": "e1", "width": 120, "height": 120,
                          "stroke": { "thickness": 1 } },
                        { "type": "ellipse", "id": "e2", "width": 120, "height": 120,
                          "stroke": { "thickness": 1 } }
                      ] },
                    { "type": "frame", "id": "ok", "name": "HealthyRing",
                      "width": 140, "height": 140, "layout": "none",
                      "children": [
                        { "type": "ellipse", "id": "e3", "width": 120, "height": 120,
                          "stroke": { "thickness": 10 } },
                        { "type": "ellipse", "id": "e4", "width": 120, "height": 120,
                          "stroke": { "thickness": 10 } }
                      ] }
                ]
            }] }"##,
    )
    .expect("doc");
    let issues = scan_ring_issues(&doc.children);
    assert_eq!(issues.len(), 1, "one cluster echoed: {issues:?}");
    assert!(
        issues[0].contains("ring") && issues[0].contains("thickness 8-12"),
        "echo names the cluster and the fix: {issues:?}"
    );
}

#[test]
fn missing_ring_wrapper_is_echoed_by_builtin_scan() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
                "type": "frame", "id": "steps-ring", "name": "Steps Ring",
                "width": 124, "height": 124, "layout": "vertical",
                "alignItems": "center", "justifyContent": "center",
                "children": [
                    {"type": "text", "id": "value", "content": "8,432"},
                    {"type": "text", "id": "label", "content": "steps"}
                ]
            }] }"##,
    )
    .expect("doc");

    let issues = scan_ring_issues(&doc.children);

    assert_eq!(issues.len(), 1, "one missing ring echoed: {issues:?}");
    assert!(issues[0].contains("missing-progress-ring"), "{issues:?}");
}

#[test]
fn same_named_top_level_frames_are_echoed_once_per_name() {
    // test0711-1-m3.op shape: model abandoned the original `Explore`
    // (empty AppContent) and rebuilt everything in a second `Explore`.
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "r1", "name": "Explore", "width": 390, "height": 844,
          "children": [ { "type": "frame", "id": "empty", "name": "AppContent",
                           "width": "fill_container", "height": "fit_content" } ] },
        { "type": "frame", "id": "r2", "name": "Explore", "width": 390,
          "height": "fit_content",
          "children": [ { "type": "frame", "id": "rich", "name": "AppContent",
                           "width": "fill_container", "height": "fit_content" } ] },
        { "type": "frame", "id": "solo", "name": "Profile", "width": 390, "height": 844,
          "children": [] }
    ]))
    .expect("nodes");
    let issues = scan_duplicate_root_issues(&nodes);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].contains("Explore") && issues[0].contains("r1") && issues[0].contains("r2"));
    assert!(issues[0].contains("M()") && issues[0].contains("D()"));
    assert!(!issues[0].contains("Profile"));
}

#[test]
fn deck_back_layers_are_exempted_as_decorative_stack() {
    // 0724-1-gm-3.op shape: a Flashcard Deck Stack under layout:none —
    // Front Flashcard (0,0,338x124, has text children) with two
    // childless "peek" layers behind it (Back Layer 1 painted, Back
    // Layer 2 unpainted), both offset a few px and near-identical size.
    // Neither back layer is an unfinished skeleton slot; both must be
    // exempted from the empty-shell blocker.
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "deck", "name": "Flashcard Deck Stack", "layout": "none",
          "width": 354, "height": 132,
          "children": [
            { "type": "frame", "id": "front", "name": "Front Flashcard",
              "x": 0, "y": 0, "width": 338, "height": 124,
              "children": [ { "type": "text", "id": "t1", "content": "Hello" } ] },
            { "type": "frame", "id": "back1", "name": "Back Layer 1",
              "x": 8, "y": 4, "width": 338, "height": 124,
              "fill": [{"type": "solid", "color": "$--accent"}],
              "children": [] },
            { "type": "frame", "id": "back2", "name": "Back Layer 2",
              "x": 16, "y": 8, "width": 338, "height": 124,
              "children": [] }
          ] }
    ]))
    .expect("nodes");
    let issues = scan_empty_shells(&nodes);
    assert!(
        issues.is_empty(),
        "deck back layers must be exempted as decorative stack, got {issues:?}"
    );
}

#[test]
fn ordinary_empty_section_scaffold_still_reported() {
    // A childless named section under layout:none with NO overlapping
    // non-empty sibling — an unfinished skeleton slot, must still fire.
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "Root", "layout": "none",
          "width": 390, "height": 400,
          "children": [
            { "type": "frame", "id": "header", "name": "Header",
              "x": 0, "y": 0, "width": 390, "height": 60,
              "children": [ { "type": "text", "id": "t1", "content": "Title" } ] },
            { "type": "frame", "id": "empty-section", "name": "Empty Section",
              "x": 0, "y": 200, "width": 390, "height": 120,
              "children": [] }
          ] }
    ]))
    .expect("nodes");
    let issues = scan_empty_shells(&nodes);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].contains("Empty Section"));
}

#[test]
fn empty_frame_under_vertical_layout_still_reported() {
    // Same overlap-shaped geometry, but the parent is auto-layout
    // (layout:vertical) — the decorative-stack exemption never applies
    // there since flowed children can't legitimately overlap.
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "Root", "layout": "vertical",
          "width": 390, "height": 400,
          "children": [
            { "type": "frame", "id": "front", "name": "Front Flashcard",
              "x": 0, "y": 0, "width": 338, "height": 124,
              "children": [ { "type": "text", "id": "t1", "content": "Hello" } ] },
            { "type": "frame", "id": "back1", "name": "Back Layer 1",
              "x": 8, "y": 4, "width": 338, "height": 124,
              "children": [] }
          ] }
    ]))
    .expect("nodes");
    let issues = scan_empty_shells(&nodes);
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(issues[0].contains("Back Layer 1"));
}

/// The injected status bar is OS chrome we author, and its battery is a
/// named frame of named childless rectangles. Descending into it reported
/// Border / Cap / Capacity as unfinished shells on every mobile screen —
/// blockers the model cannot act on and must not be asked to.
#[test]
fn status_bar_internals_are_not_empty_shells() {
    let nodes: Vec<PenNode> = serde_json::from_value(serde_json::json!([
        { "type": "frame", "id": "root", "name": "Home", "layout": "vertical",
          "width": 390, "height": 844,
          "children": [
            { "type": "frame", "id": "n3-status-bar", "name": "Status Bar",
              "role": "status-bar", "width": 390, "height": 62, "layout": "none",
              "children": [
                { "type": "frame", "id": "n3-status-bar-battery", "name": "Battery",
                  "x": 300, "y": 24, "width": 27, "height": 13, "layout": "none",
                  "children": [
                    { "type": "rectangle", "id": "n3-status-bar-battery-border",
                      "name": "Border", "x": 0, "y": 0, "width": 24, "height": 13,
                      "children": [] },
                    { "type": "rectangle", "id": "n3-status-bar-battery-capacity",
                      "name": "Capacity", "x": 2, "y": 2, "width": 18, "height": 9,
                      "children": [] }
                  ] }
              ] },
            { "type": "frame", "id": "empty-section", "name": "Empty Section",
              "x": 0, "y": 200, "width": 390, "height": 120, "children": [] }
          ] }
    ]))
    .expect("nodes");
    let issues = scan_empty_shells(&nodes);
    assert_eq!(
        issues.len(),
        1,
        "only the authored empty section is a shell: {issues:?}"
    );
    assert!(issues[0].contains("Empty Section"), "{issues:?}");
}

#[test]
fn design_quality_collection_is_read_only_and_keeps_categories_separate() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [
            { "type": "frame", "id": "root-a", "name": "Home",
              "width": 390, "height": 844, "layout": "vertical",
              "children": [
                { "type": "frame", "id": "empty", "name": "Content Shell",
                  "width": "fill_container", "height": 120, "children": [] },
                { "type": "icon_font", "id": "bad-icon", "iconFontName": "lucide",
                  "width": 20, "height": 20 }
              ] },
            { "type": "frame", "id": "root-b", "name": "Home",
              "width": 390, "height": 844, "children": [] }
        ] }"##,
    )
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let before = serde_json::to_string(&state.doc).expect("before");

    let report = collect_design_quality(&state);

    assert!(report
        .icon_issues
        .iter()
        .any(|issue| issue.contains("bad-icon")));
    assert!(report
        .structure_issues
        .iter()
        .any(|issue| issue.contains("duplicate top-level roots")));
    assert!(report
        .empty_shells
        .iter()
        .any(|shell| shell.contains("Content Shell")));
    assert_eq!(
        serde_json::to_string(&state.doc).expect("after"),
        before,
        "quality collection must not mutate the document"
    );
}
