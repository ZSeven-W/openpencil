//! `EditorState::import_svg` tests — split out of `svg_import.rs` to
//! keep both files under the repo's 800-line cap.

#![cfg(test)]

use crate::test_support::state_with;
use jian_ops_schema::node::PenNode;

#[test]
fn imports_basic_shapes() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg">
        <rect x="10" y="20" width="100" height="40" fill="#ff0000"/>
        <circle cx="50" cy="50" r="25"/>
        <ellipse cx="80" cy="80" rx="30" ry="10"/>
    </svg>"##;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    let n = s.import_svg(&mut next, svg, (0.0, 0.0));
    assert_eq!(n, 3);
    assert_eq!(s.active_children().len(), 3);
    // The rect kept its position + size.
    match &s.active_children()[0] {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(10.0));
            assert_eq!(r.base.y, Some(20.0));
        }
        other => panic!("expected rect, got {other:?}"),
    }
    // circle / ellipse imported as Ellipse nodes.
    assert!(matches!(&s.active_children()[1], PenNode::Ellipse(_)));
    assert!(matches!(&s.active_children()[2], PenNode::Ellipse(_)));
}

#[test]
fn imports_offset_translates_nodes() {
    let svg = r#"<svg><rect x="0" y="0" width="50" height="50"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (200.0, 100.0)), 1);
    match &s.active_children()[0] {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(200.0));
            assert_eq!(r.base.y, Some(100.0));
        }
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn imports_path_with_lines_and_cubic() {
    // A move + line + cubic + close. The cubic is flattened to dense
    // straight anchors at import time (no bezier handles) so the
    // renderer + pen-tool stay on the straight-segment polyline model.
    let svg = r#"<svg><path d="M0 0 L100 0 C100 50 50 100 0 100 Z"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    match &s.active_children()[0] {
        PenNode::Path(p) => {
            let anchors = p.anchors.as_ref().unwrap();
            // M + L + 24 flattened cubic samples = 26 anchors.
            assert_eq!(anchors.len(), 26);
            assert_eq!(p.closed, Some(true));
            // Flattened — no anchor carries bezier handles, so
            // `points` stays 1:1 with anchors for pen-tool editing.
            assert!(anchors
                .iter()
                .all(|a| a.handle_in.is_none() && a.handle_out.is_none()));
            // The first two anchors are the M / L endpoints.
            assert_eq!((anchors[0].x, anchors[0].y), (0.0, 0.0));
            assert_eq!((anchors[1].x, anchors[1].y), (100.0, 0.0));
            // The last cubic sample (t = 1) is the curve endpoint.
            let last = anchors.last().unwrap();
            assert!((last.x - 0.0).abs() < 1e-6 && (last.y - 100.0).abs() < 1e-6);
        }
        other => panic!("expected path, got {other:?}"),
    }
}

#[test]
fn curved_path_frame_covers_the_flattened_curve() {
    // A cubic peaking at y = 75 between two y=0 endpoints. Flattening
    // samples the curve at t = k/24, and t = 12/24 = 0.5 lands exactly
    // on the peak — so the dense anchors' bbox gives the Path a frame
    // height of 75, covering the curve rather than the y=0 chord.
    let svg = r#"<svg><path d="M0 0 C0 100 100 100 100 0"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    match &s.active_children()[0] {
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
    // m + relative l: pen ends at (10+5, 10+5) = (15,15).
    let svg = r#"<svg><path d="m10 10 l5 5"/></svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
    match &s.active_children()[0] {
        PenNode::Path(p) => {
            let a = p.anchors.as_ref().unwrap();
            assert_eq!((a[0].x, a[0].y), (10.0, 10.0));
            assert_eq!((a[1].x, a[1].y), (15.0, 15.0));
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
    match &s.active_children()[0] {
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
    // A doc with only a <g> wrapper + no shapes.
    assert_eq!(s.import_svg(&mut next, "<svg><g></g></svg>", (0.0, 0.0)), 0);
    assert_eq!(s.active_children().len(), 0);
}

#[test]
fn degenerate_shapes_are_skipped() {
    // Zero-size rect + zero-radius circle contribute nothing.
    let svg = r#"<svg>
        <rect x="0" y="0" width="0" height="40"/>
        <circle cx="5" cy="5" r="0"/>
        <rect x="0" y="0" width="20" height="20"/>
    </svg>"#;
    let mut s = state_with(vec![]);
    let mut next = 1u64;
    assert_eq!(s.import_svg(&mut next, svg, (0.0, 0.0)), 1);
}
