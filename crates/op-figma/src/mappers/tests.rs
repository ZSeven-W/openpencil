//! Style-mapper tests — exercise each mapper on hand-built FigValue
//! node objects.

use super::*;

fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn color_obj(r: f64, g: f64, b: f64) -> FigValue {
    obj(vec![
        ("r", FigValue::Float(r as f32)),
        ("g", FigValue::Float(g as f32)),
        ("b", FigValue::Float(b as f32)),
    ])
}

#[test]
fn solid_fill_maps_to_hex() {
    let paints = [obj(vec![
        ("type", FigValue::Str("SOLID".into())),
        ("color", color_obj(1.0, 0.0, 0.0)),
    ])];
    let fills = map_figma_fills(Some(&paints)).expect("one fill");
    match &fills[0] {
        PenFill::Solid(b) => assert_eq!(b.color, "#ff0000"),
        _ => panic!("expected solid"),
    }
}

#[test]
fn invisible_paints_are_dropped() {
    let paints = [obj(vec![
        ("type", FigValue::Str("SOLID".into())),
        ("color", color_obj(0.0, 0.0, 0.0)),
        ("visible", FigValue::Bool(false)),
    ])];
    assert!(map_figma_fills(Some(&paints)).is_none());
}

#[test]
fn linear_gradient_angle_from_transform() {
    // Direction column0 = (m00, m10) = (0, 1) → math atan2(1,0)=90°,
    // CSS angle = 90 - 90 = 0.
    let transform = obj(vec![
        ("m00", FigValue::Float(0.0)),
        ("m10", FigValue::Float(1.0)),
    ]);
    let paints = [obj(vec![
        ("type", FigValue::Str("GRADIENT_LINEAR".into())),
        (
            "stops",
            FigValue::Array(vec![obj(vec![
                ("position", FigValue::Float(0.0)),
                ("color", color_obj(0.0, 0.0, 0.0)),
            ])]),
        ),
        ("transform", transform),
    ])];
    match &map_figma_fills(Some(&paints)).unwrap()[0] {
        PenFill::LinearGradient(g) => assert_eq!(g.angle, Some(0.0)),
        _ => panic!("expected linear gradient"),
    }
}

#[test]
fn image_fill_hash_url() {
    let paints = [obj(vec![
        ("type", FigValue::Str("IMAGE".into())),
        (
            "image",
            obj(vec![("hash", FigValue::Bytes(vec![0xab, 0xcd]))]),
        ),
        ("imageScaleMode", FigValue::Str("FIT".into())),
    ])];
    match &map_figma_fills(Some(&paints)).unwrap()[0] {
        PenFill::Image(img) => {
            assert_eq!(img.url, "__hash:abcd");
            assert_eq!(img.mode, Some(ImageFillMode::Fit));
        }
        _ => panic!("expected image fill"),
    }
}

#[test]
fn stroke_uniform_thickness() {
    let node = obj(vec![
        (
            "strokePaints",
            FigValue::Array(vec![obj(vec![
                ("type", FigValue::Str("SOLID".into())),
                ("color", color_obj(0.0, 0.0, 0.0)),
            ])]),
        ),
        ("strokeWeight", FigValue::Float(2.5)),
        ("strokeAlign", FigValue::Str("INSIDE".into())),
    ]);
    let stroke = map_figma_stroke(&node).expect("stroke present");
    assert!(matches!(stroke.thickness, StrokeThickness::Uniform(2.5)));
    assert_eq!(stroke.align, Some(StrokeAlign::Inside));
    assert!(stroke.fill.is_some());
}

#[test]
fn stroke_per_side_thickness() {
    let node = obj(vec![
        (
            "strokePaints",
            FigValue::Array(vec![obj(vec![
                ("type", FigValue::Str("SOLID".into())),
                ("color", color_obj(0.0, 0.0, 0.0)),
            ])]),
        ),
        ("borderStrokeWeightsIndependent", FigValue::Bool(true)),
        ("borderTopWeight", FigValue::Float(1.0)),
        ("borderRightWeight", FigValue::Float(2.0)),
        ("borderBottomWeight", FigValue::Float(3.0)),
        ("borderLeftWeight", FigValue::Float(4.0)),
    ]);
    let stroke = map_figma_stroke(&node).unwrap();
    assert!(matches!(
        stroke.thickness,
        StrokeThickness::PerSide([1.0, 2.0, 3.0, 4.0])
    ));
}

