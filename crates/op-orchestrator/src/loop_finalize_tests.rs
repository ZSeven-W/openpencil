//! Tests for the Track-1 Step-4 agentic-loop finalize backstop.

use super::apply_loop_finalize;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::json;

/// Build an [`EditorState`] whose active page holds `nodes` as its top-level
/// children — the shape the agentic loop assembles by loop end (sections
/// inserted under the active page, the same forest contract the orchestrator's
/// per-subtask passes assume: each top-level node is a section). `InsertSubtree`
/// remaps node ids, so the tests below look nodes up BY NAME, not by the
/// authored id.
fn state_with_forest(nodes: serde_json::Value) -> EditorState {
    let parsed: Vec<jian_ops_schema::node::PenNode> =
        serde_json::from_value(nodes).expect("valid PenNode forest");
    let mut state = EditorState::new();
    state.apply(EditorCommand::InsertSubtree {
        nodes: parsed,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    state
}

/// First node (depth-first) whose name matches `name`.
fn find_by_name<'a>(
    nodes: &'a [jian_ops_schema::node::PenNode],
    name: &str,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    for n in nodes {
        if n.base().name.as_deref() == Some(name) {
            return Some(n);
        }
        if let Some(c) = n.children() {
            if let Some(hit) = find_by_name(c, name) {
                return Some(hit);
            }
        }
    }
    None
}

fn role_of(state: &EditorState, name: &str) -> Option<String> {
    find_by_name(state.active_children(), name).and_then(|n| n.base().role.clone())
}

fn fill_of(state: &EditorState, name: &str) -> Option<serde_json::Value> {
    let n = find_by_name(state.active_children(), name)?;
    let v = serde_json::to_value(n).ok()?;
    Some(v.get("fill").cloned().unwrap_or(json!(null)))
}

#[test]
fn loop_finalize_preserves_explicit_mobile_viewport_with_tall_content() {
    let mut state = state_with_forest(json!([{
        "type": "frame",
        "id": "root",
        "name": "Result",
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "children": [
            {"type":"frame", "id":"status", "name":"Status Bar", "width":"fill_container", "height":62},
            {"type":"frame", "id":"viewport", "name":"Scroll Viewport", "role":"viewport",
             "width":"fill_container", "height":"fill_container", "layout":"vertical", "clipContent":true,
             "children":[{"type":"frame", "id":"long", "name":"Long Content", "width":"fill_container", "height":1000}]},
            {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":390, "height":72, "layout":"horizontal"}
        ]
    }]));

    apply_loop_finalize(&mut state);

    let root = find_by_name(state.active_children(), "Result").expect("root survives");
    assert_eq!(
        root.height_px(),
        Some(844.0),
        "an explicit clipped fill-height viewport keeps its numeric root"
    );
}

/// `apply_loop_finalize` on a small whole-doc forest must run the Class-A
/// sequence: it resolves semantic roles (proving `resolve_forest_roles` ran)
/// AND strips a white structural-wrapper fill (proving `post_pass_forest`'s
/// structural-wrapper-transparency ran on the live document).
#[test]
fn loop_finalize_resolves_roles_and_strips_white_wrapper() {
    // Two top-level sections (the loop's forest). The first is named "Header"
    // with NO role — role inference must assign navbar. The second is an
    // explicitly-`section`-roled wrapper glm gave an inner-card white fill on
    // its CHILD wrapper — the structural-wrapper-transparency pass must strip
    // that inner white wrapper.
    let mut state = state_with_forest(json!([
        {
            "type": "frame",
            "id": "header",
            "name": "Header",
            "x": 0, "y": 0, "width": 1200, "height": 64,
            "children": [
                { "type": "text", "id": "logo", "name": "Logo", "content": "Acme" }
            ]
        },
        {
            "type": "frame",
            "id": "content",
            "name": "Content",
            "x": 0, "y": 64, "width": 1200, "height": 400,
            "children": [
                {
                    "type": "frame",
                    "id": "wrapper",
                    "name": "Inner Section",
                    "role": "section",
                    "x": 0, "y": 0, "width": 1200, "height": 400,
                    "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                    "children": [
                        { "type": "text", "id": "body", "name": "Body", "content": "Hello" }
                    ]
                }
            ]
        }
    ]));

    apply_loop_finalize(&mut state);

    // (a) Roles resolved: the roleless "Header" frame inferred a navbar role.
    assert_eq!(
        role_of(&state, "Header").as_deref(),
        Some("navbar"),
        "resolve_forest_roles must run — the 'Header' frame should infer the navbar role"
    );

    // (b) White structural wrapper stripped: the nested `section`-roled
    // wrapper's white fill is forced transparent.
    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([])),
        "post_pass_forest structural-wrapper-transparency must run — the white section fill should be stripped"
    );
}

