use super::*;
use crate::css::cascade::StyleRule;
use jian_ops_schema::style::ImageFillMode;
use std::collections::BTreeMap;

fn computed(properties: &[(&str, &str)]) -> ComputedStyle {
    ComputedStyle {
        props: properties
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into()))
            .collect::<BTreeMap<_, _>>(),
        font_size: 16.0,
    }
}

fn run<T>(action: impl FnOnce(&mut MapCtx<'_>) -> T) -> (T, Vec<String>) {
    let options = crate::HtmlImportOptions::default();
    let rules = Vec::<StyleRule>::new();
    let mut context = MapCtx {
        opts: &options,
        rules: &rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: options.viewport_width,
        containing_height: options.viewport_height(),
        containing_width_is_definite: true,
        positioned_width: options.viewport_width,
        positioned_height: options.viewport_height(),
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    let output = action(&mut context);
    (output, crate::render_warnings(&context.warnings))
}

#[test]
fn css_extended_blend_modes_map_to_native_compositors() {
    for (css, expected) in [
        ("soft-light", BlendMode::SoftLight),
        ("color-dodge", BlendMode::ColorDodge),
        ("color-burn", BlendMode::ColorBurn),
        ("hard-light", BlendMode::HardLight),
        ("exclusion", BlendMode::Exclusion),
    ] {
        assert_eq!(map_blend_mode(css), Some(expected), "{css}");
    }
}

#[test]
fn layered_background_keeps_css_order_color_and_variable_stops() {
    let style = computed(&[
        ("--tone", "oklch(62% 0.2 25)"),
        (
            "background-image",
            "linear-gradient(90deg,var(--tone) 10% 20%,rgb(0 0 255 / 50%) 80%),url(\"a,b.png\")",
        ),
        ("background-repeat", "no-repeat,no-repeat"),
        ("background-size", "cover,contain"),
        ("background-color", "#123456"),
    ]);
    let (fills, warnings) =
        run(|context| map_fill(&style, context, (400.0, 300.0), (true, true), false).unwrap());
    let PenFill::LinearGradient(gradient) = &fills[0] else {
        panic!("top CSS layer must remain the first fill")
    };
    assert_eq!(gradient.angle, Some(90.0));
    assert_eq!(gradient.stops.len(), 3);
    assert_eq!(
        (gradient.stops[0].offset, gradient.stops[1].offset),
        (0.1, 0.2)
    );
    assert!(matches!(&fills[1], PenFill::Image(image)
        if image.mode == Some(ImageFillMode::Fit)));
    assert!(matches!(&fills[2], PenFill::Solid(fill) if fill.color == "#123456"));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("background-size on gradients"));
}

#[test]
fn radial_gradient_maps_position_radius_and_implicit_stop() {
    let style = computed(&[]);
    let (fill, warnings) = run(|context| {
        map_gradient(
            "radial-gradient(circle 40% at top right,red 0%,blue,green 100%)",
            &style,
            context,
        )
        .unwrap()
    });
    let PenFill::RadialGradient(gradient) = fill else {
        panic!()
    };
    assert_eq!(
        (gradient.cx, gradient.cy, gradient.radius),
        (Some(1.0), Some(0.0), Some(0.4))
    );
    assert_eq!(gradient.stops[1].offset, 0.5);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn multiple_outer_and_inset_shadows_are_preserved() {
    let style = computed(&[
        ("color", "#334455"),
        (
            "box-shadow",
            "inset 0 1px 2px 3px rgb(0 0 0 / 25%),4px 5px #ff0000",
        ),
    ]);
    let (effects, warnings) = run(|context| map_effects(&style, context).unwrap());
    assert_eq!(effects.len(), 2);
    assert!(matches!(&effects[0], PenEffect::Shadow(shadow)
        if shadow.inner == Some(true) && shadow.spread == 3.0 && shadow.color == "#00000040"));
    assert!(matches!(&effects[1], PenEffect::Shadow(shadow)
        if shadow.inner.is_none() && shadow.offset_x == 4.0 && shadow.color == "#ff0000"));
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn per_side_border_widths_survive_and_unrepresentable_differences_warn() {
    let style = computed(&[
        ("border-top-style", "dashed"),
        ("border-top-width", "2px"),
        ("border-top-color", "red"),
        ("border-right-style", "none"),
        ("border-right-width", "7px"),
        ("border-bottom-style", "dotted"),
        ("border-bottom-width", "4px"),
        ("border-bottom-color", "blue"),
        ("border-left-style", "solid"),
        ("border-left-width", "6px"),
        ("border-left-color", "blue"),
    ]);
    let (stroke, warnings) = run(|context| map_stroke(&style, context).unwrap());
    assert_eq!(
        stroke.thickness,
        StrokeThickness::PerSide([2.0, 0.0, 4.0, 6.0])
    );
    assert_eq!(stroke.align, Some(StrokeAlign::Inside));
    assert!(stroke.dash_pattern.is_some());
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("border colors")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("border styles")));
}

#[test]
fn border_style_uses_medium_width_and_initial_current_color() {
    let style = computed(&[("border-style", "solid")]);
    let (stroke, warnings) = run(|context| map_stroke(&style, context).unwrap());
    assert_eq!(stroke.thickness, StrokeThickness::Uniform(3.0));
    assert!(matches!(stroke.fill.as_ref().unwrap().first(),
        Some(PenFill::Solid(fill)) if fill.color == "#000000"));
    assert!(warnings.is_empty());
}

#[test]
fn transparent_fill_is_skipped_and_unsupported_visuals_warn_once() {
    let transparent = computed(&[("background-color", "rgb(1 2 3 / 0)")]);
    let (fill, _) =
        run(|context| map_fill(&transparent, context, (400.0, 300.0), (true, true), false));
    assert!(fill.is_none());

    let unsupported = computed(&[
        ("background-image", "conic-gradient(red,blue)"),
        ("background-position", "25% bottom"),
        ("filter", "brightness(2) brightness(3)"),
    ]);
    let (_, warnings) = run(|context| {
        map_fill(&unsupported, context, (400.0, 300.0), (true, true), false);
        map_effects(&unsupported, context);
    });
    assert_eq!(
        warnings
            .iter()
            .filter(|warning| warning.contains("filter function"))
            .count(),
        1
    );
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("conic gradients")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("background-position")));
}