#[test]
fn drop_shadow_effect() {
    let effects = [obj(vec![
        ("type", FigValue::Str("DROP_SHADOW".into())),
        (
            "offset",
            obj(vec![
                ("x", FigValue::Float(2.0)),
                ("y", FigValue::Float(4.0)),
            ]),
        ),
        ("radius", FigValue::Float(8.0)),
    ])];
    match &map_figma_effects(Some(&effects)).unwrap()[0] {
        PenEffect::Shadow(s) => {
            assert_eq!(s.inner, Some(false));
            assert_eq!(s.offset_x, 2.0);
            assert_eq!(s.blur, 8.0);
            assert_eq!(s.color, "#00000040");
        }
        _ => panic!("expected shadow"),
    }
}

#[test]
fn background_blur_effect() {
    let effects = [obj(vec![
        ("type", FigValue::Str("BACKGROUND_BLUR".into())),
        ("radius", FigValue::Float(12.0)),
    ])];
    match &map_figma_effects(Some(&effects)).unwrap()[0] {
        PenEffect::BackgroundBlur(b) => assert_eq!(b.radius, 12.0),
        _ => panic!("expected background blur"),
    }
}

#[test]
fn layout_horizontal_with_gap_and_padding() {
    let node = obj(vec![
        ("stackMode", FigValue::Str("HORIZONTAL".into())),
        ("stackSpacing", FigValue::Float(12.0)),
        ("stackPadding", FigValue::Float(8.0)),
        ("stackPrimaryAlignItems", FigValue::Str("CENTER".into())),
        ("stackCounterAlignItems", FigValue::Str("MIN".into())),
    ]);
    let l = map_figma_layout(&node);
    assert_eq!(l.layout, Some(LayoutMode::Horizontal));
    assert_eq!(l.gap, Some(12.0));
    assert_eq!(l.padding, Some(Padding::Uniform(8.0)));
    assert_eq!(l.justify_content, Some(JustifyContent::Center));
    assert_eq!(l.align_items, Some(AlignItems::Start));
    assert_eq!(l.clip_content, Some(true));
}

#[test]
fn layout_space_between_skips_gap() {
    let node = obj(vec![
        ("stackMode", FigValue::Str("VERTICAL".into())),
        ("stackSpacing", FigValue::Float(10.0)),
        (
            "stackPrimaryAlignItems",
            FigValue::Str("SPACE_EVENLY".into()),
        ),
    ]);
    let l = map_figma_layout(&node);
    assert_eq!(l.justify_content, Some(JustifyContent::SpaceBetween));
    assert_eq!(l.gap, None);
}

#[test]
fn padding_per_side_quad() {
    let node = obj(vec![
        ("stackVerticalPadding", FigValue::Float(4.0)),
        ("stackHorizontalPadding", FigValue::Float(8.0)),
        ("stackPaddingBottom", FigValue::Float(16.0)),
    ]);
    // top=4, right=8, bottom=16, left=8 → not all-equal, not v==v/h==h.
    assert_eq!(
        map_padding(&node),
        Some(Padding::LtrB([4.0, 8.0, 16.0, 8.0]))
    );
}

#[test]
fn width_sizing_fill_container_in_horizontal_parent() {
    let node = obj(vec![("stackChildPrimaryGrow", FigValue::Int(1))]);
    assert!(matches!(
        map_width_sizing(&node, Some("HORIZONTAL")),
        SizingBehavior::Keyword(SizingKeyword::FillContainer)
    ));
}

#[test]
fn width_sizing_falls_back_to_size_x() {
    let node = obj(vec![(
        "size",
        obj(vec![
            ("x", FigValue::Float(240.0)),
            ("y", FigValue::Float(80.0)),
        ]),
    )]);
    assert!(matches!(
        map_width_sizing(&node, None),
        SizingBehavior::Number(n) if n == 240.0
    ));
}

#[test]
fn height_sizing_fit_content_in_vertical_stack() {
    let node = obj(vec![
        ("stackMode", FigValue::Str("VERTICAL".into())),
        ("stackPrimarySizing", FigValue::Str("RESIZE_TO_FIT".into())),
    ]);
    assert!(matches!(
        map_height_sizing(&node, None),
        SizingBehavior::Keyword(SizingKeyword::FitContent)
    ));
}
