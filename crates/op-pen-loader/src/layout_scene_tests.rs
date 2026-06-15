//! Unit tests for [`editor_state_to_layout_scene`].

use super::*;
use op_editor_core::EditorState;

/// Build an `EditorState` from a `.op` JSON source — mirrors how the
/// `adapter_tests` fixtures parse a `PenDocument`.
fn state_from(src: &str) -> EditorState {
    let parsed = jian_ops_schema::load_str(src).expect("parse .op fixture");
    EditorState::from_document(parsed.value)
}

#[test]
fn flex_layout_resolves_child_bounds_not_authored_coords() {
    // A vertical flex frame: a `fill_container`-width child must come
    // out stretched to the root's 375 px width — NOT the authored
    // `0` width the schema collapses flex tokens to. This proves the
    // scene carries layout-RESOLVED geometry.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{
        "id":"p1","name":"Page 1",
        "children":[{
          "type":"frame","id":"root","width":375,"height":812,
          "layout":"vertical","gap":16,
          "children":[
            {"type":"rectangle","id":"r1","width":"fill_container","height":40,
             "fill":[{"type":"solid","color":"#000000"}]}
          ]
        }]
      }],
      "children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    assert_eq!(scene.pages.len(), 1);
    let root = &scene.pages[0].children[0];
    assert_eq!(root.id, "root");
    let child = &root.children[0];
    assert_eq!(child.id, "r1");
    // Flex stretched the child to the root width.
    assert_eq!(
        child.bounds.size.x, 375.0,
        "fill_container stretched via taffy"
    );
    assert_eq!(child.bounds.size.y, 40.0);
}

#[test]
fn layout_scene_precomputes_unbounded_group_bounds() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{
        "id":"p1","name":"Page 1",
        "children":[{
          "type":"group","id":"g",
          "children":[
            {"type":"rectangle","id":"a","x":10,"y":20,"width":30,"height":40},
            {"type":"rectangle","id":"b","x":80,"y":5,"width":20,"height":25}
          ]
        }]
      }],
      "children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let group = &scene.pages[0].children[0];

    assert_eq!(group.bounds, op_editor_ui::Rect::ZERO);
    assert_eq!(
        group.aggregate_bounds_cache,
        op_editor_ui::Rect::xywh(10.0, 5.0, 90.0, 55.0),
        "loader should cache unbounded aggregate bounds once during scene build"
    );
}

