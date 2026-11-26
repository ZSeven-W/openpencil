use crate::fills::{node_fills, node_stroke_fills};
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::state_with;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenFill, SolidFillBody};

fn import_leaves(svg: &str) -> Vec<PenNode> {
    fn collect(node: &PenNode, leaves: &mut Vec<PenNode>) {
        if let Some(children) = node.children() {
            for child in children {
                collect(child, leaves);
            }
        } else {
            leaves.push(node.clone());
        }
    }
    let mut state = state_with(vec![]);
    assert!(state.import_svg(&mut 1, svg, (0.0, 0.0)) > 0);
    let mut leaves = Vec::new();
    for node in state.active_children() {
        collect(node, &mut leaves);
    }
    leaves
}

fn solid(fills: &[PenFill]) -> &SolidFillBody {
    match &fills[0] {
        PenFill::Solid(body) => body,
        other => panic!("expected solid fill, got {other:?}"),
    }
}

fn fill(node: &PenNode) -> &SolidFillBody {
    solid(node_fills(node).expect("fill list"))
}

fn stroke_fill(node: &PenNode) -> &SolidFillBody {
    solid(node_stroke_fills(node).expect("stroke fill"))
}

fn assert_paint(paint: &SolidFillBody, color: &str, opacity: f32) {
    assert_eq!(paint.color, color);
    assert!((paint.opacity.unwrap_or(1.0) - opacity).abs() < 0.00001);
    if opacity == 1.0 {
        assert_eq!(paint.opacity, None, "full-strength paint omits opacity");
    }
}

#[test]
fn imports_css_color_forms_for_fill_and_stroke() {
    for (css, hex, alpha) in [
        ("rgb(255,0,0)", "#ff0000", 1.0),
        ("rgba(0,0,255,.25)", "#0000ff", 64.0 / 255.0),
        ("hsl(120 100% 50%)", "#00ff00", 1.0),
        ("hsla(120,100%,50%,.5)", "#00ff00", 128.0 / 255.0),
        ("rgb(100% 0% 0% / 50%)", "#ff0000", 128.0 / 255.0),
        ("hwb(0 0% 0%)", "#ff0000", 1.0),
        ("lab(100% 0 0)", "#ffffff", 1.0),
        ("lch(0% 0 20deg)", "#000000", 1.0),
        ("oklab(100% 0 0)", "#ffffff", 1.0),
        ("oklch(0% 0 1turn)", "#000000", 1.0),
        ("color(srgb 1 0 0 / .5)", "#ff0000", 128.0 / 255.0),
        ("color(display-p3 1 0 0)", "#ff0000", 1.0),
        ("color-mix(in srgb, red, blue)", "#800080", 1.0),
        ("rebeccapurple", "#663399", 1.0),
        ("DarkSlateGrey", "#2f4f4f", 1.0),
        ("#12345680", "#123456", 128.0 / 255.0),
        ("#0f08", "#00ff00", 136.0 / 255.0),
    ] {
        let nodes = import_leaves(&format!(
            "<svg><rect width='10' height='10' fill='{css}' stroke='{css}'/></svg>"
        ));
        assert_paint(fill(&nodes[0]), hex, alpha);
        assert_paint(stroke_fill(&nodes[0]), hex, alpha);
    }
}

#[test]
fn current_color_inherits_from_group_and_defaults_to_black() {
    let nodes = import_leaves(
        "<svg><g color='rebeccapurple' fill='currentColor' stroke='currentColor'>\
         <rect width='10' height='10'/></g>\
         <rect width='10' height='10' fill='CURRENTCOLOR' stroke='currentColor'/></svg>",
    );
    assert_paint(fill(&nodes[0]), "#663399", 1.0);
    assert_paint(stroke_fill(&nodes[0]), "#663399", 1.0);
    assert_paint(fill(&nodes[1]), "#000000", 1.0);
    assert_paint(stroke_fill(&nodes[1]), "#000000", 1.0);
}

#[test]
fn group_current_color_inherits_parent_color() {
    let nodes = import_leaves(
        "<svg color='rebeccapurple' fill='currentColor' stroke='currentColor'>\
         <g color='currentColor'><rect width='10' height='10'/></g></svg>",
    );
    assert_paint(fill(&nodes[0]), "#663399", 1.0);
    assert_paint(stroke_fill(&nodes[0]), "#663399", 1.0);
}

