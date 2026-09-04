//! Command-recording parity for the whole-document loop finalizer.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

use super::{apply_loop_finalize_counted, record_loop_finalize_counted};

fn state(source: &str) -> EditorState {
    let doc = serde_json::from_str(source).expect("valid canonical document fixture");
    EditorState::from_document(doc)
}

fn node<'a>(state: &'a EditorState, id: &str) -> &'a PenNode {
    op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id))
        .unwrap_or_else(|| panic!("node {id} exists"))
}

fn canonical(state: &EditorState) -> serde_json::Value {
    serde_json::to_value(&state.doc).expect("canonical document serializes")
}

fn assert_direct_parity(input: &EditorState) -> EditorState {
    let mut direct = input.clone();
    let _ = apply_loop_finalize_counted(&mut direct);

    let recorded = record_loop_finalize_counted(input).expect("recorded finalizer succeeds");
    assert_eq!(canonical(&recorded.state), canonical(&direct));
    assert!(
        recorded
            .commands
            .iter()
            .all(|command| !matches!(command, EditorCommand::Batch { .. })),
        "the host replay Batch must never contain a nested Batch: {:?}",
        recorded.commands
    );

    let mut replay = input.clone();
    if !recorded.commands.is_empty() {
        assert!(replay.apply(EditorCommand::Batch {
            commands: recorded.commands,
        }));
    }
    assert_eq!(canonical(&replay), canonical(&direct));
    replay
}

#[test]
fn recorded_finalize_replays_promotion_and_app_state_hoist() {
    let input = state(
        r##"{
          "version":"1.1","formatVersion":"1.1","children":[{
            "type":"frame","id":"root","name":"Login","width":390,"height":844,
            "layout":"vertical","children":[{
              "type":"frame","id":"email","name":"Email input","role":"input",
              "width":342,"height":48,"layout":"horizontal",
              "state":{"email":{"type":"string","default":""}},
              "bindings":{"value":"$app.email"},
              "children":[
                {"type":"icon_font","id":"mail","iconFontName":"mail","width":20,"height":20},
                {"type":"text","id":"placeholder","content":"name@example.com",
                 "fill":[{"type":"solid","color":"#9CA3AF"}]}
              ]
            }]
          }]
        }"##,
    );

    let replay = assert_direct_parity(&input);
    assert!(matches!(node(&replay, "email"), PenNode::TextInput(_)));
    assert!(
        op_editor_core::walkers::find_node(replay.active_children(), &NodeId::new("placeholder"))
            .is_none(),
        "promotion consumes the old frame descendants"
    );
    let promoted = serde_json::to_value(node(&replay, "email")).unwrap();
    assert!(promoted.get("state").is_none(), "node state is stripped");
    let document = canonical(&replay);
    assert_eq!(
        document["state"]["email"]["default"], "",
        "hoisted app state reaches the document root"
    );
}

#[test]
fn recorded_finalize_replays_same_id_text_to_icon_leaf_conversion() {
    let input = state(
        r##"{
          "version":"1.1","formatVersion":"1.1","children":[{
            "type":"frame","id":"root","name":"Shop Home","width":390,"height":844,
            "layout":"vertical","children":[{
              "type":"frame","id":"section-header","name":"Featured Header",
              "width":"fill_container","height":40,"layout":"horizontal",
              "children":[
                {"type":"text","id":"heading","name":"Section title","content":"Featured",
                 "fontSize":20,"fontWeight":700},
                {"type":"text","id":"see-all","name":"See all","content":"View all >",
                 "fontSize":14}
              ]
            }]
          }]
        }"##,
    );

    let replay = assert_direct_parity(&input);
    let PenNode::IconFont(icon) = node(&replay, "see-all") else {
        panic!("section-header action should become an icon_font leaf");
    };
    assert_eq!(icon.icon_font_name, "chevron-right");
}