#[test]
fn multi_root_designs_keep_authored_canvas_offset() {
    // Two side-by-side designs at distinct canvas coords — each
    // root's resolved bounds must reflect its authored `(x, y)`,
    // not collapse to origin.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"a","x":100,"y":50,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#FF0000"}]},
        {"type":"frame","id":"b","x":-500,"y":2000,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#00FF00"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let kids = &scene.pages[0].children;
    assert_eq!(
        (kids[0].bounds.origin.x, kids[0].bounds.origin.y),
        (100.0, 50.0)
    );
    assert_eq!(
        (kids[1].bounds.origin.x, kids[1].bounds.origin.y),
        (-500.0, 2000.0)
    );
}

#[test]
fn multi_page_document_produces_expected_page_structure() {
    let src = r##"{
      "version":"1.0.0","pages":[
        {"id":"home","name":"Home","children":[
          {"type":"rectangle","id":"h1","width":80,"height":40}
        ]},
        {"id":"about","name":"About","children":[
          {"type":"rectangle","id":"a1","width":60,"height":30},
          {"type":"rectangle","id":"a2","width":60,"height":30}
        ]}
      ],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    assert_eq!(scene.pages.len(), 2);
    assert_eq!(scene.pages[0].id, "home");
    assert_eq!(scene.pages[0].name, "Home");
    assert_eq!(scene.pages[0].children.len(), 1);
    assert_eq!(scene.pages[1].id, "about");
    assert_eq!(scene.pages[1].name, "About");
    assert_eq!(scene.pages[1].children.len(), 2);
    // The builder always opens on page 0 (the loader resets the
    // active page index).
    assert_eq!(scene.active_page_index, 0);
    assert_eq!(scene.active_page().map(|p| p.id.as_str()), Some("home"));
}

#[test]
fn variable_ref_fill_resolves_to_concrete_color() {
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::NodeId;

    // A rectangle whose fill should follow a `$ref` Color variable.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r1","width":100,"height":50,
         "fill":[{"type":"solid","color":"#000000"}]}
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
    assert_eq!(n.text_align, op_editor_ui::SceneTextAlign::Center);
    assert_eq!(
        n.text_vertical_align,
        op_editor_ui::SceneTextVerticalAlign::Middle
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

#[test]
fn no_variable_ref_keeps_authored_fill() {
    // Without a registered `$ref`, the node keeps its authored fill.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r1","width":100,"height":50,
         "fill":[{"type":"solid","color":"#112233"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let fill = scene.pages[0].children[0].fill.expect("authored fill");
    assert!((fill.r - 0.0667).abs() < 0.02);
    assert!((fill.g - 0.1333).abs() < 0.02);
    assert!((fill.b - 0.2).abs() < 0.02);
}

#[test]
fn text_node_carries_content_and_text_style() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"hd","content":"Welcome Back",
         "fontSize":28,"fontWeight":700,
         "fill":[{"type":"solid","color":"#0F172A"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(n.text.as_deref(), Some("Welcome Back"));
    assert_eq!(n.font_size, 28.0);
    assert_eq!(n.font_weight, 700);
}

#[test]
fn property_panel_fill_type_switch_produces_scene_gradient() {
    // Reproduce what the property-panel does on
    // "Solid → LinearGradient": mutate `EditorState.doc` via the
    // same code path the host calls, then re-build the scene and
    // assert that `SceneNode.gradient` is populated. Catches the
    // case where the live mutation path bypasses the scene's
    // gradient field — e.g. if the adapter only reads .op files
    // and not in-memory `PenDocument` mutations.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"solid","color":"#FF0000"}]
      }]}],"children":[]
    }"##;
    let mut state = state_from(src);
    let node_id = op_editor_core::NodeId::new("r");
    state.selection.set = vec![node_id.clone()];
    state.selection.anchor = node_id;
    assert!(state.set_selected_fill_type(op_editor_core::FillType::LinearGradient));
    let scene = editor_state_to_layout_scene(&state);
    let n = &scene.pages[0].children[0];
    assert!(
        n.gradient.is_some(),
        "scene gradient must populate after Solid → LinearGradient toggle; \
         live-mutated PenDocument is being silently dropped"
    );
}

#[test]
fn linear_gradient_payload_threads_into_scene_node() {
    // A LinearGradient first-fill must come out of the scene builder
    // as `SceneNode.gradient = Some(Linear { ... })` — otherwise the
    // canvas painter falls back to the first-stop solid and the
    // user's "switch fill to LinearGradient" intent doesn't render.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"linear_gradient","angle":90,
                 "stops":[{"color":"#FF0000","offset":0},
                          {"color":"#0000FF","offset":1}]}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    let g = n.gradient.as_ref().expect("scene gradient must populate");
    match g {
        op_editor_ui::layout_scene::SceneGradient::Linear { stops, .. } => {
            assert_eq!(stops.len(), 2);
            assert!(stops[0].color.r > 0.99 && stops[0].color.b < 0.01);
        }
        other => panic!("expected linear, got {other:?}"),
    }
}

#[test]
fn image_fill_mode_threads_into_scene_node() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":360,"height":240,
        "fill":[{"type":"image","url":"data:image/png;base64,AA==","mode":"fit"}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(n.image_src.as_deref(), Some("data:image/png;base64,AA=="));
    assert_eq!(n.image_fit, op_editor_ui::layout_scene::SceneImageFit::Fit);
}

#[test]
fn image_fill_adjustments_thread_into_scene_node() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":360,"height":240,
        "fill":[{"type":"image","url":"data:image/png;base64,AA==",
          "mode":"fit","exposure":100,"contrast":-100,"saturation":50,
          "temperature":25,"tint":-25,"highlights":75,"shadows":-75}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let a = scene.pages[0].children[0].image_adjustments;
    assert_eq!(a.exposure, 100.0);
    assert_eq!(a.contrast, -100.0);
    assert_eq!(a.saturation, 50.0);
    assert_eq!(a.temperature, 25.0);
    assert_eq!(a.tint, -25.0);
    assert_eq!(a.highlights, 75.0);
    assert_eq!(a.shadows, -75.0);
}

