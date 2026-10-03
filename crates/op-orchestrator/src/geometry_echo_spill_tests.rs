//! Model-facing echo diagnostics (nav order, rail collapse, vertical spill)
//! plus the image-clamp, grow-to-fit and chip-row fixes.

use super::*;

#[test]
fn late_section_after_bottom_nav_is_echoed_for_the_model() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Explore",
            "width": 375, "height": "fit_content", "layout": "vertical",
            "children": [
                { "type": "frame", "id": "nav", "name": "Bottom Navigation Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 },
                { "type": "frame", "id": "hdr", "name": "Header & Search",
                  "width": "fill_container", "height": "fit_content" }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        issues
            .iter()
            .any(|i| i.contains("Header & Search") && i.contains("AFTER the bottom tab bar")),
        "late section must be echoed: {issues:?}"
    );
}

#[test]
fn desktop_roots_and_nav_last_mobile_roots_emit_no_nav_order_echo() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [
            { "type": "frame", "id": "m", "name": "Mobile", "width": 390, "height": 844,
              "children": [
                { "type": "frame", "id": "c", "name": "Content", "width": "fill_container",
                  "height": 400 },
                { "type": "frame", "id": "nav", "name": "Bottom Tab Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 }
              ] }
        ]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        !issues
            .iter()
            .any(|i| i.contains("AFTER the bottom tab bar")),
        "nav-last root must not echo: {issues:?}"
    );
}

/// `geometry_diagnostics`'s detect-only twin of `fix_rail_width_collapse`
/// against REAL jian layout — the same "Savings Goals" rail shape
/// `geometry_rail_collapse_tests.rs` proves the FIX for, but here just
/// checking the DIAGNOSTIC fires (this is what `geometry_echo` consumes;
/// the fixer stays the deterministic-net fallback, untouched).
#[test]
fn rail_width_collapse_is_echoed_for_the_model_under_real_layout() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "rail", "name": "Goals Rail", "layout": "horizontal",
            "width": 327, "height": "fit_content", "gap": 12,
            "children": [
                { "type": "frame", "id": "c1", "name": "Emergency Fund", "layout": "vertical",
                  "width": 200, "height": "fit_content" },
                { "type": "frame", "id": "c2", "name": "New Car", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content" },
                { "type": "frame", "id": "c3", "name": "Vacation", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content" }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        issues
            .iter()
            .any(|i| i.contains("New Car") && i.contains("collapsed to")),
        "New Car's fill_container width starved beside the 200px reference must be echoed: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.contains("Vacation") && i.contains("collapsed to")),
        "Vacation's fill_container width starved beside the 200px reference must be echoed: {issues:?}"
    );
    assert!(
        !issues.iter().any(|i| i.contains("Emergency Fund")),
        "the fixed-width REFERENCE card is never itself the violation: {issues:?}"
    );
}

/// GLM-5.2 measured (test0711-1.op): a 300px-tall image inside a 42px
/// "Avatar" strip painted across half the header. The width-overflow echo
/// is blind to the vertical axis — this echo covers it.
#[test]
fn image_much_taller_than_its_parent_is_echoed_vertically() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "avatar", "name": "Avatar",
                  "width": "fill_container", "height": 42, "layout": "horizontal",
                  "padding": [8, 0],
                  "children": [
                    { "type": "image", "id": "img", "name": "woman face headshot", "src": "",
                      "width": "fill_container", "height": 300 }
                  ] },
                { "type": "frame", "id": "body", "name": "Body",
                  "width": "fill_container", "height": "fill_container" }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        issues
            .iter()
            .any(|i| i.contains("woman face headshot") && i.contains("inflates")),
        "vertical spill must be echoed: {issues:?}"
    );
}

/// Padding on an implicitly clipped numeric artboard must not inflate its
/// viewport or be reported as an oversized child.
#[test]
fn numeric_root_padding_alone_is_not_echoed_as_vertical_spill() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "padding": [0, 0, 28, 0],
            "children": [
                { "type": "frame", "id": "body", "name": "Body",
                  "width": "fill_container", "height": 844 }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let rects = resolved_rects(&state);
    let resolved_root = rects.get("root").expect("root rect").h;
    assert_eq!(resolved_root, 844.0);

    let issues = super::geometry_diagnostics(&state);
    assert!(
        !issues
            .iter()
            .any(|issue| issue.contains("Screen (root): declared")),
        "numeric padding alone is not a vertical spill: {issues:?}"
    );
}

/// `clipContent` parents are intentional croppers — no vertical-spill noise.
#[test]
fn clipping_parent_suppresses_vertical_spill_echo() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "avatar", "name": "Avatar", "clipContent": true,
                  "width": 44, "height": 44, "layout": "horizontal",
                  "children": [
                    { "type": "image", "id": "img", "name": "man face headshot", "src": "",
                      "width": "fill_container", "height": 300 }
                  ] }
            ]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = super::geometry_diagnostics(&state);
    assert!(
        !issues
            .iter()
            .any(|i| i.contains("inflates") || i.contains("resolved")),
        "clipContent crops on purpose — no echo expected: {issues:?}"
    );
}

/// A 400x300 enrichment image inside a declared 358x170 card cover — jian
/// inflates the card instead of overflowing, so only the declared-size check
/// catches it. The image is retargeted to fill its slot.
#[test]
fn oversized_image_child_is_clamped_to_fill_its_slot() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "cover", "name": "Card Cover",
                  "width": 358, "height": 170, "layout": "vertical",
                  "children": [
                    { "type": "image", "id": "img", "name": "midnight city neon", "src": "",
                      "width": 400, "height": 300 }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let img = find_by_id(root, "img").expect("img");
    {
        use op_editor_core::PenNodeExt;
        assert!(
            img.width_px().is_none() && img.height_px().is_none(),
            "oversized image switches to fill_container on both axes"
        );
    }
}

