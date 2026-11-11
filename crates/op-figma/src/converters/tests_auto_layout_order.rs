//! `order_children` tests: auto-layout children reach the converted frame
//! in flow order (OpenPencil mode) or keep their authored stack positions
//! and absolute intent (preserve mode).

use super::tests::{fresh_ctx, obj};
use super::*;
use jian_ops_schema::constraints::{HConstraint, VConstraint};
use jian_ops_schema::node::PenNode;

fn rect_child(name: &str, local_id: u32, position: &str) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("RECTANGLE".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(local_id)),
                ]),
            ),
            ("name", FigValue::Str(name.into())),
            (
                "parentIndex",
                obj(vec![("position", FigValue::Str(position.into()))]),
            ),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(10.0)),
                    ("y", FigValue::Float(10.0)),
                ]),
            ),
        ]),
        children: vec![],
    }
}

fn transform(x: f32, y: f32) -> FigValue {
    obj(vec![
        ("m00", FigValue::Float(1.0)),
        ("m01", FigValue::Float(0.0)),
        ("m02", FigValue::Float(x)),
        ("m10", FigValue::Float(0.0)),
        ("m11", FigValue::Float(1.0)),
        ("m12", FigValue::Float(y)),
    ])
}

fn add_fields(node: &mut TreeNode, fields: Vec<(&str, FigValue)>) {
    let FigValue::Object(pairs) = &mut node.figma else {
        unreachable!();
    };
    pairs.extend(fields.into_iter().map(|(key, value)| (key.into(), value)));
}

fn line_child(name: &str, local_id: u32, position: &str) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("LINE".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(local_id)),
                ]),
            ),
            ("name", FigValue::Str(name.into())),
            (
                "parentIndex",
                obj(vec![("position", FigValue::Str(position.into()))]),
            ),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(30.0)),
                    ("y", FigValue::Float(0.0)),
                ]),
            ),
            ("transform", transform(12.0, 7.0)),
        ]),
        children: vec![],
    }
}

fn auto_layout_frame(children: Vec<TreeNode>) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(100)),
                ]),
            ),
            ("name", FigValue::Str("stack".into())),
            ("stackMode", FigValue::Str("VERTICAL".into())),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(100.0)),
                    ("y", FigValue::Float(50.0)),
                ]),
            ),
        ]),
        // Tree-builder order: DESCENDING parentIndex.position, i.e.
        // z-order with the topmost (= last layout item) first.
        children,
    }
}

