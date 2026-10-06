use super::*;
use jian_ops_schema::node::PenNode;

fn nested_frame_doc(depth: usize) -> String {
    let mut src = String::from(r#"{"version":"1.0.0","children":["#);
    for i in 0..depth {
        src.push_str(&format!(
            r##"{{"type":"frame","id":"nest-{i:05}","name":"Nested Layer {i:05}","x":8,"y":6,"width":400,"height":220,"fill":[{{"type":"solid","color":"#ffffff20"}}],"stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#0088ff"}}]}},"children":["##
        ));
    }
    for _ in 0..depth {
        src.push_str("]}");
    }
    src.push_str("]}");
    src
}

fn nested_legacy_doc(depth: usize, version: &str) -> String {
    let mut src = format!(r##"{{"version":"{version}","futureTopLevel":true,"children":["##);
    for i in 0..depth {
        src.push_str(&format!(
            r##"{{"type":"frame","id":"legacy-{i}","fill":"#123456","children":["##
        ));
    }
    src.push_str(r#"{"type":"path","id":"legacy-path","geometry":"M0 0L9 9"}"#);
    for _ in 0..depth {
        src.push_str("]}");
    }
    src.push_str("]}");
    src
}

fn on_deep_test_stack(name: &str, test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name(name.to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn deep loader test")
        .join()
        .expect("deep loader test completed");
}

fn children(node: &PenNode) -> Option<&[PenNode]> {
    match node {
        PenNode::Frame(n) => n.children.as_deref(),
        PenNode::Group(n) => n.children.as_deref(),
        PenNode::Rectangle(n) => n.children.as_deref(),
        _ => None,
    }
}

fn max_depth(nodes: &[PenNode]) -> usize {
    let mut max = 0;
    let mut stack: Vec<(&PenNode, usize)> = nodes.iter().map(|node| (node, 1)).collect();
    while let Some((node, depth)) = stack.pop() {
        max = max.max(depth);
        if let Some(children) = children(node) {
            stack.extend(children.iter().map(|child| (child, depth + 1)));
        }
    }
    max
}

fn node_id(node: &PenNode) -> &str {
    match node {
        PenNode::Frame(node) => &node.base.id,
        PenNode::Group(node) => &node.base.id,
        PenNode::Rectangle(node) => &node.base.id,
        PenNode::Ellipse(node) => &node.base.id,
        PenNode::Line(node) => &node.base.id,
        PenNode::Polygon(node) => &node.base.id,
        PenNode::Path(node) => &node.base.id,
        PenNode::Text(node) => &node.base.id,
        PenNode::TextInput(node) => &node.base.id,
        PenNode::Image(node) => &node.base.id,
        PenNode::IconFont(node) => &node.base.id,
        PenNode::TextArea(node) => &node.base.id,
        PenNode::Select(node) => &node.base.id,
        PenNode::Switch(node) => &node.base.id,
        PenNode::Checkbox(node) => &node.base.id,
        PenNode::Slider(node) => &node.base.id,
        PenNode::RadioGroup(node) => &node.base.id,
        PenNode::NumberInput(node) => &node.base.id,
        PenNode::Progress(node) => &node.base.id,
        PenNode::Tabs(node) => &node.base.id,
        PenNode::Ref(node) => &node.base.id,
    }
}

fn ids(nodes: &[PenNode]) -> Vec<&str> {
    nodes.iter().map(node_id).collect()
}

#[test]
fn load_canonical_accepts_5000_deep_visible_frame_tree() {
    std::thread::Builder::new()
        .name("deep-loader-test".into())
        .stack_size(512 * 1024 * 1024)
        .spawn(|| {
            let loaded = load_canonical(&nested_frame_doc(5_000)).expect("deep document loads");
            assert_eq!(max_depth(&loaded.value.children), 5_000);
            // `PenDocument` is a recursive enum; drop is recursive too.
            // This test is about the loader accepting the document.
            std::mem::forget(loaded);
        })
        .expect("spawn deep-loader-test")
        .join()
        .expect("deep-loader-test completed");
}

#[test]
fn deep_loader_recognizes_editor_meta_as_a_known_extension() {
    std::thread::Builder::new()
        .name("deep-editor-meta-test".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let src = nested_frame_doc(121).replacen(
                r#""children""#,
                r#""editorMeta":{"activePageIndex":2},"children""#,
                1,
            );
            let loaded = load_canonical(&src).expect("deep document with editor metadata loads");
            assert!(
                !loaded.warnings.iter().any(|warning| matches!(
                    warning,
                    jian_ops_schema::LoadWarning::UnknownField { field, .. }
                        if field == "editorMeta"
                )),
                "the normal and deep compatibility paths must recognize the same extension"
            );
            // `PenDocument` is recursively dropped; this test only exercises
            // the compatibility header of the deep-loader path.
            std::mem::forget(loaded);
        })
        .expect("spawn deep-editor-meta-test")
        .join()
        .expect("deep-editor-meta-test completed");
}

#[test]
fn depth_probe_ignores_brackets_and_escapes_inside_strings() {
    let fake_nesting = format!(
        "prefix {} \"quoted\\\\tail\" {}",
        "[".repeat(121),
        "]".repeat(121)
    );
    let src = serde_json::json!({
        "version": "2.8",
        "name": fake_nesting,
        "children": [{
            "type": "frame",
            "id": "legacy",
            "fill": "#123456",
            "children": []
        }]
    })
    .to_string();

    assert!(!looks_deeply_nested(&src));
    let loaded = load_canonical(&src).expect("shallow legacy document loads");
    assert_eq!(loaded.value.version, "1.0");
    let PenNode::Frame(frame) = &loaded.value.children[0] else {
        panic!("legacy frame expected");
    };
    assert_eq!(frame.container.fill.as_ref().map(Vec::len), Some(1));
}

#[test]
fn deep_loader_normalizes_legacy_fields_without_image_tables() {
    on_deep_test_stack("deep-legacy-normalize-test", || {
        let loaded =
            load_canonical(&nested_legacy_doc(121, "1.0")).expect("deep legacy document loads");
        let mut node = &loaded.value.children[0];
        for depth in 0..121 {
            let PenNode::Frame(frame) = node else {
                panic!("frame expected at depth {depth}");
            };
            assert_eq!(frame.container.fill.as_ref().map(Vec::len), Some(1));
            node = &frame.children.as_deref().expect("nested children")[0];
        }
        let PenNode::Path(path) = node else {
            panic!("legacy path leaf expected");
        };
        assert_eq!(path.d.as_deref(), Some("M0 0L9 9"));
        std::mem::forget(loaded);
    });
}

#[test]
fn deep_loader_patches_unsupported_version_and_preserves_warnings() {
    on_deep_test_stack("deep-legacy-version-test", || {
        let loaded =
            load_canonical(&nested_legacy_doc(121, "2.8")).expect("deep best-effort load succeeds");
        assert_eq!(loaded.value.version, "1.0");
        assert!(loaded.warnings.iter().any(|warning| matches!(
            warning,
            jian_ops_schema::LoadWarning::UnknownField { field, .. }
                if field == "futureTopLevel"
        )));
        std::mem::forget(loaded);
    });
}

#[test]
fn intermediate_figma_preserve_format_repairs_nested_auto_layout_child_order() {
    let src = r#"{
      "version":"1.0.0",
      "pages":[{"id":"figma-page-0","name":"Imported","children":[
        {"type":"frame","id":"outer","layout":"vertical","children":[
          {"type":"frame","id":"plain","y":0,"children":[
            {"type":"rectangle","id":"plain-a"},
            {"type":"rectangle","id":"plain-b"}
          ]},
          {"type":"rectangle","id":"absolute-a","y":1000,"constraints":{"h":"left","v":"top"}},
          {"type":"rectangle","id":"absolute-b","y":-1000,"constraints":{"h":"left","v":"top"}},
          {"type":"frame","id":"nested","y":100,"layout":"horizontal","children":[
            {"type":"rectangle","id":"nested-a","x":0},
            {"type":"rectangle","id":"nested-b","x":100},
            {"type":"rectangle","id":"nested-c","x":200},
            {"type":"rectangle","id":"nested-d","x":300}
          ]}
        ]}
      ]}],
      "children":[],
      "editorMeta":{"activePageIndex":0}
    }"#;

    let loaded = load_canonical(src).expect("intermediate Figma document loads");
    let page = &loaded.value.pages.as_ref().expect("pages")[0];
    let PenNode::Frame(outer) = &page.children[0] else {
        panic!("outer frame expected");
    };
    let outer_children = outer.children.as_deref().expect("outer children");
    assert_eq!(
        ids(outer_children),
        ["nested", "absolute-b", "absolute-a", "plain"]
    );

    let PenNode::Frame(nested) = &outer_children[0] else {
        panic!("nested auto-layout frame expected");
    };
    assert_eq!(
        ids(nested.children.as_deref().expect("nested children")),
        ["nested-d", "nested-c", "nested-b", "nested-a"]
    );

    let PenNode::Frame(plain) = &outer_children[3] else {
        panic!("plain frame expected");
    };
    assert_eq!(
        ids(plain.children.as_deref().expect("plain children")),
        ["plain-a", "plain-b"],
        "non-auto-layout containers keep canonical paint order"
    );
}

#[test]
fn intermediate_format_already_in_paint_order_is_not_reversed_again() {
    let src = r#"{
      "version":"1.0.0",
      "pages":[{"id":"figma-page-0","name":"Imported","children":[
        {"type":"frame","id":"stack","layout":"vertical","children":[
          {"type":"rectangle","id":"paint-front","y":300},
          {"type":"rectangle","id":"paint-middle-2","y":200},
          {"type":"rectangle","id":"paint-middle-1","y":100},
          {"type":"rectangle","id":"paint-back","y":0}
        ]}
      ]}],
      "children":[],
      "editorMeta":{"activePageIndex":0}
    }"#;

    let loaded = load_canonical(src).expect("already-fixed document loads");
    let page = &loaded.value.pages.as_ref().expect("pages")[0];
    let PenNode::Frame(stack) = &page.children[0] else {
        panic!("stack frame expected");
    };
    assert_eq!(
        ids(stack.children.as_deref().expect("stack children")),
        [
            "paint-front",
            "paint-middle-2",
            "paint-middle-1",
            "paint-back"
        ]
    );
}

#[test]
fn intermediate_format_without_authored_axis_samples_is_not_reversed() {
    let src = r#"{
      "version":"1.0.0",
      "pages":[{"id":"figma-page-0","name":"Imported","children":[
        {"type":"frame","id":"stack","layout":"horizontal","children":[
          {"type":"rectangle","id":"first"},
          {"type":"rectangle","id":"second"},
          {"type":"rectangle","id":"third"},
          {"type":"rectangle","id":"fourth"}
        ]}
      ]}],
      "children":[],
      "editorMeta":{"activePageIndex":0}
    }"#;

    let loaded = load_canonical(src).expect("coordinate-free document loads");
    let page = &loaded.value.pages.as_ref().expect("pages")[0];
    let PenNode::Frame(stack) = &page.children[0] else {
        panic!("stack frame expected");
    };
    assert_eq!(
        ids(stack.children.as_deref().expect("stack children")),
        ["first", "second", "third", "fourth"]
    );
}