/// When the doc is a single page-root wrapper (sections inside one page frame —
/// the realistic agentic-loop structure), the Class-A section passes run on the
/// wrapper's CHILDREN: roles resolve + a white inner-section wrapper is stripped
/// while the page-root's OWN background fill survives (it is the page surface,
/// not a redundant section fill).
#[test]
fn loop_finalize_preserves_page_root_background() {
    let mut state = state_with_forest(json!([{
        "type": "frame",
        "id": "page",
        "name": "Page",
        "x": 0, "y": 0, "width": 1200, "height": 900,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "header",
                "name": "Header",
                "x": 0, "y": 0, "width": 1200, "height": 64,
                "children": [
                    { "type": "text", "id": "logo", "name": "Logo", "content": "Acme" }
                ]
            },
            {
                "type": "frame",
                "id": "wrapper",
                "name": "Inner Section",
                "role": "section",
                "x": 0, "y": 64, "width": 1200, "height": 400,
                "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                "children": [
                    { "type": "text", "id": "body", "name": "Body", "content": "Hello" }
                ]
            }
        ]
    }]));

    apply_loop_finalize(&mut state);

    // Roles resolved on the page-root's children.
    assert_eq!(
        role_of(&state, "Header").as_deref(),
        Some("navbar"),
        "role inference must run on the page-root's section children"
    );
    // The inner white section wrapper is stripped transparent.
    assert_eq!(
        fill_of(&state, "Inner Section"),
        Some(json!([])),
        "the inner white section wrapper should be stripped"
    );
    // The page-root's OWN background fill survives — it is NOT a section.
    assert_eq!(
        fill_of(&state, "Page"),
        Some(json!([{ "type": "solid", "color": "#FFFFFF" }])),
        "the page-root background fill must survive (it is the page surface, not a section)"
    );
}

/// Empty document → no panic, no-op.
#[test]
fn loop_finalize_on_empty_doc_is_noop() {
    let mut state = EditorState::new();
    apply_loop_finalize(&mut state);
    assert!(state.active_children().is_empty());
}

#[test]
fn loop_finalize_removes_sparse_overlapping_duplicate_root() {
    let sparse_children: Vec<serde_json::Value> = (0..5)
        .map(|idx| {
            json!({
                "type": "text",
                "id": format!("stub-{idx}"),
                "name": "Header Item",
                "content": "Header"
            })
        })
        .collect();
    let rich_children: Vec<serde_json::Value> = (0..168)
        .map(|idx| {
            json!({
                "type": "text",
                "id": format!("real-{idx}"),
                "name": "Content Item",
                "content": "Content"
            })
        })
        .collect();
    let mut state = state_with_forest(json!([
        {
            "type": "frame",
            "id": "stub",
            "name": "Explore",
            "width": 390,
            "height": 844,
            "fill": [{ "type": "solid", "color": "#FFFDF7" }],
            "children": sparse_children
        },
        {
            "type": "frame",
            "id": "real",
            "name": "Explore",
            "width": 390,
            "height": "fit_content",
            "children": rich_children
        }
    ]));

    apply_loop_finalize(&mut state);

    let roots = state.active_children();
    assert_eq!(
        roots.len(),
        1,
        "loop finalize should delete the sparse root"
    );
    assert_eq!(roots[0].base().name.as_deref(), Some("Explore"));
    assert_eq!(crate::cleanup::count_descendants(&roots[0]), 168);
}