/// test0711-22 00:44 shape: a fill×fill image inside a `layout:"none"`
/// Cover — `fill_container` is meaningless in an absolute container and the
/// engine painted the cover as a thin right-edge sliver. The image is
/// pinned to the parent's resolved rect.
#[test]
fn fill_image_in_absolute_container_is_pinned_to_parent_rect() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "cover", "name": "Cover",
                  "width": 160, "height": 160, "layout": "none", "clipContent": true,
                  "children": [
                    { "type": "image", "id": "img", "name": "album art", "src": "",
                      "width": "fill_container", "height": "fill_container" }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let img = find_by_id(root, "img").expect("img");
    {
        use op_editor_core::PenNodeExt;
        assert_eq!(img.width_px(), Some(160.0), "pinned to parent width");
        assert_eq!(img.height_px(), Some(160.0), "pinned to parent height");
    }
}

/// One-off forensic harness: `OP_FORENSIC_FILE=<path> cargo test -p
/// op-orchestrator forensic_resolved_rects -- --ignored --nocapture`
#[test]
#[ignore]
fn forensic_resolved_rects() {
    let Ok(path) = std::env::var("OP_FORENSIC_FILE") else {
        return;
    };
    let json = std::fs::read_to_string(&path).expect("read file");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&json).expect("parse");
    let mut state = op_editor_core::EditorState::from_document(doc);
    // `OP_FORENSIC_FIX=1` additionally runs the repair loop on every page
    // root, so the rect dump below shows the POST-fix geometry.
    if std::env::var("OP_FORENSIC_FIX").as_deref() == Ok("1") {
        use op_editor_core::PenNodeExt as _;
        let roots: Vec<String> = state
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();
        let mut sink = crate::test_support::VecDocSink::new();
        std::mem::swap(&mut sink.state, &mut state);
        for root in roots {
            let rounds = super::geometry_validate_and_fix(&mut sink, &root);
            eprintln!(
                "FIX ROUNDS for {root}: {rounds} ({} commands)",
                sink.applied.len()
            );
        }
        std::mem::swap(&mut sink.state, &mut state);
    }
    let issues = super::geometry_diagnostics(&state);
    eprintln!("DIAGNOSTICS ({}):", issues.len());
    for issue in &issues {
        eprintln!("  - {issue}");
    }
    let scene = op_pen_loader::editor_state_to_layout_scene(&state);
    fn dump(nodes: &[jian_scene::layout_scene::SceneNode], depth: usize) {
        for n in nodes {
            let b = n.aggregate_bounds();
            let kind = format!("{:?}", n.kind);
            eprintln!(
                "{}{} [{kind}] x={:.0} y={:.0} w={:.0} h={:.0}",
                "  ".repeat(depth),
                n.id,
                b.origin.x,
                b.origin.y,
                b.size.x,
                b.size.y
            );
            if depth < 4 {
                dump(&n.children, depth + 1);
            }
        }
    }
    for page in &scene.pages {
        dump(&page.children, 0);
    }
}

/// test0711-2-ds: a card row declared 156 tall whose children resolve 165 —
/// the 9px overshoot hid the artist line's bottom under the next section.
/// Small overshoots grow the frame; the big-inflation class stays an echo.
#[test]
fn slightly_short_fixed_frame_grows_to_fit_its_children() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "rail", "name": "Card Rail",
                  "width": "fill_container", "height": 156, "layout": "horizontal", "gap": 12,
                  "children": [
                    { "type": "frame", "id": "card", "width": 140, "height": 156,
                      "layout": "vertical", "gap": 8,
                      "children": [
                        { "type": "frame", "id": "cover", "width": 140, "height": 120 },
                        { "type": "text", "id": "t1", "content": "Blinding Lights",
                          "width": "fit_content", "height": 18 },
                        { "type": "text", "id": "t2", "content": "The Weeknd",
                          "width": "fit_content", "height": 15 }
                      ] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    super::geometry_validate_and_fix(&mut sink, "root");

    fn find_by_id<'a>(
        node: &'a jian_ops_schema::node::PenNode,
        id: &str,
    ) -> Option<&'a jian_ops_schema::node::PenNode> {
        use op_editor_core::PenNodeExt;
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let card = find_by_id(root, "card").expect("card");
    {
        use op_editor_core::PenNodeExt;
        assert!(
            card.height_px().is_some_and(|h| h > 156.0),
            "card grew to cover its children, got {:?}",
            card.height_px()
        );
    }
}

#[test]
fn narrow_value_and_chip_row_is_not_stacked_when_it_fits() {
    let row = json!({
        "type":"frame", "id":"row", "layout":"horizontal", "width":240, "height":48,
        "gap":8, "children":[
            {"type":"text", "id":"value", "content":"$48K", "fontSize":32, "width":100, "height":40},
            {"type":"frame", "id":"chip", "width":60, "height":28,
             "fill":[{"type":"solid","color":"#DCFCE7"}]}
        ]
    });
    let rects = HashMap::from([
        (
            "row".to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 240.0,
                h: 48.0,
            },
        ),
        (
            "value".to_string(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 40.0,
            },
        ),
        (
            "chip".to_string(),
            Rect {
                x: 108.0,
                y: 0.0,
                w: 60.0,
                h: 28.0,
            },
        ),
    ]);
    let mut commands = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut commands, false);

    assert!(
        commands.is_empty(),
        "narrow is not the same as overfull: {commands:?}"
    );
}