#[test]
fn intermediate_format_with_tied_order_evidence_is_not_reversed() {
    let src = r#"{
      "version":"1.0.0",
      "pages":[{"id":"figma-page-0","name":"Imported","children":[
        {"type":"frame","id":"vertical","layout":"vertical","children":[
          {"type":"rectangle","id":"v-a","y":0},
          {"type":"rectangle","id":"v-b","y":100},
          {"type":"rectangle","id":"v-c","y":0}
        ]},
        {"type":"frame","id":"horizontal","layout":"horizontal","children":[
          {"type":"rectangle","id":"h-a","x":0},
          {"type":"rectangle","id":"h-b","x":100},
          {"type":"rectangle","id":"h-c","x":0}
        ]}
      ]}],
      "children":[],
      "editorMeta":{"activePageIndex":0}
    }"#;

    let loaded = load_canonical(src).expect("ambiguous document loads");
    let page = &loaded.value.pages.as_ref().expect("pages")[0];
    let PenNode::Frame(vertical) = &page.children[0] else {
        panic!("vertical frame expected");
    };
    let PenNode::Frame(horizontal) = &page.children[1] else {
        panic!("horizontal frame expected");
    };
    assert_eq!(
        ids(vertical.children.as_deref().expect("vertical children")),
        ["v-a", "v-b", "v-c"]
    );
    assert_eq!(
        ids(horizontal.children.as_deref().expect("horizontal children")),
        ["h-a", "h-b", "h-c"]
    );
}