/// The reported glm bug shape (a single page-root wrapper whose first child is a
/// full-width sidebar) is restructured into a horizontal `[sidebar | Main
/// Content]` app-shell by `apply_loop_finalize`, AND the restructure survives
/// the subsequent Class-A passes — in particular the sidebar is NOT re-widened
/// by `fix_horizontal_overflow` (the guard added for this).
#[test]
fn loop_finalize_restructures_sidebar_dashboard() {
    let mut state = state_with_forest(json!([
        {
            "type": "frame", "id": "root", "name": "Barbershop Client Management",
            "width": 1200, "height": 1775, "layout": "vertical", "gap": 32,
            "children": [
                { "type": "frame", "id": "sb", "name": "Sidebar Navigation",
                  "width": 1200, "height": 605, "layout": "vertical",
                  "children": [
                    { "type": "frame", "id": "logo", "name": "Logo",
                      "width": 1152, "height": 27, "layout": "vertical", "children": [] }
                  ] },
                { "type": "frame", "id": "hd", "name": "Top Header",
                  "width": 1200, "height": 94, "layout": "vertical", "children": [] },
                { "type": "frame", "id": "km", "name": "Key Metrics",
                  "width": 1200, "height": 117, "layout": "horizontal", "children": [] },
                { "type": "frame", "id": "ct", "name": "Client Table Section",
                  "width": 1200, "height": 488, "layout": "vertical", "children": [] },
                { "type": "frame", "id": "ua", "name": "Upcoming Appointments",
                  "width": 1200, "height": 391, "layout": "vertical", "children": [] }
            ]
        }
    ]));
    apply_loop_finalize(&mut state);

    let root = &state.active_children()[0];
    let rv = serde_json::to_value(root).unwrap();
    assert_eq!(
        rv.get("layout").and_then(|l| l.as_str()),
        Some("horizontal"),
        "root restructured to a horizontal app-shell"
    );
    assert_eq!(
        rv["children"].as_array().unwrap().len(),
        2,
        "[sidebar | Main Content]"
    );

    // Sidebar stayed a narrow left column through the whole sequence (the
    // fix_horizontal_overflow guard must NOT re-widen this vertical frame).
    let sidebar = find_by_name(state.active_children(), "Sidebar Navigation").unwrap();
    let sv = serde_json::to_value(sidebar).unwrap();
    assert!(
        sv["width"].as_f64().is_some_and(|w| w <= 300.0),
        "sidebar must stay narrow, got {:?}",
        sv["width"]
    );

    // The data sections moved into the new content column, in order.
    let content =
        find_by_name(state.active_children(), "Main Content").expect("content column created");
    let cv = serde_json::to_value(content).unwrap();
    let names: Vec<String> = cv["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(
        names,
        [
            "Top Header",
            "Key Metrics",
            "Client Table Section",
            "Upcoming Appointments"
        ],
        "data sections nested under Main Content in order"
    );
}

#[test]
fn loop_finalize_gives_fill_less_text_a_visible_fill_on_dark_surface() {
    // glm-5.2 in the loop builds a dark design but omits `fill` on text nodes,
    // so they render with the default color — invisible on the dark surfaces it
    // also built. apply_loop_finalize must inject a background-contrasting fill.
    let mut state = state_with_forest(json!([{
        "type": "frame", "id": "root", "name": "Page", "layout": "vertical",
        "width": 1200, "height": 800,
        "fill": [{ "type": "solid", "color": "#0A0A0A" }],
        "children": [{
            "type": "frame", "id": "card", "name": "Card", "layout": "vertical",
            "width": 400, "height": 200,
            "fill": [{ "type": "solid", "color": "#0D0D0D" }],
            "children": [
                { "type": "text", "id": "label", "name": "Label", "content": "Total Clients", "fontSize": 14 }
            ]
        }]
    }]));
    let before = fill_of(&state, "Label").unwrap_or(json!(null));
    assert!(
        before.is_null() || before.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "precondition: Label starts fill-less, got {before:?}"
    );
    apply_loop_finalize(&mut state);
    let after = fill_of(&state, "Label").expect("Label survives finalize");
    assert!(
        after.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "fill-less text on a dark surface must get a visible fill; got {after:?}"
    );
}

