//! `convert_vector` icon-lookup tests — exercise the host-resolver
//! branch (`set_icon_lookup` → name match → emit `Path` with `icon_id`).
//! Tests serialise on `LOOKUP_GUARD` because the resolver is
//! process-global state.

use super::*;
use crate::common::{clear_icon_lookup, set_icon_lookup, IconLookupResult, IconStyle};
use crate::figma_types::BlobOrString;
use crate::vector_decoder::compute_svg_path_bounds;
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
