//! `convert_vector` icon-lookup tests — exercise the host-resolver
//! branch (`set_icon_lookup` → name match → emit `Path` with `icon_id`).
//! Tests serialise on `LOOKUP_GUARD` because the resolver is
//! process-global state.

use super::*;
use crate::common::{clear_icon_lookup, set_icon_lookup, IconLookupResult, IconStyle};
use jian_ops_schema::node::PenNode;
use std::collections::HashMap;
use std::sync::Mutex;

/// Serialise tests that touch the process-global icon-lookup so they
/// don't race with each other under cargo's parallel runner.
static LOOKUP_GUARD: Mutex<()> = Mutex::new(());

fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn vector_node(name: &str) -> TreeNode {
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

fn fresh_ctx() -> ConversionContext {
    ConversionContext {
        component_map: HashMap::new(),
        symbol_tree: HashMap::new(),
        warnings: Vec::new(),
        id_counter: 1,
        blobs: Vec::new(),
        layout_mode: FigLayoutMode::OpenPencil,
    }
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

    // No icon match — falls through to rectangle fallback (vector has
    // no decoded path data in this synthetic fixture).
    assert!(matches!(node, PenNode::Rectangle(_)));

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
            if k == "size" {
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
