//! `convert_vector` icon-lookup tests — exercise the host-resolver
//! branch (`set_icon_lookup` → name match → emit `Path` with `icon_id`).
//! Tests serialise on `LOOKUP_GUARD` because the resolver is
//! process-global state.

use super::*;
use crate::common::{clear_icon_lookup, set_icon_lookup, IconLookupResult, IconStyle};
use crate::figma_types::BlobOrString;
use crate::vector_decoder::compute_svg_path_bounds;
use jian_ops_schema::constraints::{HConstraint, VConstraint};
use jian_ops_schema::node::{MaskType, PenNode};
use std::collections::HashMap;
use std::sync::Mutex;

/// Serialise tests that touch the process-global icon-lookup so they
/// don't race with each other under cargo's parallel runner.
pub(super) static LOOKUP_GUARD: Mutex<()> = Mutex::new(());

pub(super) fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

pub(super) fn vector_node(name: &str) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("VECTOR".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(10)),
                ]),
            ),
            ("name", FigValue::Str(name.into())),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(24.0)),
                    ("y", FigValue::Float(24.0)),
                ]),
            ),
        ]),
        children: vec![],
    }
}

pub(super) fn fresh_ctx<'tree>() -> ConversionContext<'tree> {
    ConversionContext {
        component_map: HashMap::new(),
        symbol_tree: HashMap::new(),
        warnings: Vec::new(),
        id_counter: 1,
        blobs: Vec::new(),
        layout_mode: FigLayoutMode::OpenPencil,
        instance_assignments: HashMap::new(),
        instance_expansions: Default::default(),
    }
}

pub(super) fn captured_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0, "captured fixture must contain byte pairs");
    (0..hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).expect("fixture hex"))
        .collect()
}

pub(super) fn solid_paint(r: f32, g: f32, b: f32) -> FigValue {
    obj(vec![
        ("type", FigValue::Str("SOLID".into())),
        (
            "color",
            obj(vec![
                ("r", FigValue::Float(r)),
                ("g", FigValue::Float(g)),
                ("b", FigValue::Float(b)),
                ("a", FigValue::Float(1.0)),
            ]),
        ),
        ("visible", FigValue::Bool(true)),
    ])
}

// tesla.fig blob 1480, Vector 711 fillGeometry (82 bytes).
#[test]
fn smoothed_rounded_rectangle_imports_as_bounded_cubic_path() {
    let tree = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("ROUNDED_RECTANGLE".into())),
            ("name", FigValue::Str("smooth card".into())),
            (
                "size",
                obj(vec![
                    ("x", FigValue::Float(100.0)),
                    ("y", FigValue::Float(60.0)),
                ]),
            ),
            ("cornerRadius", FigValue::Float(20.0)),
            ("cornerSmoothing", FigValue::Float(1.0)),
        ]),
        children: vec![],
    };
    let mut ctx = fresh_ctx();

    let node = convert_rectangle(&tree, None, &mut ctx);
    let PenNode::Path(path) = node else {
        panic!("expected smoothed rectangle to bake into a Path");
    };
    let d = path.d.expect("smoothed path data");
    assert!(d.contains('C'), "squircle corners must be cubic: {d}");
    assert!(
        !d.contains('A'),
        "squircle must not retain arc commands: {d}"
    );
    let bounds = compute_svg_path_bounds(&d).expect("smoothed path bounds");
    assert_eq!(
        (bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y),
        (0.0, 0.0, 100.0, 60.0)
    );
}

#[test]
fn convert_vector_uses_icon_lookup_when_set() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|name| {
        if name == "menu" {
            Some(IconLookupResult {
                d: "M3 12h18".into(),
                icon_id: Some("menu".into()),
                style: Some(IconStyle::Stroke),
            })
        } else {
            None
        }
    });

    let tree = vector_node("menu");
    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);

    match node {
        PenNode::Path(p) => {
            assert_eq!(p.icon_id.as_deref(), Some("menu"));
            assert_eq!(p.d.as_deref(), Some("M3 12h18"));
            assert!(p.stroke.is_some());
            assert!(p.fill.is_none());
        }
        other => panic!("expected Path, got {other:?}"),
    }

    clear_icon_lookup();
}