#[test]
fn recorded_finalize_preserves_six_tile_desktop_category_rail() {
    let input = state(
        r##"{
          "version":"1.1","formatVersion":"1.1","children":[{
            "type":"frame","id":"root","name":"Shop Home","width":1440,
            "height":"fit_content","layout":"vertical","children":[{
              "type":"frame","id":"categories","name":"Shop by category",
              "width":"fill_container","height":"fit_content","layout":"vertical",
              "children":[{
                "type":"frame","id":"rail","name":"Category tiles",
                "width":"fill_container","height":232,"layout":"horizontal",
                "gap":24,"justifyContent":"space_between","children":[
                  {"type":"frame","id":"home","name":"Home tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"home-icon","iconFontName":"house"},{"type":"text","id":"home-label","content":"Home"}]},
                  {"type":"frame","id":"bags","name":"Bags tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"bags-icon","iconFontName":"shopping-bag"},{"type":"text","id":"bags-label","content":"Bags"}]},
                  {"type":"frame","id":"apparel","name":"Apparel tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"apparel-icon","iconFontName":"shirt"},{"type":"text","id":"apparel-label","content":"Apparel"}]},
                  {"type":"frame","id":"lighting","name":"Lighting tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"lighting-icon","iconFontName":"lamp"},{"type":"text","id":"lighting-label","content":"Lighting"}]},
                  {"type":"frame","id":"kitchen","name":"Kitchen tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"kitchen-icon","iconFontName":"cooking-pot"},{"type":"text","id":"kitchen-label","content":"Kitchen"}]},
                  {"type":"frame","id":"furniture","name":"Furniture tile","layout":"vertical","width":196,"height":150,"children":[{"type":"icon_font","id":"furniture-icon","iconFontName":"sofa"},{"type":"text","id":"furniture-label","content":"Furniture"}]}
                ]
              }]
            }]
          }]
        }"##,
    );

    let replay = assert_direct_parity(&input);
    assert!(
        op_editor_core::walkers::find_node(replay.active_children(), &NodeId::new("furniture"))
            .is_some(),
        "desktop finalization must preserve the sixth authored category tile"
    );
    assert_eq!(
        node(&replay, "rail").height_px(),
        Some(232.0),
        "desktop finalization must preserve the authored category rail height"
    );
}

#[test]
fn recorded_finalize_replays_mobile_header_adoption_and_content_rails() {
    let input = state(
        r##"{
          "version":"1.1","formatVersion":"1.1","children":[{
            "type":"frame","id":"root","name":"Shop Home","width":390,"height":1000,
            "layout":"vertical","children":[
              {"type":"frame","id":"header","name":"Header","width":"fill_container",
               "height":64,"layout":"horizontal","children":[]},
              {"type":"text","id":"brand","name":"Brand","content":"Nook","fontSize":28},
              {"type":"frame","id":"search","name":"Search","width":"fill_container",
               "height":44,"layout":"horizontal","children":[
                 {"type":"text","id":"query","content":"Search products"}
               ]},
              {"type":"icon_font","id":"cart","name":"Cart","iconFontName":"shopping-bag",
               "width":24,"height":24},
              {"type":"text","id":"section-title","name":"Section title","content":"Featured",
               "fontSize":20},
              {"type":"frame","id":"hero","name":"Hero","width":342,"height":220,
               "layout":"vertical","children":[]}
            ]
          }]
        }"##,
    );

    let replay = assert_direct_parity(&input);
    let header = node(&replay, "header");
    let header_ids: Vec<&str> = header
        .children()
        .expect("header remains a container")
        .iter()
        .map(PenNodeExt::id_str)
        .collect();
    assert_eq!(header_ids, vec!["brand", "cart"]);
    let title = node(&replay, "section-title");
    assert_ne!(
        title.id_str(),
        replay.active_children()[0].id_str(),
        "section title survives under the mobile document"
    );
    let title_parent = parent_of(replay.active_children(), "section-title")
        .expect("section title has a parent after rail repair");
    assert_ne!(
        title_parent, "root",
        "naked title is wrapped in a content rail"
    );
}

#[test]
fn recorded_finalize_flattens_atomic_duplicate_page_shell_merge() {
    // Two direct-JS generation batches can each author an "App Content"
    // shell. The mobile repair intentionally lands the merge as an atomic
    // Batch (moves the second shell's children, then deletes that shell).
    // The whole-finalize recorder must preserve those operations without
    // nesting that Batch inside the host replay Batch, which EditorState
    // correctly rejects.
    let input = duplicate_mobile_page_shells();

    let replay = assert_direct_parity(&input);
    assert!(
        op_editor_core::walkers::find_node(replay.active_children(), &NodeId::new("content-b"))
            .is_none(),
        "the duplicate shell is removed"
    );
    assert!(
        canonical(&replay).to_string().contains("Original design"),
        "content from the duplicate shell survives the merge and later cleanup"
    );
}

#[test]
fn recorded_finalize_replays_after_u_repair_and_refinalizes_idempotently() {
    // Mirrors the native-MCP sequence that exposed the nested-Batch bug: DSH
    // applies a numeric U repair from the quality result, then calls finish
    // again on a mobile document whose duplicate page shells still need the
    // pass-local atomic merge.
    let mut repaired = duplicate_mobile_page_shells();
    assert!(repaired.apply(EditorCommand::UpdateNode {
        node_id: NodeId::new("header"),
        x: None,
        y: None,
        width: None,
        height: Some(61),
        name: None,
        fill_hex: None,
        page_id: None,
    }));
    assert_eq!(node(&repaired, "header").height_px(), Some(61.0));

    let finalized = assert_direct_parity(&repaired);
    let repeated =
        record_loop_finalize_counted(&finalized).expect("repeat finalize remains replayable");
    assert!(
        repeated.commands.is_empty(),
        "the post-U finalized document is idempotent: {:?}",
        repeated.commands
    );
    assert_eq!(canonical(&repeated.state), canonical(&finalized));
}

fn duplicate_mobile_page_shells() -> EditorState {
    state(
        r##"{
          "version":"1.1","formatVersion":"1.1","children":[{
            "type":"frame","id":"root","name":"Shop Home","width":390,
            "height":"fit_content","minHeight":844,"layout":"vertical","gap":16,
            "children":[
              {"type":"frame","id":"status","role":"status-bar","height":62,"children":[]},
              {"type":"frame","id":"content-a","name":"App Content",
               "width":"fill_container","height":"fit_content","layout":"vertical",
               "gap":24,"padding":24,"children":[
                  {"type":"frame","id":"header","name":"Header","role":"navbar",
                   "width":"fill_container","height":44,"layout":"horizontal","children":[
                    {"type":"text","id":"brand","content":"Mono Market"}
                  ]},
                  {"type":"frame","id":"featured","name":"Featured Product",
                   "width":"fill_container","children":[
                    {"type":"text","id":"product","content":"Sweater"}
                  ]}
               ]},
              {"type":"frame","id":"content-b","name":"app-content",
               "width":"fill_container","height":487,"layout":"vertical",
               "gap":24,"padding":[24,24,24,24],"children":[
                  {"type":"frame","id":"benefits","name":"Benefits","children":[
                    {"type":"text","id":"benefit","content":"Original design"}
                  ]},
                  {"type":"frame","id":"cta","name":"CTA","role":"cta-section","children":[
                    {"type":"text","id":"cta-label","content":"Shop now"}
                  ]}
               ]}
            ]
          }]
        }"##,
    )
}

#[test]
fn recorded_finalize_is_command_idempotent_after_replay() {
    let input = state(
        r##"{"version":"1.1","children":[{
          "type":"frame","id":"root","name":"Page","width":1200,"height":844,
          "layout":"vertical","children":[{
            "type":"text","id":"title","content":"Hello",
            "fill":[{"type":"solid","color":"#111827"}]
          }]
        }]}"##,
    );
    let first = record_loop_finalize_counted(&input).expect("first finalize records");
    let second = record_loop_finalize_counted(&first.state).expect("second finalize records");
    assert!(
        second.commands.is_empty(),
        "an already-finalized document must not emit a version-bumping batch: {:?}",
        second.commands
    );
    assert_eq!(canonical(&second.state), canonical(&first.state));
}

fn parent_of<'a>(nodes: &'a [PenNode], target: &str) -> Option<&'a str> {
    for candidate in nodes {
        if let Some(children) = candidate.children() {
            if children.iter().any(|child| child.id_str() == target) {
                return Some(candidate.id_str());
            }
            if let Some(parent) = parent_of(children, target) {
                return Some(parent);
            }
        }
    }
    None
}