#[test]
fn inline_color_overrides_attributes_and_inherited_current_color() {
    let nodes = import_leaves(
        "<svg color='red' style='color:blue' fill='currentColor'>\
         <g color='yellow' style='color:rebeccapurple'>\
         <rect width='10' height='10' color='green' style='color:currentColor'/>\
         <rect width='10' height='10' style='color:rgb(0,255,0)'/></g>\
         <rect width='10' height='10'/></svg>",
    );
    assert_paint(fill(&nodes[0]), "#663399", 1.0);
    assert_paint(fill(&nodes[1]), "#00ff00", 1.0);
    assert_paint(fill(&nodes[2]), "#0000ff", 1.0);
}

#[test]
fn inherited_paint_opacity_multiplies_alpha_and_child_values_replace_it() {
    let nodes = import_leaves(
        "<svg><g fill='#12345680' fill-opacity='.4' stroke='rgba(0,0,255,.25)' \
         stroke-opacity='.6'><rect width='10' height='10'/>\
         <rect width='10' height='10' fill-opacity='.8' stroke-opacity='.2'\
         style='fill-opacity:50%;stroke-opacity:25%'/></g>\
         <rect width='10' height='10' stroke='red'/></svg>",
    );
    assert_paint(fill(&nodes[0]), "#123456", 0.4 * 128.0 / 255.0);
    assert_paint(stroke_fill(&nodes[0]), "#0000ff", 0.6 * 64.0 / 255.0);
    assert_paint(fill(&nodes[1]), "#123456", 0.5 * 128.0 / 255.0);
    assert_paint(stroke_fill(&nodes[1]), "#0000ff", 0.25 * 64.0 / 255.0);
    assert_paint(fill(&nodes[2]), "#000000", 1.0);
    assert_paint(stroke_fill(&nodes[2]), "#ff0000", 1.0);
}

#[test]
fn invalid_fill_colors_remain_unset_without_panicking() {
    for value in ["#zzzzzz", "#éééé", "rgb(bad)", "unknown"] {
        let nodes = import_leaves(&format!(
            "<svg><rect width='10' height='10' fill='{value}'/></svg>"
        ));
        assert!(node_fills(&nodes[0]).is_none());
    }
}

#[test]
fn root_inline_opacity_reaches_paths_and_default_fills() {
    let nodes = import_leaves(
        "<svg fill-opacity='.9' stroke='currentColor' color='#ff000080' \
         style='fill-opacity:.3;stroke-opacity:.5'>\
         <path d='M0 0 L10 0 L10 10 Z' fill='currentColor'/>\
         <circle r='5'/></svg>",
    );
    assert_paint(fill(&nodes[0]), "#ff0000", 0.3 * 128.0 / 255.0);
    assert_paint(stroke_fill(&nodes[0]), "#ff0000", 0.5 * 128.0 / 255.0);
    assert_paint(fill(&nodes[1]), "#000000", 0.3);
}

#[test]
fn opacity_is_clamped_and_invalid_values_keep_inherited_value() {
    for (value, expected) in [("-1", 0.0), ("2", 1.0), ("NaN", 0.4), ("bad", 0.4)] {
        let nodes = import_leaves(&format!(
            "<svg fill-opacity='.4' stroke-opacity='.4'><rect width='10' height='10' \
             stroke='red' fill-opacity='{value}' stroke-opacity='{value}'/></svg>"
        ));
        assert_paint(fill(&nodes[0]), "#000000", expected);
        assert_paint(stroke_fill(&nodes[0]), "#ff0000", expected);
    }
}

#[test]
fn preserves_none_transparent_and_url_paint_behavior() {
    for value in ["none", "transparent", "url(#paint)"] {
        let nodes = import_leaves(&format!(
            "<svg><rect width='10' height='10' fill='{value}' stroke='{value}'/></svg>"
        ));
        if value.starts_with("url(") {
            assert_paint(fill(&nodes[0]), "#000000", 1.0);
        } else {
            assert!(node_fills(&nodes[0]).is_none());
        }
        assert!(node_stroke_fills(&nodes[0]).is_none());
    }
}