#[test]
fn omitted_gradient_stops_are_interpolated() {
    let mut stops = vec![
        ParsedStop {
            color: "a".into(),
            offset: 0.0,
            explicit: false,
        },
        ParsedStop {
            color: "b".into(),
            offset: 0.0,
            explicit: false,
        },
        ParsedStop {
            color: "c".into(),
            offset: 0.0,
            explicit: false,
        },
    ];
    distribute_stops(&mut stops);
    assert_eq!(
        (stops[0].offset, stops[1].offset, stops[2].offset),
        (0.0, 0.5, 1.0)
    );
}

#[test]
fn text_shadow_keeps_the_first_layer_and_reports_the_rest() {
    let style = computed(&[("text-shadow", "1px 2px 3px #ff0000, 0 0 9px blue")]);
    let (effects, warnings) = run(|context| map_text_shadow(&style, context).unwrap());
    let [PenEffect::Shadow(shadow)] = effects.as_slice() else {
        panic!("expected exactly one shadow, found {effects:?}")
    };
    assert_eq!(
        (shadow.offset_x, shadow.offset_y, shadow.blur),
        (1.0, 2.0, 3.0)
    );
    assert_eq!(shadow.color, "#ff0000");
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("after the first")),
        "{warnings:?}"
    );

    let none = computed(&[("text-shadow", "none")]);
    let (effects, _) = run(|context| map_text_shadow(&none, context));
    assert!(effects.is_none());

    let broken = computed(&[("text-shadow", "totally-not-a-shadow")]);
    let (effects, warnings) = run(|context| map_text_shadow(&broken, context));
    assert!(effects.is_none());
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("unsupported CSS text-shadow")),
        "{warnings:?}"
    );
}