#[test]
fn convert_vector_falls_back_when_no_match() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let tree = vector_node("unmatched-name");
    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);

    // No icon match — the synthetic fixture has no geometry, so it
    // remains present in the layer tree as an invisible path.
    assert_invisible_degenerate_path(&node, "unmatched-name");
    assert!(ctx.warnings.iter().any(|w| w.contains("empty geometry")));

    clear_icon_lookup();
}

#[test]
fn convert_vector_preserves_numeric_sibling_mask_marker() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("mask shape");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    pairs.push(("mask".into(), FigValue::Uint(1)));
    pairs.push((
        "fillPaints".into(),
        FigValue::Array(vec![solid_paint(1.0, 1.0, 1.0)]),
    ));

    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);
    let PenNode::Path(path) = node else {
        panic!("expected mask Path");
    };
    assert_eq!(path.mask, Some(true));
    assert_eq!(path.base.mask_type, Some(MaskType::Alpha));

    clear_icon_lookup();
}

#[test]
fn convert_vector_does_not_claim_luminance_mask_support() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("luminance mask");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    pairs.push(("mask".into(), FigValue::Uint(1)));
    pairs.push(("maskType".into(), FigValue::Str("LUMINANCE".into())));
    pairs.push((
        "fillPaints".into(),
        FigValue::Array(vec![solid_paint(1.0, 1.0, 1.0)]),
    ));

    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);
    let PenNode::Path(path) = node else {
        panic!("expected Path");
    };
    assert_eq!(path.mask, None);
    assert_eq!(path.base.mask_type, Some(MaskType::Luminance));

    clear_icon_lookup();
}

#[test]
fn convert_vector_does_not_claim_translucent_alpha_mask_support() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("alpha mask");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    pairs.push(("mask".into(), FigValue::Uint(1)));
    pairs.push(("opacity".into(), FigValue::Float(0.5)));
    pairs.push((
        "fillPaints".into(),
        FigValue::Array(vec![solid_paint(1.0, 1.0, 1.0)]),
    ));

    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);
    let PenNode::Path(path) = node else {
        panic!("expected Path");
    };
    assert_eq!(path.mask, None);
    assert_eq!(path.base.mask_type, Some(MaskType::Alpha));

    clear_icon_lookup();
}

fn assert_invisible_degenerate_path(node: &PenNode, expected_name: &str) {
    let PenNode::Path(path) = node else {
        panic!("expected Path, got {node:?}");
    };
    assert_eq!(path.base.name.as_deref(), Some(expected_name));
    assert!(path.d.as_deref().unwrap_or_default().is_empty());
    assert!(path.anchors.as_deref().unwrap_or_default().is_empty());
    assert!(path.fill.is_none());
    assert!(path.stroke.is_none());
    assert_eq!(path.width, Some(SizingBehavior::Number(24.0)));
    assert_eq!(path.height, Some(SizingBehavior::Number(24.0)));
}

#[test]
fn degenerate_commands_blob_imports_as_invisible_path() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("four-byte-vector");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    pairs.push((
        "fillGeometry".into(),
        FigValue::Array(vec![obj(vec![("commandsBlob", FigValue::Uint(0))])]),
    ));
    let mut ctx = fresh_ctx();
    ctx.blobs = vec![BlobOrString::Bytes(vec![0; 4])];

    let node = convert_vector(&tree, None, &mut ctx);

    assert_invisible_degenerate_path(&node, "four-byte-vector");
    assert!(ctx.warnings.iter().any(|w| w.contains("empty geometry")));
    clear_icon_lookup();
}

#[test]
fn degenerate_boolean_without_geometry_imports_as_invisible_path() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("empty-boolean");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    let node_type = pairs
        .iter_mut()
        .find(|(key, _)| key.as_ref() == "type")
        .expect("type field");
    node_type.1 = FigValue::Str("BOOLEAN_OPERATION".into());
    let mut ctx = fresh_ctx();

    let node = convert_vector(&tree, None, &mut ctx);

    assert_invisible_degenerate_path(&node, "empty-boolean");
    assert!(ctx.warnings.iter().any(|w| w.contains("empty geometry")));
    clear_icon_lookup();
}

