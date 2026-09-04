//! Design-variable tokens, `ref` component instances, clip flags,
//! text runs / widget icons and the `PenDocument` entry-point parity
//! check.

use super::*;

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
         "fill":[{"type":"solid","color":"$--card"}]}
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
        .expect("$--card must resolve via the palette fallback");
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
fn progress_semantics_reach_scene_widget() {
    let state = state_from(
        r##"{"version":"1.1","formatVersion":"1.1","id":"x",
        "app":{"name":"x","version":"1","id":"x"},
        "children":[{"type":"progress","id":"p","width":200,"height":8,
          "value":75,"max":100,"indeterminate":true,"cornerRadius":0}]}"##,
    );
    let scene = editor_state_to_layout_scene(&state);
    let progress = scene.active_page().unwrap().find("p").unwrap();
    let widget = progress.widget.as_ref().expect("progress SceneWidget");

    assert!(widget.indeterminate);
    assert!(widget.corner_radius_authored);
    assert_eq!(progress.corner_radius, 0.0);
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

#[test]
fn mesh_gradient_fill_threads_into_scene_node() {
    // The REAL editor path: a `.op` mesh_gradient first-fill must come
    // out of EditorState → scene as `SceneGradient::Mesh` so the canvas
    // dispatch reaches the backend's Gouraud painter — otherwise the
    // node silently flattens to the first vertex colour.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"mesh_gradient","rows":2,"cols":2,
                 "stops":[{"row":0,"col":0,"color":"#FF0000"},
                          {"row":0,"col":1,"color":"#00FF00"},
                          {"row":1,"col":0,"color":"#0000FF"},
                          {"row":1,"col":1,"color":"#FFFFFF"}]}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(
        n.fill_type,
        jian_scene::layout_scene::SceneFillType::MeshGradient
    );
    match n.gradient.as_ref().expect("mesh gradient must populate") {
        jian_scene::layout_scene::SceneGradient::Mesh {
            rows,
            cols,
            colors,
            opacity,
        } => {
            assert_eq!((*rows, *cols), (2, 2));
            assert_eq!(colors.len(), 4);
            assert!(colors[0].r > 0.99 && colors[0].g < 0.01, "vertex (0,0) red");
            assert!(colors[3].b > 0.99, "vertex (1,1) white");
            assert!((opacity - 1.0).abs() < 1e-6);
        }
        other => panic!("expected mesh, got {other:?}"),
    }
}

#[test]
fn shader_fill_threads_into_scene_node() {
    // Same reachability proof for SkSL shader fills: the scene must
    // carry the program + resolved uniforms + fallback so the native
    // painter can compile it and every other painter shows the
    // fallback solid.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"shader",
                 "sksl":"half4 main(float2 p){ return half4(1.0,0.0,0.0,1.0); }",
                 "uniforms":{"u_tint":"#336699","u_mix":0.25}}]
      }]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let n = &scene.pages[0].children[0];
    assert_eq!(n.fill_type, jian_scene::layout_scene::SceneFillType::Shader);
    let s = n.shader.as_ref().expect("scene shader must populate");
    assert!(s.sksl.contains("half4 main"));
    assert_eq!(s.uniforms.len(), 2, "both uniforms resolve");
    let tint = s
        .uniforms
        .iter()
        .find(|u| u.name == "u_tint")
        .expect("colour uniform");
    assert_eq!(tint.values.len(), 4, "colour expands to a vec4");
    // Fallback = the first colour uniform (#336699), not mid-gray.
    assert!((s.fallback.b - 0x99 as f32 / 255.0).abs() < 0.01);
    // The scene's flat `fill` matches the fallback so painters without
    // shader support (web) show the same colour.
    let flat = n.fill.expect("fallback fill");
    assert!((flat.b - 0x99 as f32 / 255.0).abs() < 0.01);
}