#[test]
fn loop_finalize_hoists_node_state_to_doc_root() {
    let mut state = state_with_forest(json!([
        {"type": "frame", "id": "sec", "name": "Hero", "width": 1200, "height": 400,
         "state": {"count": {"type": "int", "default": 3}},
         "children": [
             {"type": "text", "id": "t", "name": "HeroLabel", "content": "hi"}
         ]}
    ]));
    apply_loop_finalize(&mut state);

    // Node-level `state` must be stripped from the tree…
    let hero = find_by_name(state.active_children(), "Hero").expect("hero survives finalize");
    let v = serde_json::to_value(hero).expect("serialize hero");
    assert!(v.get("state").is_none(), "node state must be stripped");

    // …and merged into the document root (the `$app` store).
    let root_state = state.doc.state.as_ref().expect("doc-root state seeded");
    assert_eq!(
        root_state.get("count").and_then(|e| e.default.clone()),
        Some(json!(3))
    );
}

#[test]
fn loop_finalize_leaves_text_on_gradient_surface_alone() {
    // A gradient (or image) fill cannot be reduced to a solid hex — guessing a
    // text color there risks dark-on-dark, the mirror of the bug the injection
    // pass fixes. Texts under an indeterminate surface must be left fill-less.
    let mut state = state_with_forest(json!([{
        "type": "frame", "id": "root", "name": "Page", "layout": "vertical",
        "width": 1200, "height": 800,
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "children": [{
            "type": "frame", "id": "hero", "name": "Hero", "layout": "vertical",
            "width": 1200, "height": 400,
            "fill": [{ "type": "linear_gradient",
                       "stops": [ { "offset": 0.0, "color": "#0B1020" },
                                  { "offset": 1.0, "color": "#3B0764" } ] }],
            "children": [
                { "type": "text", "id": "h", "name": "Hero Title", "content": "Launch", "fontSize": 42 }
            ]
        }]
    }]));
    apply_loop_finalize(&mut state);
    let after = fill_of(&state, "Hero Title").unwrap_or(json!(null));
    assert!(
        after.is_null() || after.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "text on a gradient surface must NOT get a guessed fill; got {after:?}"
    );
}

#[test]
fn theme_polarity_split_variables_are_healed() {
    // test07021's verbatim split: a DARK design whose active (Light-slot)
    // values are dark for bg/text but stock-light for surface-2 — the chip
    // resolved #F1F5F9 on a #0A0A0A page. The dark value exists in the other
    // slot; the polarity pass must adopt it. text-primary (correctly light on
    // dark) must be left alone.
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "variables": {
            "color-bg-deep": {"type":"color","value":[
                {"value":"#0A0A0A","theme":{"Mode":"Light"}},
                {"value":"#0F172A","theme":{"Mode":"Dark"}}]},
            "color-surface-2": {"type":"color","value":[
                {"value":"#F1F5F9","theme":{"Mode":"Light"}},
                {"value":"#334155","theme":{"Mode":"Dark"}}]},
            "color-text-primary": {"type":"color","value":[
                {"value":"#FAFAFA","theme":{"Mode":"Light"}},
                {"value":"#F1F5F9","theme":{"Mode":"Dark"}}]}
        },
        "themes": {"Mode": ["Light", "Dark"]},
        "children": [
            {"type":"frame","id":"root","name":"Page","width":1200,"height":800,
             "fill":[{"type":"solid","color":"$color-bg-deep"}],"children":[]}
        ]
    }))
    .expect("valid doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::loop_finalize::fix_theme_variable_polarity(&mut sink);
    let s2 = state
        .resolve_color_variable_hex("color-surface-2")
        .expect("resolves");
    assert!(
        s2.eq_ignore_ascii_case("#334155"),
        "light-slot surface-2 adopted the dark value, got {s2}"
    );
    let tp = state
        .resolve_color_variable_hex("color-text-primary")
        .expect("resolves");
    assert!(
        tp.eq_ignore_ascii_case("#FAFAFA"),
        "correctly-opposite text variable untouched, got {tp}"
    );
}