#[test]
fn explicit_preserve_field_and_absent_editor_meta_do_not_reorder_figma_page_children() {
    for editor_meta in [
        r#", "editorMeta":{"activePageIndex":0,"preserveAuthoredGeometry":true}"#,
        r#", "editorMeta":{"activePageIndex":0,"preserveAuthoredGeometry":false}"#,
        r#", "editorMeta":{"active_page_index":0,"preserve_authored_geometry":false}"#,
        "",
    ] {
        let src = format!(
            r#"{{"version":"1.0.0","pages":[{{"id":"figma-page-0","name":"Imported","children":[{{"type":"frame","id":"stack","layout":"vertical","children":[{{"type":"rectangle","id":"first","y":0}},{{"type":"rectangle","id":"second","y":100}},{{"type":"rectangle","id":"third","y":200}},{{"type":"rectangle","id":"fourth","y":300}}]}}]}}],"children":[]{editor_meta}}}"#
        );
        let loaded = load_canonical(&src).expect("guarded document loads");
        let page = &loaded.value.pages.as_ref().expect("pages")[0];
        let PenNode::Frame(stack) = &page.children[0] else {
            panic!("stack frame expected");
        };
        assert_eq!(
            ids(stack.children.as_deref().expect("stack children")),
            ["first", "second", "third", "fourth"],
            "the explicit-field/absent-meta gate wins despite strong flow evidence"
        );
    }
}

