//! `EditorState::import_svg` tests — split out of `svg_import.rs` to
//! keep both files under the repo's 800-line cap.
//!
//! TS-parity import wraps every multi-node SVG in a `Group` (mirrors
//! `parseSvgToNodes`'s `wrapIfMultiple`); single-node imports land
//! flat. Tests honour that convention via the `import_root` helper
//! which transparently unwraps the Group when present.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{frame, state_with};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;

/// Return the imported top-level nodes regardless of whether they
/// were wrapped in a Group (multi-node SVG) or left flat (single
/// node). Mirrors `wrapIfMultiple` in the TS parser.
fn imported_nodes(s: &crate::EditorState) -> Vec<&PenNode> {
    let kids = s.active_children();
    if kids.len() == 1 {
        if let PenNode::Group(g) = &kids[0] {
            if let Some(inner) = g.children.as_ref() {
                return inner.iter().collect();
            }
        }
    }
    kids.iter().collect()
}

#[test]
fn imports_basic_shapes() {
    // No viewBox / width / height → fallback (vb 100×100, width 100,
    // height 100, scale = 1.0) so coords pass through unchanged.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="20" width="100" height="40" fill="#ff0000"/>
        <circle cx="50" cy="50" r="25"/>
        <ellipse cx="80" cy="80" rx="30" ry="10"/>
    </svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    let n = s.import_svg(&mut next, svg, (0.0, 0.0));
    assert_eq!(n, 3);
    let kids = imported_nodes(&s);
    assert_eq!(kids.len(), 3);
    match kids[0] {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(10.0));
            assert_eq!(r.base.y, Some(20.0));
        }
        other => panic!("expected rect, got {other:?}"),
    }
    assert!(matches!(kids[1], PenNode::Ellipse(_)));
    assert!(matches!(kids[2], PenNode::Ellipse(_)));
}

