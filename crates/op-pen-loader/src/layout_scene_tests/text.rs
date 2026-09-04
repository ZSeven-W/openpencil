//! Text scene coverage: typography fields, `fit_content` measurement,
//! omitted / explicit heights and stack reservation.

use super::*;

#[test]
fn variable_ref_replaces_only_primary_fill_layer() {
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::NodeId;

    // A rectangle with three authored paints. The `$ref` colour replaces only
    // the front-most one; its opacity/blend and both layers behind it survive.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r1","width":100,"height":50,"opacity":0.5,
         "fill":[
           {"type":"solid","color":"#000000","opacity":0.4,"blendMode":"multiply"},
           {"type":"linear_gradient","angle":90,"opacity":0.6,"blendMode":"screen",
            "stops":[{"offset":0,"color":"#001122"},{"offset":1,"color":"#334455"}]},
           {"type":"image","url":"data:image/png;base64,AA==","mode":"fit",
            "opacity":0.8,"blendMode":"overlay"}
         ]}
      ]}],"children":[]
    }"##;
    let mut state = state_from(src);
    // Persisted Color variable.
    state.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    );
    // Transient fill-ref cache: node `r1`'s fill follows `brand`.
    state
        .ui
        .variables
        .fill_refs
        .insert(NodeId::new("r1"), "brand".into());

    let scene = editor_state_to_layout_scene(&state);
    let r1 = &scene.pages[0].children[0];
    let fill = r1.fill.expect("r1 must carry a resolved fill");
    // `$ref` won over the authored black — resolved to #ff8800.
    assert!((fill.r - 1.0).abs() < 0.01, "red channel: {}", fill.r);
    assert!((fill.g - 0.533).abs() < 0.02, "green channel: {}", fill.g);
    assert!(fill.b.abs() < 0.01, "blue channel: {}", fill.b);
    assert!(
        (fill.a - 0.5).abs() < 0.001,
        "legacy fill alpha: {}",
        fill.a
    );
    assert_eq!(r1.fill_layers.len(), 3);
    assert!(matches!(
        &r1.fill_layers[0],
        SceneFillLayer::Solid {
            color,
            blend_mode: ImageBlendMode::Multiply,
        } if (color.r - 1.0).abs() < 0.01
          && (color.g - 0.533).abs() < 0.02
          && color.b.abs() < 0.01
          && (color.a - 0.4).abs() < 0.001
    ));
    assert!(matches!(
        &r1.fill_layers[1],
        SceneFillLayer::Gradient {
            gradient: SceneGradient::Linear { opacity, stops, .. },
            blend_mode: ImageBlendMode::Screen,
        } if (opacity - 0.6).abs() < 0.001
          && (stops[0].color.g - 0x11 as f32 / 255.0).abs() < 0.001
    ));
    assert!(matches!(
        &r1.fill_layers[2],
        SceneFillLayer::Image {
            opacity,
            blend_mode: ImageBlendMode::Overlay,
            fit: SceneImageFit::Fit,
            ..
        } if (opacity - 0.8).abs() < 0.001
    ));
    assert!((r1.opacity - 0.5).abs() < 0.001);

    let fallback = fill_layers_to_scene(&[], Some(Color::rgba_u8(1, 2, 3, 0.75)), [0.0, 0.0]);
    assert!(matches!(
        fallback.as_slice(),
        [SceneFillLayer::Solid {
            color,
            blend_mode: ImageBlendMode::Normal,
        }] if (color.a - 0.75).abs() < 0.001
    ));
}

#[test]
fn text_typography_fields_flow_through_to_scene() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"t","x":10,"y":20,"width":200,"height":80,
         "content":"hello",
         "fontFamily":"Georgia","fontSize":28,"fontWeight":700,
         "lineHeight":1.6,"letterSpacing":3,
         "textAlign":"center","textAlignVertical":"middle",
         "textGrowth":"fixed-width-height"}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = scene.pages[0].find("t").expect("text node in scene");
    assert_eq!(n.font_family, "Georgia");
    assert_eq!(n.font_size, 28.0);
    assert_eq!(n.font_weight, 700);
    assert_eq!(n.line_height, 1.6);
    assert_eq!(n.letter_spacing, 3.0);
    assert_eq!(
        n.text_align,
        jian_scene::layout_scene::SceneTextAlign::Center
    );
    assert_eq!(
        n.text_vertical_align,
        jian_scene::layout_scene::SceneTextVerticalAlign::Middle
    );
    assert!(n.text_wrap);
}