#[test]
fn loop_finalize_does_not_delete_or_resize_ambiguous_image_layers() {
    let mut state = state_with_forest(serde_json::from_str(r##"[{
        "type": "frame", "id": "root", "name": "Travel Page", "width": 800, "height": 600,
        "layout": "vertical", "children": [{
            "type": "frame", "id": "section", "name": "Popular", "width": "fill_container",
            "height": 260, "layout": "vertical", "children": [{
                "type": "frame", "id": "rail", "name": "Destination Rail", "width": "fill_container",
                "height": 220, "layout": "horizontal", "children": [{
                    "type": "frame", "id": "card", "name": "Bali Card", "width": 200,
                    "height": 210, "layout": "vertical", "children": [{
                        "type": "frame", "id": "band", "name": "Bali Media", "width": 200,
                        "height": 56, "layout": "none", "clipContent": true, "children": [
                            {"type":"frame","id":"plate","name":"Bali Image Plate","x":0,"y":0,
                             "width":200,"height":130,"fill":[{"type":"image","url":"data:image/png;base64,AAAA"}]},
                            {"type":"image","id":"photo","name":"Bali Photo","x":0,"y":0,
                             "width":200,"height":130,"src":"data:image/png;base64,BBBB"}
                        ]
                    }]
                }]
            }]
        }]
    }]"##).expect("valid image-layer fixture"));

    apply_loop_finalize(&mut state);

    let band = find_by_name(state.active_children(), "Bali Media").expect("band survives");
    assert_eq!(
        band.height_px(),
        Some(56.0),
        "a clipped band stays authored; image height is not design intent"
    );
    assert!(
        find_by_name(state.active_children(), "Bali Image Plate").is_some(),
        "an image-filled layer is not deleted merely because an image node shares its parent"
    );
    let photo = find_by_name(state.active_children(), "Bali Photo").expect("photo survives");
    assert_eq!(
        photo.height_px(),
        Some(56.0),
        "the image may conform to the explicit slot, but the slot never expands from image defaults"
    );
}