fn child_names(node: &PenNode) -> Vec<String> {
    match node {
        PenNode::Frame(f) => f
            .children
            .as_ref()
            .map(|cs| {
                cs.iter()
                    .map(|c| match c {
                        PenNode::Rectangle(r) => r.base.name.clone().unwrap_or_default(),
                        _ => String::new(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        other => panic!("expected Frame, got {other:?}"),
    }
}

#[test]
fn openpencil_mode_orders_auto_layout_children_in_flow_order() {
    // Flow (layout) order is A then B; the tree builder hands the
    // converter [B, A] (descending z). OpenPencil mode feeds a flex
    // solver, so the emitted children MUST be flow order [A, B].
    let tree = auto_layout_frame(vec![rect_child("B", 2, "\""), rect_child("A", 1, "!")]);
    let mut ctx = fresh_ctx();
    let node = convert_frame(&tree, None, &mut ctx);
    assert_eq!(child_names(&node), vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn preserve_mode_keeps_library_updater_content_above_background_vector() {
    // Real Ant Design fixture order: the tree builder hands us the topmost
    // content first and the large decorative vector second. Reversing this in
    // Preserve mode makes the pale 237x237 vector cover the card's labels.
    let mut content = rect_child("content", 2, "$");
    add_fields(&mut content, vec![("transform", transform(16.0, 16.0))]);
    let mut background = rect_child("Vector", 1, "#");
    add_fields(
        &mut background,
        vec![
            ("transform", transform(-72.0, 56.0)),
            ("stackPositioning", FigValue::Str("ABSOLUTE".into())),
        ],
    );
    let tree = auto_layout_frame(vec![content, background]);
    let mut ctx = fresh_ctx();
    ctx.layout_mode = FigLayoutMode::Preserve;
    let node = convert_frame(&tree, None, &mut ctx);
    assert_eq!(
        child_names(&node),
        vec!["content".to_string(), "Vector".to_string()]
    );
    let PenNode::Frame(frame) = node else {
        unreachable!();
    };
    let children = frame.children.as_deref().expect("converted children");
    let PenNode::Rectangle(content) = &children[0] else {
        unreachable!();
    };
    let PenNode::Rectangle(background) = &children[1] else {
        unreachable!();
    };
    assert_eq!((content.base.x, content.base.y), (Some(16.0), Some(16.0)));
    assert_eq!(
        (background.base.x, background.base.y),
        (Some(-72.0), Some(56.0))
    );
}

#[test]
fn openpencil_mode_separates_flow_and_absolute_stack_children() {
    let mut overlay = rect_child("Overlay", 2, "\"");
    add_fields(
        &mut overlay,
        vec![
            ("transform", transform(14.0, 9.0)),
            ("stackPositioning", FigValue::Str("ABSOLUTE".into())),
            ("horizontalConstraint", FigValue::Str("STRETCH".into())),
            ("verticalConstraint", FigValue::Str("MAX".into())),
            ("stackChildPrimaryGrow", FigValue::Float(1.0)),
            ("stackChildAlignSelf", FigValue::Str("STRETCH".into())),
        ],
    );
    // Descending tree/paint order. Conversion reverses this to the
    // ascending layout order A, Overlay, B without pulling Overlay
    // into the flex flow.
    let tree = auto_layout_frame(vec![
        rect_child("B", 3, "#"),
        overlay,
        rect_child("A", 1, "!"),
    ]);
    let mut ctx = fresh_ctx();
    let node = convert_frame(&tree, None, &mut ctx);
    let PenNode::Frame(frame) = node else {
        panic!("expected frame");
    };
    let children = frame.children.expect("converted children");
    assert_eq!(
        children
            .iter()
            .map(|node| match node {
                PenNode::Rectangle(rect) => rect.base.name.as_deref().unwrap_or(""),
                _ => "",
            })
            .collect::<Vec<_>>(),
        ["A", "Overlay", "B"]
    );

    for flow in [&children[0], &children[2]] {
        let PenNode::Rectangle(flow) = flow else {
            panic!("expected flow rectangle");
        };
        assert_eq!((flow.base.x, flow.base.y), (None, None));
        assert!(flow.base.constraints.is_none());
    }

    let PenNode::Rectangle(overlay) = &children[1] else {
        panic!("expected overlay rectangle");
    };
    assert_eq!((overlay.base.x, overlay.base.y), (Some(14.0), Some(9.0)));
    let constraints = overlay.base.constraints.expect("absolute constraints");
    assert_eq!(constraints.h, HConstraint::LeftRight);
    assert_eq!(constraints.v, VConstraint::Bottom);
    assert!(matches!(
        overlay.container.width,
        Some(SizingBehavior::Number(10.0))
    ));
    assert!(matches!(
        overlay.container.height,
        Some(SizingBehavior::Number(10.0))
    ));
}

#[test]
fn preserve_mode_keeps_authored_stack_positions_and_absolute_intent() {
    let mut overlay = rect_child("Overlay", 2, "\"");
    add_fields(
        &mut overlay,
        vec![
            ("transform", transform(14.0, 9.0)),
            ("stackPositioning", FigValue::Str("ABSOLUTE".into())),
            ("horizontalConstraint", FigValue::Str("CENTER".into())),
            ("verticalConstraint", FigValue::Str("STRETCH".into())),
            ("stackChildPrimaryGrow", FigValue::Float(1.0)),
            ("stackChildAlignSelf", FigValue::Str("STRETCH".into())),
        ],
    );
    let tree = auto_layout_frame(vec![
        rect_child("B", 3, "#"),
        overlay,
        rect_child("A", 1, "!"),
    ]);
    let mut ctx = fresh_ctx();
    ctx.layout_mode = FigLayoutMode::Preserve;
    let node = convert_frame(&tree, None, &mut ctx);
    let PenNode::Frame(frame) = node else {
        panic!("expected frame");
    };
    let children = frame.children.as_deref().expect("converted children");
    // Preserve retains canonical topmost-first paint order together with every
    // authored coordinate, including ordinary auto-layout children.
    assert_eq!(
        children
            .iter()
            .map(|node| match node {
                PenNode::Rectangle(rect) => rect.base.name.as_deref().unwrap_or(""),
                _ => "",
            })
            .collect::<Vec<_>>(),
        ["B", "Overlay", "A"]
    );
    for child in children {
        let PenNode::Rectangle(child) = child else {
            panic!("expected rectangle");
        };
        assert!(child.base.x.is_some());
        assert!(child.base.y.is_some());
        assert!(matches!(
            child.container.width,
            Some(SizingBehavior::Number(10.0))
        ));
        assert!(matches!(
            child.container.height,
            Some(SizingBehavior::Number(10.0))
        ));
    }
    let PenNode::Rectangle(overlay) = &children[1] else {
        unreachable!();
    };
    let constraints = overlay.base.constraints.expect("absolute constraints");
    assert_eq!(constraints.h, HConstraint::Center);
    assert_eq!(constraints.v, VConstraint::TopBottom);
}

#[test]
fn flow_line_clears_position_without_baking_it_into_endpoint() {
    let tree = auto_layout_frame(vec![line_child("Divider", 1, "!")]);
    let mut ctx = fresh_ctx();
    let node = convert_frame(&tree, None, &mut ctx);
    let PenNode::Frame(frame) = node else {
        panic!("expected frame");
    };
    let PenNode::Line(line) = &frame.children.as_deref().unwrap()[0] else {
        panic!("expected line");
    };
    assert_eq!((line.base.x, line.base.y), (None, None));
    assert_eq!((line.x2, line.y2), (Some(30.0), Some(0.0)));

    let mut preserve_ctx = fresh_ctx();
    preserve_ctx.layout_mode = FigLayoutMode::Preserve;
    let preserved = convert_frame(&tree, None, &mut preserve_ctx);
    let PenNode::Frame(frame) = preserved else {
        unreachable!();
    };
    let PenNode::Line(line) = &frame.children.as_deref().unwrap()[0] else {
        unreachable!();
    };
    assert_eq!((line.base.x, line.base.y), (Some(12.0), Some(7.0)));
    assert_eq!((line.x2, line.y2), (Some(30.0), Some(0.0)));
}

#[test]
fn preserve_plain_frame_keeps_disabled_mask_as_explicit_open_content() {
    let tree = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("frameMaskDisabled", FigValue::Bool(true)),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(100.0)),
                    ("y", FigValue::Float(80.0)),
                ]),
            ),
        ]),
        children: vec![],
    };
    let mut ctx = fresh_ctx();
    ctx.layout_mode = FigLayoutMode::Preserve;
    let PenNode::Frame(frame) = convert_frame(&tree, None, &mut ctx) else {
        panic!("expected frame");
    };
    assert_eq!(frame.container.clip_content, Some(false));
}

/// Issue #276: overlays that were topmost in Figma stay topmost (first)
/// while the flow children still come out in flow order.
#[test]
fn openpencil_mode_keeps_topmost_absolute_overlays_on_top() {
    let absolute = |name: &str, id: u32, pos: &str| {
        let mut node = rect_child(name, id, pos);
        add_fields(
            &mut node,
            vec![
                ("transform", transform(0.0, 0.0)),
                ("stackPositioning", FigValue::Str("ABSOLUTE".into())),
            ],
        );
        node
    };
    // Descending (topmost-first) tree order of [topNav, body, mask, sheet].
    let tree = auto_layout_frame(vec![
        absolute("actionSheet", 4, "$"),
        absolute("mask", 3, "#"),
        rect_child("body", 2, "\""),
        rect_child("topNav", 1, "!"),
    ]);
    let mut ctx = fresh_ctx();
    let node = convert_frame(&tree, None, &mut ctx);
    assert_eq!(
        child_names(&node),
        ["actionSheet", "mask", "topNav", "body"]
            .map(String::from)
            .to_vec()
    );
}