#[test]
fn odd_fill_geometry_sets_evenodd_path_fill_rule() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| None);

    let mut tree = vector_node("ring");
    let FigValue::Object(pairs) = &mut tree.figma else {
        unreachable!();
    };
    pairs.push((
        "fillGeometry".into(),
        FigValue::Array(vec![obj(vec![
            ("commandsBlob", FigValue::Uint(0)),
            ("windingRule", FigValue::Str("ODD".into())),
        ])]),
    ));
    let mut blob = vec![0x01];
    blob.extend_from_slice(&0.0f32.to_le_bytes());
    blob.extend_from_slice(&0.0f32.to_le_bytes());
    blob.push(0x02);
    blob.extend_from_slice(&10.0f32.to_le_bytes());
    blob.extend_from_slice(&0.0f32.to_le_bytes());
    let mut ctx = fresh_ctx();
    ctx.blobs = vec![BlobOrString::Bytes(blob)];

    let node = convert_vector(&tree, None, &mut ctx);
    let PenNode::Path(path) = node else {
        panic!("expected Path");
    };
    assert_eq!(
        path.fill_rule,
        Some(jian_ops_schema::node::PathFillRule::Evenodd)
    );

    clear_icon_lookup();
}

#[test]
fn icon_stroke_thickness_scales_for_small_icons() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| {
        Some(IconLookupResult {
            d: "M3 12h18".into(),
            icon_id: Some("menu".into()),
            style: Some(IconStyle::Stroke),
        })
    });

    // 12×12 vector → icon_scale = 12/24 = 0.5 → thickness 1.5 × 0.5 = 0.75.
    let mut tree = vector_node("menu");
    if let FigValue::Object(pairs) = &mut tree.figma {
        for (k, v) in pairs.iter_mut() {
            if k.as_ref() == "size" {
                *v = obj(vec![
                    ("x", FigValue::Float(12.0)),
                    ("y", FigValue::Float(12.0)),
                ]);
            }
        }
    }

    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);

    match node {
        PenNode::Path(p) => {
            let stroke = p.stroke.expect("stroke present");
            match stroke.thickness {
                jian_ops_schema::style::StrokeThickness::Uniform(t) => {
                    assert!(
                        (t - 0.75).abs() < 0.001,
                        "expected scaled thickness 0.75 got {t}"
                    );
                }
                other => panic!("expected Uniform thickness, got {other:?}"),
            }
        }
        other => panic!("expected Path, got {other:?}"),
    }

    clear_icon_lookup();
}

#[test]
fn icon_fill_style_emits_fill_no_stroke() {
    let _g = LOOKUP_GUARD.lock().unwrap();
    set_icon_lookup(|_| {
        Some(IconLookupResult {
            d: "M3 3h18v18H3z".into(),
            icon_id: Some("solid-square".into()),
            style: Some(IconStyle::Fill),
        })
    });
    let tree = {
        let mut t = vector_node("solid-square");
        if let FigValue::Object(pairs) = &mut t.figma {
            pairs.push((
                "fillPaints".into(),
                FigValue::Array(vec![obj(vec![
                    ("type", FigValue::Str("SOLID".into())),
                    (
                        "color",
                        obj(vec![
                            ("r", FigValue::Float(0.0)),
                            ("g", FigValue::Float(0.0)),
                            ("b", FigValue::Float(0.0)),
                        ]),
                    ),
                ])]),
            ));
        }
        t
    };
    let mut ctx = fresh_ctx();
    let node = convert_vector(&tree, None, &mut ctx);
    match node {
        PenNode::Path(p) => {
            assert!(p.fill.is_some(), "fill style must populate fill");
            assert!(p.stroke.is_none());
        }
        other => panic!("expected Path, got {other:?}"),
    }
    clear_icon_lookup();
}

// ── order_children: auto-layout flow order ────────────────────────

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