#[test]
fn loop_finalize_does_not_turn_image_cards_into_category_chips() {
    let mut state = state_with_forest(serde_json::from_str(r##"[{
        "type":"frame", "id":"root", "name":"Travel", "width":375, "height":800,
        "layout":"vertical", "children":[{
            "type":"frame", "id":"popular", "name":"Popular Destinations",
            "width":"fill_container", "height":"fit_content", "layout":"vertical",
            "children":[{
                "type":"frame", "id":"row", "name":"Cards Row", "layout":"horizontal",
                "width":"fit_content", "height":"fit_content", "gap":12,
                "justifyContent":"space_between", "children":[
                    {"type":"frame", "id":"c1", "name":"Santorini Card", "layout":"vertical",
                     "width":"fill_container", "height":"fill_container", "gap":12,
                     "cornerRadius":16, "children":[
                        {"type":"frame", "id":"p1", "name":"Santorini Photo Wrap",
                         "layout":"none", "width":"fill_container", "height":140,
                         "clipContent":true, "children":[
                            {"type":"image", "id":"i1", "name":"Santorini Photo", "src":"",
                             "imagePrompt":"Santorini Greece", "width":"fill_container", "height":140},
                            {"type":"frame", "id":"f1", "name":"Favorite", "children":[
                                {"type":"icon_font", "id":"h1", "iconFontName":"heart", "width":18, "height":18}
                            ]}
                         ]},
                        {"type":"text", "id":"t1", "content":"Santorini", "width":"fit_content", "height":"fit_content"}
                     ]},
                    {"type":"frame", "id":"c2", "name":"Kyoto Card", "layout":"vertical",
                     "width":"fill_container", "height":"fill_container", "gap":12,
                     "cornerRadius":16, "children":[
                        {"type":"frame", "id":"p2", "name":"Kyoto Photo Wrap",
                         "layout":"none", "width":"fill_container", "height":140,
                         "clipContent":true, "children":[
                            {"type":"image", "id":"i2", "name":"Kyoto Photo", "src":"",
                             "imagePrompt":"Kyoto temple", "width":"fill_container", "height":140},
                            {"type":"frame", "id":"f2", "name":"Favorite", "children":[
                                {"type":"icon_font", "id":"h2", "iconFontName":"heart", "width":18, "height":18}
                            ]}
                         ]},
                        {"type":"text", "id":"t2", "content":"Kyoto", "width":"fit_content", "height":"fit_content"}
                     ]},
                    {"type":"frame", "id":"c3", "name":"Lisbon Card", "layout":"vertical",
                     "width":"fill_container", "height":"fill_container", "gap":12,
                     "cornerRadius":16, "children":[
                        {"type":"frame", "id":"p3", "name":"Lisbon Photo Wrap",
                         "layout":"none", "width":"fill_container", "height":140,
                         "clipContent":true, "children":[
                            {"type":"image", "id":"i3", "name":"Lisbon Photo", "src":"",
                             "imagePrompt":"Lisbon Portugal", "width":"fill_container", "height":140},
                            {"type":"frame", "id":"f3", "name":"Favorite", "children":[
                                {"type":"icon_font", "id":"h3", "iconFontName":"heart", "width":18, "height":18}
                            ]}
                         ]},
                        {"type":"text", "id":"t3", "content":"Lisbon", "width":"fit_content", "height":"fit_content"}
                     ]}
                ]
            }]
        }]
    }]"##).expect("valid image-card rail fixture"));

    apply_loop_finalize(&mut state);

    let card = find_by_name(state.active_children(), "Santorini Card").expect("card survives");
    let card_json = serde_json::to_value(card).expect("card json");
    assert_eq!(card_json["cornerRadius"], json!(16.0));
    let media = find_by_name(state.active_children(), "Santorini Photo Wrap")
        .expect("media wrapper survives");
    let media_json = serde_json::to_value(media).expect("media json");
    assert_eq!(media_json["height"], json!(140.0));
    assert_eq!(media_json["layout"], json!("none"));
}

#[test]
fn loop_finalize_preserves_fixed_cards_in_explicit_scroller() {
    let card = |id: &str, name: &str| {
        json!({
            "type":"frame", "id":id, "name":name, "layout":"vertical",
            "width":210, "height":240, "children":[
                {"type":"image", "id":format!("{id}-image"), "name":format!("{name} Photo"),
                 "src":"https://example.com/photo.jpg", "width":210, "height":140},
                {"type":"icon_font", "id":format!("{id}-heart"), "iconFontName":"heart",
                 "width":18, "height":18},
                {"type":"text", "id":format!("{id}-label"), "content":name,
                 "width":"fit_content", "height":20}
            ]
        })
    };
    let forest = json!([{
        "type":"frame", "id":"root", "name":"Travel", "width":390, "height":844,
        "layout":"vertical", "children":[{
            "type":"frame", "id":"rail", "name":"Popular Scroller",
            "width":"fill_container", "height":"fit_content", "layout":"horizontal",
            "justifyContent":"start", "clipContent":true, "gap":12,
            "children":[card("c1", "Santorini"), card("c2", "Kyoto"), card("c3", "Banff")]
        }]
    }]);
    let mut state = state_with_forest(serde_json::from_value(forest).expect("valid scroller"));

    apply_loop_finalize(&mut state);

    let rail = find_by_name(state.active_children(), "Popular Scroller").expect("rail survives");
    let rail_json = serde_json::to_value(rail).expect("rail json");
    assert_eq!(rail_json["justifyContent"], json!("start"));
    assert_eq!(rail_json["clipContent"], json!(true));
    for name in ["Santorini", "Kyoto", "Banff"] {
        let card = find_by_name(state.active_children(), name).expect("card survives");
        assert_eq!(
            serde_json::to_value(card).expect("card json")["width"],
            json!(210.0),
            "fixed card width is the authored overflow intent"
        );
    }
}