#[test]
fn image_node_without_src_loads_via_injected_empty_src() {
    // LLM-authored image node: `imageSearchQuery` / `imagePrompt` but no
    // `src`. jian's `ImageNode.src` is required, so this must be repaired
    // on load rather than rejected (the web app opened these files fine).
    let doc = r#"{"version":"1.0.0","children":[{"id":"hero","type":"image","name":"Hero","width":120,"height":80,"imageSearchQuery":"pizza","imagePrompt":"a hot pizza"}]}"#;
    let loaded = load_canonical(doc).expect("image node without src must load");
    assert_eq!(loaded.value.children.len(), 1, "the image node should load");
}

#[test]
fn video_alias_loads_as_an_image_with_the_poster_source() {
    let doc = r#"{
      "version":"1.0.0",
      "children":[{
        "type":"video",
        "id":"hero",
        "name":"Hero video",
        "src":"https://cdn.example.com/hero.mp4",
        "poster":"data:image/png;base64,AA==",
        "autoplay":true,
        "playback":{"holdLastFrame":true,"clickToReplay":true}
      }],
      "editorMeta":{"preserveAuthoredGeometry":true}
    }"#;
    let loaded = load_canonical(doc).expect("video alias must load");
    let PenNode::Image(image) = &loaded.value.children[0] else {
        panic!("video alias must become an image node");
    };
    assert_eq!(image.src.as_str(), "data:image/png;base64,AA==");
    let video = image.video.as_ref().expect("video metadata");
    assert_eq!(video.src, "https://cdn.example.com/hero.mp4");
    assert!(video.autoplay);
    assert!(video.hold_last_frame);
    assert!(video.click_to_replay);
}

#[test]
fn node_fill_as_color_string_is_wrapped_into_solid_fill() {
    // Older TS files write a PenNode `fill` as a bare color string where
    // jian expects `Vec<PenFill>`. Must be wrapped, not rejected. The
    // type-less `StyledTextSegment.fill` (also a string) stays a string.
    let doc = r##"{"version":"1.0.0","children":[{"id":"bg","type":"frame","name":"BG","x":0,"y":0,"width":100,"height":100,"fill":"#1A1D2E","children":[]}]}"##;
    let loaded = load_canonical(doc).expect("string fill must load");
    assert_eq!(loaded.value.children.len(), 1, "the frame should load");
}

#[test]
fn path_geometry_is_repaired_before_typed_conversion() {
    let doc = r#"{"version":"1.0.0","children":[{"id":"p","type":"path","geometry":"M0 0L9 9"}]}"#;
    let loaded = load_canonical(doc).expect("legacy path geometry must load");
    let PenNode::Path(path) = &loaded.value.children[0] else {
        panic!("path node expected");
    };
    assert_eq!(path.d.as_deref(), Some("M0 0L9 9"));
}

#[test]
fn disabled_legacy_fill_entries_stay_unpainted() {
    let doc = r##"{"version":"1.0.0","children":[{"id":"f","type":"frame",
      "fill":[{"type":"solid","color":"#ff0000","enabled":false}],
      "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#00ff00","enabled":false}]},
      "children":[]}]}"##;
    let loaded = load_canonical(doc).expect("disabled legacy fills must load");
    let PenNode::Frame(frame) = &loaded.value.children[0] else {
        panic!("frame node expected");
    };
    assert!(frame.container.fill.as_ref().is_some_and(Vec::is_empty));
    assert!(frame
        .container
        .stroke
        .as_ref()
        .and_then(|stroke| stroke.fill.as_ref())
        .is_some_and(Vec::is_empty));
}

#[test]
fn unsupported_legacy_version_and_fill_load_in_one_preprocessed_dom() {
    let doc = r##"{"version":"2.8","children":[{"id":"f","type":"frame","fill":"#123456","children":[]}]}"##;
    let loaded = load_canonical(doc).expect("best-effort legacy document must load");
    assert_eq!(loaded.value.version, "1.0");
    let PenNode::Frame(frame) = &loaded.value.children[0] else {
        panic!("frame node expected");
    };
    assert_eq!(frame.container.fill.as_ref().map(Vec::len), Some(1));
}
