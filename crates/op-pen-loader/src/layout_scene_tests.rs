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
fn empty_editor_state_yields_empty_single_page_scene() {
    let scene = editor_state_to_layout_scene(&EditorState::new());
    // The loader's single-page fallback yields one empty page.
    assert_eq!(scene.pages.len(), 1);
    assert!(scene.pages[0].children.is_empty());
}