#[test]
fn empty_editor_state_yields_empty_single_page_scene() {
    let scene = editor_state_to_layout_scene(&EditorState::new());
    // The loader's single-page fallback yields one empty page.
    assert_eq!(scene.pages.len(), 1);
    assert!(scene.pages[0].children.is_empty());
}

#[test]
fn node_opacity_bakes_into_resolved_fill_alpha() {
    // Node-level `opacity` must be folded into the resolved fill
    // alpha at scene-build time (TS parity: a 50%-opacity shape with
    // an opaque fill paints at 50% alpha). Mirrors the risk-monitor
    // pedestal, whose translucent stacked layers depend on it.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
         "opacity":0.5,
         "fill":[{"type":"solid","color":"#ffffff"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let fill = scene.pages[0].children[0].fill.expect("rect has a fill");
    assert!(
        (fill.a - 0.5).abs() < 1e-3,
        "node opacity 0.5 should bake into fill alpha, got {}",
        fill.a
    );
}

#[test]
fn node_opacity_is_cumulative_through_containers() {
    // A 0.5-opacity frame containing a 0.5-opacity child folds to
    // 0.25 on the child's resolved fill (group opacity composites
    // multiplicatively down the tree).
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"f","x":0,"y":0,"width":100,"height":100,
         "opacity":0.5,
         "children":[
           {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
            "opacity":0.5,
            "fill":[{"type":"solid","color":"#ffffff"}]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let frame = &scene.pages[0].children[0];
    let child = frame
        .children
        .iter()
        .find(|n| n.id == "r")
        .expect("child r");
    let fill = child.fill.expect("rect fill");
    assert!(
        (fill.a - 0.25).abs() < 1e-3,
        "0.5 * 0.5 cumulative opacity should bake to 0.25, got {}",
        fill.a
    );
}

#[test]
fn node_opacity_scales_gradient_multiplier_not_stops() {
    // A 0.5-opacity node with a linear-gradient fill: node opacity
    // folds into the gradient's `opacity` multiplier only. The
    // backend already applies `opacity` to every stop when it builds
    // the shader, so scaling stop alpha here too would dim twice.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
         "opacity":0.5,
         "fill":[{"type":"linear_gradient","angle":0,"opacity":1,
                  "stops":[{"offset":0,"color":"#ffffff"},{"offset":1,"color":"#000000"}]}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    match scene.pages[0].children[0]
        .gradient
        .as_ref()
        .expect("gradient")
    {
        SceneGradient::Linear { opacity, stops, .. } => {
            assert!(
                (opacity - 0.5).abs() < 1e-3,
                "gradient opacity should scale to 0.5, got {opacity}"
            );
            assert!(
                (stops[0].color.a - 1.0).abs() < 1e-3,
                "stop alpha must stay authored (1.0), got {}",
                stops[0].color.a
            );
        }
        _ => panic!("expected linear gradient"),
    }
}

#[test]
fn node_opacity_bakes_into_stroke_alpha() {
    // Node opacity dims the stroke alongside the fill.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
         "opacity":0.5,
         "stroke":{"thickness":2,"fill":[{"type":"solid","color":"#ff0000"}]}}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let stroke = scene.pages[0].children[0].stroke.expect("stroke");
    assert!(
        (stroke.color.a - 0.5).abs() < 1e-3,
        "node opacity should bake into stroke alpha, got {}",
        stroke.color.a
    );
}

#[test]
fn node_opacity_dims_drop_shadow_color() {
    // A drop shadow is part of the node's paint, so node opacity dims
    // it too (opaque-black shadow on a 0.5 node → 0.5 alpha).
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
         "opacity":0.5,
         "fill":[{"type":"solid","color":"#ffffff"}],
         "effects":[{"type":"shadow","offsetX":2,"offsetY":4,"blur":8,"spread":0,"color":"#000000"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let effects = &scene.pages[0].children[0].effects;
    assert_eq!(effects.len(), 1, "expected one drop shadow");
    let Effect::DropShadow(s) = &effects[0];
    assert!(
        (s.color.a - 0.5).abs() < 1e-3,
        "node opacity should dim shadow alpha, got {}",
        s.color.a
    );
}

#[test]
fn inner_shadow_flag_survives_into_scene() {
    // An `inner: true` shadow in the .op must reach the scene as an
    // inset shadow (the risk-monitor pedestal's top hexagon uses one
    // for its recessed glow).
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10,
         "fill":[{"type":"solid","color":"#ffffff"}],
         "effects":[{"type":"shadow","inner":true,"offsetX":0,"offsetY":0,"blur":4,"spread":0,"color":"#000000"}]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let effects = &scene.pages[0].children[0].effects;
    assert_eq!(effects.len(), 1, "expected one shadow");
    let Effect::DropShadow(s) = &effects[0];
    assert!(s.inner, "inner:true must survive into the scene effect");
}

#[test]
fn loaded_document_dollar_ref_fill_resolves_without_any_cache() {
    // The P0 document-compat repro: a TS-authored / reloaded .op whose
    // fill IS the `$ref` string. No transient `fill_refs` cache exists
    // after a load — the render-time resolution pass alone must land
    // the concrete colour (and the palette fallback must cover tokens
    // the document never seeded).
    let src = r##"{
      "version":"1.0.0",
      "variables":{"brand":{"type":"color","value":"#ff8800"}},
      "pages":[{"id":"p","name":"P","children":[
        {"type":"rectangle","id":"r1","width":100,"height":50,
         "fill":[{"type":"solid","color":"$brand"}]},
        {"type":"rectangle","id":"r2","x":0,"y":60,"width":100,"height":50,
         "fill":[{"type":"solid","color":"$color-surface"}]}
      ]}],"children":[]
    }"##;
    let state = state_from(src);
    let scene = editor_state_to_layout_scene(&state);

    let r1 = &scene.pages[0].children[0];
    let fill = r1.fill.expect("$brand fill must resolve from the document");
    assert!((fill.r - 1.0).abs() < 0.01, "red channel: {}", fill.r);
    assert!((fill.g - 0.533).abs() < 0.02, "green channel: {}", fill.g);

    // Un-seeded semantic token → built-in palette (#FFFFFF in Light).
    let r2 = &scene.pages[0].children[1];
    let fill2 = r2
        .fill
        .expect("$color-surface must resolve via the palette fallback");
    assert!((fill2.r - 1.0).abs() < 0.01);
    assert!((fill2.g - 1.0).abs() < 0.01);
    assert!((fill2.b - 1.0).abs() < 0.01);
}

#[test]
fn loaded_document_spacing_token_gap_reaches_flex_solver() {
    // `$spacing-2` (palette fallback = 8) must reach jian's flex pass
    // as a real number: two 50px-tall children in a vertical frame
    // land at y=0 and y=58.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"f","width":200,"height":200,"layout":"vertical","gap":"$spacing-2",
         "children":[
           {"type":"rectangle","id":"a","width":50,"height":50},
           {"type":"rectangle","id":"b","width":50,"height":50}
         ]}
      ]}],"children":[]
    }"##;
    let state = state_from(src);
    let scene = editor_state_to_layout_scene(&state);
    let frame = &scene.pages[0].children[0];
    assert_eq!(frame.children.len(), 2);
    let a = &frame.children[0];
    let b = &frame.children[1];
    assert!(
        (b.bounds.origin.y - a.bounds.origin.y - 58.0).abs() < 0.5,
        "vertical gap must be 8 (got delta {})",
        b.bounds.origin.y - a.bounds.origin.y
    );
}