#[test]
fn imports_offset_translates_nodes() {
    let svg = r#"<svg><rect x="0" y="0" width="50" height="50"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (200.0, 100.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(200.0));
            assert_eq!(r.base.y, Some(100.0));
        }
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn import_svg_command_can_insert_under_parent() {
    let mut s = state_with(vec![frame("n1", "Frame", 0.0, 0.0, 400.0, 400.0, vec![])]);

    assert!(s.apply(EditorCommand::ImportSvg {
        svg: r#"<svg><rect width="10" height="10"/></svg>"#.into(),
        x: 0,
        y: 0,
        target_parent: crate::NodeId::new("n1"),
        page_id: None,
    }));

    assert_eq!(s.active_children().len(), 1);
    let frame = &s.active_children()[0];
    let children = frame.children().expect("parent children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].base().name.as_deref(), Some("Imported SVG"));
}

#[test]
fn import_svg_command_can_insert_on_requested_page_without_switching_active_page() {
    let mut s = state_with(vec![]);
    s.doc.pages = Some(vec![
        PenPage {
            id: "page-1".into(),
            name: "Page 1".into(),
            children: vec![frame("n1", "Current", 0.0, 0.0, 400.0, 400.0, vec![])],
            background_color: None,
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-2".into(),
            name: "Page 2".into(),
            children: Vec::new(),
            background_color: None,
            state: None,
            lifecycle: None,
        },
    ]);
    s.ui.active_page_index = 0;
    s.set_single_selection(crate::NodeId::new("n1"));

    assert!(s.apply(EditorCommand::ImportSvg {
        svg: r#"<svg><rect width="10" height="10"/></svg>"#.into(),
        x: 0,
        y: 0,
        target_parent: crate::NodeId::NONE,
        page_id: Some("page-2".into()),
    }));

    let pages = s.doc.pages.as_ref().expect("pages");
    assert_eq!(pages[0].children.len(), 1);
    assert_eq!(pages[1].children.len(), 1);
    assert_eq!(
        pages[1].children[0].base().name.as_deref(),
        Some("Imported SVG")
    );
    assert_eq!(s.ui.active_page_index, 0);
    assert_eq!(s.selection.anchor.as_str(), "n1");
}

#[test]
fn imports_path_with_lines_and_cubic() {
    // SVG path import preserves the source `d` so Canvas/Skia keeps
    // cubic commands, arcs and compound subpaths intact.
    let svg = r#"<svg><path d="M0 0 L100 0 C100 50 50 100 0 100 Z"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Path(p) => {
            let d = p.d.as_deref().expect("path d");
            assert!(d.contains('C'), "cubic command must survive: {d}");
            assert!(p.anchors.as_ref().is_none_or(Vec::is_empty));
            assert_eq!(p.closed, Some(true));
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn multiple_subpaths_remain_one_compound_path() {
    // `M ... M ...` is SVG pen-up; keeping the source `d` preserves
    // both the moveto break and compound fill rules.
    let svg = r#"<svg><path d="M0 0 L10 0 M50 50 L60 50"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    assert_eq!(kids.len(), 1);
    match kids[0] {
        PenNode::Path(p) => {
            let d = p.d.as_deref().expect("compound d");
            assert_eq!(d.matches('M').count(), 2);
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn curved_path_frame_covers_the_flattened_curve() {
    let svg = r#"<svg><path d="M0 0 C0 100 100 100 100 0"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Path(p) => {
            assert_eq!(p.base.y, Some(0.0));
            match &p.height {
                Some(jian_ops_schema::sizing::SizingBehavior::Number(h)) => {
                    assert!(
                        (h - 75.0).abs() < 1e-6,
                        "frame height should cover the curve peak (~75), got {h}"
                    );
                }
                other => panic!("expected numeric height, got {other:?}"),
            }
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn relative_path_commands_resolve() {
    let svg = r#"<svg><path d="m10 10 l5 5"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Path(p) => {
            let d = p.d.as_deref().expect("path d");
            assert_eq!(d, "M 0 0 L 5 5");
            assert_eq!(p.base.x, Some(10.0));
            assert_eq!(p.base.y, Some(10.0));
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn polygon_imports_as_closed_path() {
    let svg = r#"<svg><polygon points="0,0 10,0 10,10"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Path(p) => {
            assert_eq!(p.anchors.as_ref().unwrap().len(), 3);
            assert_eq!(p.closed, Some(true));
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn empty_or_unsupported_svg_imports_nothing() {
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, "", (0.0, 0.0)), 0);
    assert_eq!(s.import_svg(&mut next, "<svg><g></g></svg>", (0.0, 0.0)), 0);
    assert_eq!(s.active_children().len(), 0);
}

#[test]
fn degenerate_shapes_are_skipped() {
    let svg = r#"<svg>
        <rect x="0" y="0" width="0" height="40"/>
        <circle cx="5" cy="5" r="0"/>
        <rect x="0" y="0" width="20" height="20"/>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
}

#[test]
fn viewbox_caps_oversized_svg_at_max_dim() {
    // A 1024-unit SVG with no explicit width — viewBox-aware
    // sizing caps the output at maxDim (400 px); a unit shape in
    // the viewBox lands scaled accordingly. TS parity with
    // `parseSvgDimensions`'s `if (outW > maxDim ...)` cap.
    let svg = r##"<svg viewBox="0 0 1024 1024"><rect x="0" y="0" width="1024" height="1024" fill="#000"/></svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    match kids[0] {
        PenNode::Rectangle(r) => {
            // scale = 400/1024 ≈ 0.39 → width ≈ 400.
            match &r.container.width {
                Some(jian_ops_schema::sizing::SizingBehavior::Number(w)) => {
                    assert!(
                        (*w - 400.0).abs() < 1.0,
                        "expected capped width ≈ 400, got {w}"
                    );
                }
                other => panic!("expected numeric width, got {other:?}"),
            }
        }
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn group_wraps_multi_node_svg() {
    // Multi-node SVG must land as a single Group so the layer panel
    // shows one row instead of N flat siblings.
    let svg = r##"<svg>
        <rect x="0" y="0" width="10" height="10"/>
        <rect x="20" y="0" width="10" height="10"/>
    </svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 2);
    assert_eq!(s.active_children().len(), 1);
    match &s.active_children()[0] {
        PenNode::Group(g) => {
            assert_eq!(g.children.as_ref().unwrap().len(), 2);
        }
        other => panic!("expected Group wrap, got {other:?}"),
    }
}

#[test]
fn g_children_inherit_parent_fill() {
    // `<g fill="#0000ff">` with a `<path>` that has no `fill` of its
    // own — the path must paint blue, mirroring `mergeStyleCtxAttrs`.
    let svg = r##"<svg>
        <g fill="#0000ff">
            <path d="M0 0 L10 0 L10 10 Z"/>
        </g>
    </svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let path = match kids[0] {
        PenNode::Path(p) => p,
        other => panic!("expected path, got {other:?}"),
    };
    let fill = path.fill.as_ref().expect("inherited fill");
    let first = fill.first().expect("at least one fill");
    let hex = match first {
        jian_ops_schema::style::PenFill::Solid(b) => &b.color,
        other => panic!("expected solid fill, got {other:?}"),
    };
    assert!(hex.eq_ignore_ascii_case("#0000ff"), "got {hex}");
}

#[test]
fn filled_arc_path_preserves_svg_d_and_default_black_fill() {
    let svg = r#"<svg viewBox="0 0 100 100" width="100" height="100">
        <path d="M10 50 A40 40 0 0 1 90 50 Z"/>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let path = match kids[0] {
        PenNode::Path(p) => p,
        other => panic!("expected path, got {other:?}"),
    };
    let d = path.d.as_deref().expect("imported SVG path d");
    assert!(d.contains('A'), "arc command must survive, got {d}");
    assert!(path.anchors.as_ref().is_none_or(Vec::is_empty));
    let fill = path.fill.as_ref().expect("default SVG fill");
    let first = fill.first().expect("at least one fill");
    let hex = match first {
        jian_ops_schema::style::PenFill::Solid(b) => &b.color,
        other => panic!("expected solid fill, got {other:?}"),
    };
    assert!(hex.eq_ignore_ascii_case("#000000"), "got {hex}");
}

#[test]
fn compound_filled_path_stays_single_path_node() {
    let svg = r#"<svg viewBox="0 0 20 20" width="20" height="20">
        <path d="M0 0 H20 V20 H0 Z M5 5 H15 V15 H5 Z"/>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    assert_eq!(kids.len(), 1);
    let path = match kids[0] {
        PenNode::Path(p) => p,
        other => panic!("expected single compound path, got {other:?}"),
    };
    let d = path.d.as_deref().expect("compound path d");
    assert_eq!(
        d.matches('M').count(),
        2,
        "subpaths must stay in one d: {d}"
    );
}

#[test]
fn path_fill_rule_imports_from_attribute_style_and_group_inheritance() {
    use jian_ops_schema::node::path::PathFillRule;

    for (svg, expected) in [
        (
            r#"<svg><path fill-rule="evenodd" d="M0 0H20V20H0Z M5 5H15V15H5Z"/></svg>"#,
            PathFillRule::Evenodd,
        ),
        (
            r#"<svg><path style="fill-rule: nonzero" d="M0 0H20V20H0Z"/></svg>"#,
            PathFillRule::Nonzero,
        ),
        (
            r#"<svg><g style="fill-rule:evenodd"><path d="M0 0H20V20H0Z M5 5H15V15H5Z"/></g></svg>"#,
            PathFillRule::Evenodd,
        ),
    ] {
        let mut state = state_with(vec![]);
        let mut next = 1u64;
        assert_eq!(state.import_svg(&mut next, svg, (0.0, 0.0)), 1);
        let nodes = imported_nodes(&state);
        let PenNode::Path(path) = nodes[0] else {
            panic!("expected path");
        };
        assert_eq!(path.fill_rule, Some(expected));
    }
}

// ── `transform` attributes and `<style>` sheets (issue #214) ────────
//
// Design-tool exports (MasterGo, Figma) lean on nested `<g transform>`,
// rotated shapes and class-based fills; before this the transforms were
// ignored and the classes dropped, so an export landed at the wrong
// place in the wrong colour.

fn size_of(node: &PenNode) -> (f64, f64) {
    (
        node.width_px().expect("width"),
        node.height_px().expect("height"),
    )
}

fn solid_hex(fill: &Option<Vec<jian_ops_schema::style::PenFill>>) -> String {
    match fill.as_ref().and_then(|f| f.first()) {
        Some(jian_ops_schema::style::PenFill::Solid(b)) => b.color.to_ascii_lowercase(),
        other => panic!("expected a solid fill, got {other:?}"),
    }
}

#[test]
fn translate_and_uniform_scale_compose_through_groups_and_keep_shapes() {
    let svg = r#"<svg>
        <g transform="translate(100 50)">
            <g transform="scale(2)">
                <rect x="5" y="5" width="10" height="20" transform="translate(1 2)"/>
                <circle cx="10" cy="10" r="5"/>
            </g>
        </g>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    // The two shapes sit inside nested `<g>`s, so the importer sees one
    // top-level node: the inner group carrying both.
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let kids: Vec<&PenNode> = match kids[0] {
        PenNode::Group(g) => g
            .children
            .as_ref()
            .expect("group children")
            .iter()
            .collect(),
        other => panic!("expected the nested group, got {other:?}"),
    };
    // (5+1, 5+2) scaled by 2 then moved by (100, 50)
    match kids[0] {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(112.0));
            assert_eq!(r.base.y, Some(64.0));
            assert_eq!(size_of(kids[0]), (20.0, 40.0));
        }
        other => panic!("expected the rect to stay a rect, got {other:?}"),
    }
    match kids[1] {
        PenNode::Ellipse(e) => {
            assert_eq!(e.base.x, Some(110.0));
            assert_eq!(e.base.y, Some(60.0));
        }
        other => panic!("expected the circle to stay an ellipse, got {other:?}"),
    }
}

#[test]
fn a_rotated_rect_becomes_a_path_with_the_rotated_corners() {
    // rotate(90) about the origin: (x, y) -> (-y, x). The 10×20 rect at
    // (0,0) lands at x ∈ [-20, 0], y ∈ [0, 10].
    let svg = r##"<svg><rect x="0" y="0" width="10" height="20" transform="rotate(90)" fill="#00ff00"/></svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let path = match kids[0] {
        PenNode::Path(p) => p,
        other => panic!("expected a path, got {other:?}"),
    };
    assert_eq!(path.base.name.as_deref(), Some("Rect"));
    assert_eq!(path.base.x, Some(-20.0));
    assert_eq!(path.base.y, Some(0.0));
    assert_eq!(size_of(kids[0]), (20.0, 10.0));
    assert_eq!(path.closed, Some(true));
    assert_eq!(solid_hex(&path.fill), "#00ff00");
}

#[test]
fn a_non_uniform_scale_traces_a_circle_as_cubics_and_a_matrix_moves_a_path() {
    let svg = r#"<svg>
        <circle cx="10" cy="10" r="10" transform="scale(2 1)"/>
        <path d="M0 0 H10 V10" transform="matrix(1 0 0 1 100 100)"/>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 2);
    let kids = imported_nodes(&s);
    let circle = match kids[0] {
        PenNode::Path(p) => p,
        other => panic!("expected the squashed circle as a path, got {other:?}"),
    };
    let d = circle.d.as_deref().expect("path d");
    assert!(
        !d.contains('A') && d.contains('C'),
        "arcs must be traced as cubics under an anisotropic scale: {d}"
    );
    let (w, h) = size_of(kids[0]);
    // r=10 circle scaled ×2 in x: 40 wide, 20 tall, from (0,0)
    assert!((w - 40.0).abs() < 1.0 && (h - 20.0).abs() < 1.0, "{w}×{h}");
    assert_eq!(circle.base.x, Some(0.0));
    let path = match kids[1] {
        PenNode::Path(p) => p,
        other => panic!("expected path, got {other:?}"),
    };
    assert_eq!(path.base.x, Some(100.0));
    assert_eq!(path.base.y, Some(100.0));
}

#[test]
fn a_rotated_path_keeps_its_shape_in_the_new_orientation() {
    // A horizontal 10-unit line rotated 90° is a vertical 10-unit line.
    let svg = r##"<svg><path d="M0 0 H10" transform="rotate(90)" stroke="#000"/></svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let (w, h) = size_of(kids[0]);
    assert!(w <= 1.0 && (h - 10.0).abs() <= 1.0, "{w}×{h}");
}

#[test]
fn stroke_width_follows_the_transform_scale() {
    let svg = r##"<svg><g transform="scale(3)"><line x1="0" y1="0" x2="10" y2="0" stroke="#000" stroke-width="2"/></g></svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    let kids = imported_nodes(&s);
    let stroke = match kids[0] {
        PenNode::Path(p) => p.stroke.as_ref().expect("stroke"),
        other => panic!("expected path, got {other:?}"),
    };
    match stroke.thickness {
        jian_ops_schema::style::StrokeThickness::Uniform(w) => assert_eq!(w, 6.0),
        ref other => panic!("expected a uniform stroke, got {other:?}"),
    }
}

#[test]
fn style_sheet_classes_and_ids_colour_the_shapes_they_select() {
    let svg = r##"<svg>
        <style>.brand { fill: #ff0000 } #hero { fill: #0000ff } rect { stroke: #00ff00; stroke-width: 4 }</style>
        <rect class="brand" x="0" y="0" width="10" height="10"/>
        <rect id="hero" class="brand" x="20" y="0" width="10" height="10"/>
        <rect x="40" y="0" width="10" height="10" fill="#123456" style="fill:#abcdef"/>
    </svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 3);
    let kids = imported_nodes(&s);
    let rect = |i: usize| match kids[i] {
        PenNode::Rectangle(r) => r,
        other => panic!("expected rect, got {other:?}"),
    };
    assert_eq!(solid_hex(&rect(0).container.fill), "#ff0000");
    assert_eq!(
        solid_hex(&rect(1).container.fill),
        "#0000ff",
        "#id outranks .class"
    );
    assert_eq!(
        solid_hex(&rect(2).container.fill),
        "#abcdef",
        "inline style outranks the sheet"
    );
    let stroke = rect(0).container.stroke.as_ref().expect("tag rule stroke");
    assert!(
        matches!(stroke.thickness, jian_ops_schema::style::StrokeThickness::Uniform(w) if w == 4.0)
    );
}