/// `text-shadow` has no `spread` and no `inset`; a layer that uses either is
/// invalid CSS, which a browser drops rather than paints.
#[test]
fn text_shadow_rejects_the_box_shadow_only_grammar() {
    for invalid in ["1px 2px 3px 4px #ff0000", "inset 1px 2px 3px #ff0000"] {
        let style = computed(&[("text-shadow", invalid)]);
        let (effects, warnings) = run(|context| map_text_shadow(&style, context));
        assert!(effects.is_none(), "{invalid}: {effects:?}");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("unsupported CSS text-shadow")),
            "{invalid}: {warnings:?}"
        );
    }

    // The same shapes stay legal on `box-shadow`.
    let boxed = computed(&[("box-shadow", "inset 1px 2px 3px 4px #ff0000")]);
    let (effects, _) = run(|context| map_effects(&boxed, context).unwrap());
    let [PenEffect::Shadow(shadow)] = effects.as_slice() else {
        panic!("expected a box shadow, found {effects:?}")
    };
    assert_eq!((shadow.spread, shadow.inner), (4.0, Some(true)));
}

/// The schema keeps effects per text NODE, so a shadow authored on an inline
/// element inside a paragraph cannot survive and says so.
#[test]
fn a_segment_level_text_shadow_is_reported() {
    let result = crate::import_html(
        "<p>plain <span style='text-shadow:0 1px 2px #000000'>shadowed</span></p>",
        &crate::HtmlImportOptions::default(),
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("text-shadow on an inline element")),
        "{:?}",
        result.warnings
    );

    // A segment that merely inherits the block's shadow is not a drop.
    let inherited = crate::import_html(
        "<p style='text-shadow:0 1px 2px #000000'>plain <span>same</span></p>",
        &crate::HtmlImportOptions::default(),
    );
    assert!(
        !inherited
            .warnings
            .iter()
            .any(|warning| warning.contains("text-shadow on an inline element")),
        "{:?}",
        inherited.warnings
    );
}

#[test]
fn text_shadow_inherits_onto_descendant_text_nodes() {
    use jian_ops_schema::node::PenNode;
    let result = crate::import_html(
        "<div style='text-shadow:0 1px 2px #000000'><p>copy</p></div>",
        &crate::HtmlImportOptions::default(),
    );
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!()
    };
    let PenNode::Frame(division) =
        crate::mapper::unwrap_margin_node(&root.children.as_ref().unwrap()[0])
    else {
        panic!()
    };
    let PenNode::Frame(paragraph) =
        crate::mapper::unwrap_margin_node(&division.children.as_ref().unwrap()[0])
    else {
        panic!()
    };
    let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert!(matches!(
        text.effects.as_deref(),
        Some([PenEffect::Shadow(_)])
    ));
}

#[test]
fn every_non_visible_overflow_value_clips() {
    use jian_ops_schema::node::PenNode;
    for (declaration, expected) in [
        ("overflow:visible", None),
        ("overflow:hidden", Some(true)),
        ("overflow:clip", Some(true)),
        ("overflow:auto", Some(true)),
        ("overflow:scroll", Some(true)),
        ("overflow-y:auto", Some(true)),
        ("overflow-x:hidden;overflow-y:visible", Some(true)),
        ("overflow:visible auto", Some(true)),
    ] {
        let html = format!("<div style='{declaration}'>x</div>");
        let result = crate::import_html(&html, &crate::HtmlImportOptions::default());
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(division) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(
            division.container.clip_content, expected,
            "declaration: {declaration}"
        );
    }
}