#[test]
fn ref_instance_renders_component_subtree_at_instance_position() {
    // The P0 document-compat repro: a TS-authored .op with a reusable
    // component + a ref instance must render the instance's full
    // subtree (previously `PenNode::Ref` collapsed to an empty group).
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#222222"}],
         "children":[
           {"type":"rectangle","id":"badge","x":10,"y":10,"width":20,"height":20,
            "fill":[{"type":"solid","color":"#ff0000"}]}
         ]},
        {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
      ]}],"children":[]
    }"##;
    let state = state_from(src);
    let scene = editor_state_to_layout_scene(&state);
    let page = &scene.pages[0];
    assert_eq!(page.children.len(), 2, "component + expanded instance");
    let inst = &page.children[1];
    assert_eq!(inst.id, "inst1");
    assert!(
        (inst.bounds.origin.x - 300.0).abs() < 0.5,
        "instance renders at its own position (x = {})",
        inst.bounds.origin.x
    );
    assert!(
        (inst.bounds.size.x - 200.0).abs() < 0.5,
        "instance inherits the component's width (w = {})",
        inst.bounds.size.x
    );
    assert_eq!(inst.children.len(), 1, "subtree expands");
    assert_eq!(inst.children[0].id, "inst1__badge", "virtual child id");
    let badge_fill = inst.children[0].fill.expect("badge fill survives");
    assert!((badge_fill.r - 1.0).abs() < 0.01, "badge stays red");
}