#[test]
fn fit_content_text_resolves_scene_bounds_to_measured_text() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"t","x":10,"y":20,
         "width":"fit_content","height":"fit_content",
         "content":"N490-测试-9","fontSize":14,
         "fill":[{"type":"solid","color":"#0F172A"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = scene.pages[0].find("t").expect("text node in scene");

    assert_eq!(n.bounds.origin.x, 10.0);
    assert_eq!(n.bounds.origin.y, 20.0);
    assert!(
        n.bounds.size.x > op_editor_core::DEFAULT_TEXT_NODE_WIDTH as f32,
        "fit_content text should resolve to measured content width, got {}",
        n.bounds.size.x
    );
    assert!(
        n.bounds.size.y >= 14.0,
        "fit_content text should resolve to measured content height, got {}",
        n.bounds.size.y
    );
}

#[test]
fn omitted_height_text_uses_content_height_for_fixed_and_fill_width() {
    // Live M3 regression: the model emitted pixel-like lineHeight values while
    // omitting text height. They are outside the canonical multiplier range;
    // auto-height text must fall back to content measurement instead of
    // becoming fontSize * lineHeight hundreds of pixels tall.
    let src = r##"{
      "version":"1.0.0","children":[{
        "type":"frame","id":"root","width":320,"height":300,
        "layout":"vertical","gap":4,"children":[
          {"type":"text","id":"fixed","width":144,
           "content":"Neon Lights Live","fontSize":13,"lineHeight":17},
          {"type":"text","id":"fill","width":"fill_container",
           "content":"Sunset Jazz on Pier 17","fontSize":14,"lineHeight":18},
          {"type":"rectangle","id":"after","width":"fill_container","height":20}
        ]
      }]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let page = scene.active_page().expect("active page");
    let fixed = page.find("fixed").expect("fixed-width text");
    let fill = page.find("fill").expect("fill-width text");
    let after = page.find("after").expect("following sibling");

    assert_eq!(
        fixed.line_height, 0.0,
        "paint should use default line height"
    );
    assert_eq!(
        fill.line_height, 0.0,
        "paint should use default line height"
    );
    assert!(
        (10.0..40.0).contains(&fixed.bounds.size.y),
        "fixed-width omitted-height text should use content height, got {:?}",
        fixed.bounds
    );
    assert!(
        (10.0..40.0).contains(&fill.bounds.size.y),
        "fill-width omitted-height text should use content height, got {:?}",
        fill.bounds
    );
    assert!(
        after.bounds.origin.y >= fill.bounds.origin.y + fill.bounds.size.y,
        "following sibling must come after auto-height text: fixed={:?} fill={:?} after={:?}",
        fixed.bounds,
        fill.bounds,
        after.bounds
    );
    assert!(
        after.bounds.origin.y < 100.0,
        "auto-height text must not consume the clipped parent: {:?}",
        after.bounds
    );
}

#[test]
fn explicit_height_multiline_text_rejects_pixel_like_line_height_for_paint() {
    // An explicit box height is a geometry contract, not permission to change
    // lineHeight from a multiplier into pixels. Layout and paint must therefore
    // use the same fallback semantics for multi-line fixed-height text.
    let src = r##"{
      "version":"1.0.0","children":[{
        "type":"text","id":"explicit","width":180,"height":52,
        "textGrowth":"fixed-width-height",
        "content":"First line\nSecond line","fontSize":14,"lineHeight":17
      }]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let node = scene
        .active_page()
        .expect("active page")
        .find("explicit")
        .expect("explicit-height text");

    assert_eq!(node.bounds.size.y, 52.0, "authored box height is preserved");
    assert_eq!(
        node.line_height, 0.0,
        "paint must use the default multiplier instead of treating 17 as 17x"
    );
}

#[test]
fn fit_content_stack_reserves_missing_height_text_before_next_section() {
    let src = r##"{
      "version":"1.0.0",
      "children":[{
        "type":"frame","id":"root","width":390,"height":844,
        "layout":"vertical","gap":0,
        "children":[
          {"type":"frame","id":"status","width":"fill_container","height":62},
          {"type":"frame","id":"header","width":"fill_container","height":"fit_content",
           "layout":"vertical","padding":[20,24],"children":[{
            "type":"frame","id":"header-row","width":"fill_container","height":"fit_content",
            "layout":"horizontal","gap":12,"children":[
              {"type":"frame","id":"greeting-block","width":"fit_content","height":"fit_content",
               "layout":"vertical","gap":4,"children":[
                {"type":"text","id":"greeting","width":"fit_content",
                 "content":"Good morning, Alex","fontSize":24},
                {"type":"text","id":"date","width":"fit_content",
                 "content":"Tuesday, June 7","fontSize":14}
              ]},
              {"type":"frame","id":"avatar","width":44,"height":44}
            ]}]},
          {"type":"frame","id":"weekly","width":"fill_container","height":"fit_content",
           "layout":"vertical","padding":[0,24],"children":[{
            "type":"frame","id":"card","width":"fill_container","height":"fit_content",
            "layout":"vertical","padding":20,"children":[
              {"type":"text","id":"title","content":"Weekly Streak","fontSize":16}
            ]}]}
        ]
      }]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let page = scene.active_page().expect("active page");
    let header = page.find("header").expect("header");
    let header_row = page.find("header-row").expect("header row");
    let greeting_block = page.find("greeting-block").expect("greeting block");
    let greeting = page.find("greeting").expect("greeting text");
    let date = page.find("date").expect("date text");
    let weekly = page.find("weekly").expect("weekly section");

    assert!(
        date.bounds.size.y >= 14.0,
        "missing-height text should still measure; got {}",
        date.bounds.size.y
    );
    assert!(
        weekly.bounds.origin.y >= date.bounds.origin.y + date.bounds.size.y,
        "next vertical sibling must not overlap measured text: header={:?} row={:?} greeting_block={:?} greeting={:?} date={:?} weekly={:?}",
        header.bounds,
        header_row.bounds,
        greeting_block.bounds,
        greeting.bounds,
        date.bounds,
        weekly.bounds
    );
}