#[test]
fn scene_threads_clip_content_flag() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","width":300,"height":200,
         "children":[
           {"type":"frame","id":"clipped","width":100,"height":100,"clipContent":true},
           {"type":"frame","id":"open","width":100,"height":100}
         ]}
      ]}],
      "children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let root = &scene.pages[0].children[0];
    assert!(root.clip_content, "root frame clips implicitly");
    assert!(root
        .children
        .iter()
        .any(|c| c.id == "clipped" && c.clip_content));
    assert!(root
        .children
        .iter()
        .any(|c| c.id == "open" && !c.clip_content));
}

#[test]
fn scene_text_runs_map_segments_onto_byte_ranges() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"t","opacity":0.5,
         "content":[
           {"text":"汉字","fontWeight":700},
           {"text":"ab","fill":"#00ff00","fontStyle":"italic"}
         ]}
      ]}],
      "children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let t = scene.pages[0].find("t").expect("text node");
    assert_eq!(t.text.as_deref(), Some("汉字ab"));
    assert_eq!(t.text_runs.len(), 2);
    // "汉字" = 6 bytes; ranges are cumulative byte offsets.
    assert_eq!((t.text_runs[0].start, t.text_runs[0].end), (0, 6));
    assert_eq!((t.text_runs[1].start, t.text_runs[1].end), (6, 8));
    assert_eq!(t.text_runs[0].font_weight, 700);
    assert!(t.text_runs[1].italic);
    // Node opacity 0.5 folds into the run fill's alpha like the
    // node-level fill.
    let fill = t.text_runs[1].fill.expect("per-run fill");
    assert!((fill.g - 1.0).abs() < 1e-6);
    assert!((fill.a - 0.5).abs() < 1e-6, "alpha carries cum_opacity");
}

#[test]
fn text_input_icons_reach_scene_widget() {
    let state = state_from(
        r##"{"version":"1.1","formatVersion":"1.1","id":"x",
        "app":{"name":"x","version":"1","id":"x"},
        "children":[{"type":"frame","id":"s","width":300,"height":300,"children":[
            {"type":"text_input","id":"f","width":280,"height":48,
             "leadingIcon":"mail","trailingIcon":"eye","placeholder":"you@example.com"}]}]}"##,
    );
    let scene = editor_state_to_layout_scene(&state);
    let field = scene
        .active_page()
        .unwrap()
        .find("f")
        .expect("text_input node in scene");
    let w = field
        .widget
        .as_ref()
        .expect("text_input carries a SceneWidget");
    assert_eq!(w.kind, "text_input");
    assert_eq!(w.leading_icon.as_deref(), Some("mail"));
    assert_eq!(w.trailing_icon.as_deref(), Some("eye"));
    assert_eq!(w.placeholder.as_deref(), Some("you@example.com"));
}

#[test]
fn pen_document_to_layout_scene_matches_editor_state_path() {
    let src = r##"{"version":"1.1","formatVersion":"1.1","id":"x",
        "app":{"name":"x","version":"1","id":"x"},
        "children":[{"type":"frame","id":"s","width":200,"height":200,
          "fill":[{"type":"solid","color":"#ffffff"}]}]}"##;
    let state = state_from(src);
    let via_state = editor_state_to_layout_scene(&state);
    let via_doc =
        crate::pen_document_to_layout_scene(&state.doc, &std::collections::BTreeMap::new(), 0);
    assert_eq!(
        via_state, via_doc,
        "bare-doc scene must equal the editor-state scene for a cache-free doc"
    );
}
