//! Instance inspection / override editing host tests (GAP #10 + #22
//! + #26): property-panel commits on a Ref anchor route through the
//!
//! instance-write redirect, the panel's component lifecycle actions
//! dispatch, context-menu detach rows work, and remote icon inserts
//! bake their SVG `d`.

use super::WidgetHostNative;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::{NodeId, PenNodeExt};

const COMPONENT_DOC: &str = r##"{
  "version":"0.8.0",
  "children":[
    {"type":"frame","id":"card","name":"Card","reusable":true,"x":0,"y":0,"width":200,"height":100,
     "fill":[{"type":"solid","color":"#222222"}],
     "children":[
       {"type":"text","id":"title","name":"Title","content":"Hello"},
       {"type":"icon_font","id":"icon","name":"home","iconFontName":"home",
        "width":24,"height":24,"fill":[{"type":"solid","color":"#111111"}]}
     ]},
    {"type":"ref","id":"inst1","ref":"card","x":300,"y":50}
  ]
}"##;

fn seeded_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(COMPONENT_DOC)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst1"));
    host
}

#[test]
fn native_move_fill_action_dispatches_as_one_undoable_edit() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[{
          "type":"rectangle","id":"rect","name":"Rect",
          "x":0,"y":0,"width":10,"height":10,
          "fill":[
            {"type":"solid","color":"#111111"},
            {"type":"solid","color":"#222222"},
            {"type":"solid","color":"#333333"}
          ]
        }]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("rect"));

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::MoveFill {
        from: 2,
        to: 0,
    });

    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("rect"),
    )
    .expect("rect exists");
    let colors: Vec<_> = op_editor_core::fills::node_fills(node)
        .expect("fills exist")
        .iter()
        .map(|fill| match fill {
            PenFill::Solid(body) => body.color.as_str(),
            other => panic!("expected solid, got {other:?}"),
        })
        .collect();
    assert_eq!(colors, ["#333333", "#111111", "#222222"]);
    assert_eq!(host.editor_state().history.past.len(), 1);
}

#[test]
fn native_instance_move_fill_undo_restores_the_original_ref() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"master","name":"Master","reusable":true,
           "x":0,"y":0,"width":10,"height":10,
           "fill":[
             {"type":"solid","color":"#111111"},
             {"type":"solid","color":"#222222"},
             {"type":"solid","color":"#333333"}
           ]},
          {"type":"ref","id":"inst","ref":"master","x":20,"y":0}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst"));

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::MoveFill {
        from: 2,
        to: 0,
    });
    assert_eq!(host.editor_state().history.past.len(), 1);
    assert!(host.editor_state_mut().undo());

    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("inst"),
    )
    .expect("instance exists after undo");
    let PenNode::Ref(reference) = node else {
        panic!("undo must restore a Ref, got {node:?}");
    };
    assert!(
        reference.descendants.is_none(),
        "undo must remove the fill-order override"
    );
}

fn ref_node(host: &WidgetHostNative) -> &jian_ops_schema::node::RefNode {
    match op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("inst1"),
    ) {
        Some(PenNode::Ref(r)) => r,
        other => panic!("inst1 must stay a Ref, got {other:?}"),
    }
}

#[test]
fn fill_hex_commit_on_instance_lands_in_descendants() {
    let mut host = seeded_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::FillHex(0));
    host.editor_state_mut()
        .ui
        .property_input
        .set_text("#ff0000");
    host.commit_property_focus_if_any();
    let over = ref_node(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("card"))
        .expect("fill override routed under descendants[card]");
    // The host hex commit path re-cases through `color_to_hex`
    // (uppercase) — compare case-insensitively.
    assert_eq!(
        over.pointer("/fill/0/color")
            .and_then(|v| v.as_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("#ff0000")
    );
}

#[test]
fn position_commit_on_instance_writes_the_ref_base() {
    let mut host = seeded_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("400");
    host.commit_property_focus_if_any();
    let r = ref_node(&host);
    assert_eq!(r.base.x, Some(400.0), "x is an INSTANCE_DIRECT_PROP");
    assert!(r.descendants.is_none(), "no override for a direct prop");
}

#[test]
fn fill_hex_commit_on_virtual_child_lands_in_child_override() {
    let mut host = seeded_host();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst1__icon"));
    assert!(
        op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state()).is_some(),
        "virtual child keeps the inspector mounted"
    );
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::FillHex(0));
    host.editor_state_mut()
        .ui
        .property_input
        .set_text("#ff0000");
    host.commit_property_focus_if_any();
    let over = ref_node(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("icon"))
        .expect("fill override routed under descendants[icon]");
    assert_eq!(
        over.pointer("/fill/0/color")
            .and_then(serde_json::Value::as_str)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("#ff0000")
    );
}

#[test]
fn stroke_width_commit_on_virtual_child_lands_in_child_override() {
    let mut host = seeded_host();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst1__icon"));
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::StrokeWidth);
    host.editor_state_mut().ui.property_input.set_text("4");
    host.commit_property_focus_if_any();
    let over = ref_node(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("icon"))
        .expect("stroke override routed under descendants[icon]");
    assert_eq!(
        over.pointer("/stroke/thickness")
            .and_then(serde_json::Value::as_f64),
        Some(4.0)
    );
}

#[test]
fn color_picker_on_virtual_child_opens_edits_and_keeps_ref_history() {
    let mut host = seeded_host();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("inst1__icon"));
    assert!(host
        .editor_state_mut()
        .open_color_picker(op_editor_core::ui_draft::ColorTarget::Fill, 120.0));
    assert!(host.editor_state_mut().color_picker_set_hsv(0.0, 1.0, 1.0));
    assert!(host.editor_state_mut().close_color_picker());
    let over = ref_node(&host)
        .descendants
        .as_ref()
        .and_then(|d| d.get("icon"))
        .expect("picker edit routed under descendants[icon]");
    assert_eq!(
        over.pointer("/fill/0/color")
            .and_then(serde_json::Value::as_str),
        Some("#ff0000")
    );
    let snapshot = host
        .editor_state()
        .history
        .past
        .back()
        .expect("picker edit history");
    assert!(matches!(
        snapshot.doc.snapshot_find_node(0, &NodeId::new("inst1")),
        Some(PenNode::Ref(_))
    ));
}

#[test]
fn detach_instance_panel_action_materializes_the_ref() {
    let mut host = seeded_host();
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::DetachInstance);
    let anchor = host.editor_state().selection.anchor.clone();
    assert_ne!(anchor.as_str(), "inst1", "detach selects the new subtree");
    let detached =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &anchor)
            .expect("detached node on the page");
    assert!(
        matches!(detached, PenNode::Frame(_)),
        "the Ref materialized into the component's Frame"
    );
    assert!(
        op_editor_core::walkers::find_node(
            host.editor_state().active_children(),
            &NodeId::new("inst1")
        )
        .is_none(),
        "the Ref slot was replaced"
    );
}

#[test]
fn go_to_component_panel_action_selects_the_master() {
    let mut host = seeded_host();
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::GoToComponent);
    assert_eq!(host.editor_state().selection.anchor.as_str(), "card");
}

#[test]
fn context_menu_swaps_create_component_for_detach_rows() {
    use op_editor_core::editor_ui_state::{LayerContextMenuState, LayerContextTarget};
    use op_editor_ui::widgets::layer_context_menu::{LayerContextAction, LayerContextMenu};
    let host = seeded_host();
    let menu_for = |id: &str| {
        LayerContextMenu::for_state(
            host.editor_state(),
            LayerContextMenuState {
                target: LayerContextTarget::Layer(NodeId::new(id)),
                anchor_x: 0.0,
                anchor_y: 0.0,
                menu: Default::default(),
            },
        )
    };
    // Instance row → Detach Instance replaces Create Component.
    let menu = menu_for("inst1");
    let hits: Vec<_> = (0..10)
        .filter_map(|i| menu.hit_test(op_editor_ui::Point2D::new(4.0, 6.0 + 1.0 + i as f32 * 32.0)))
        .collect();
    assert!(hits.contains(&LayerContextAction::DetachInstance));
    assert!(!hits.contains(&LayerContextAction::CreateComponent));
    // Component row → Detach Component replaces Create Component.
    let menu = menu_for("card");
    let hits: Vec<_> = (0..10)
        .filter_map(|i| menu.hit_test(op_editor_ui::Point2D::new(4.0, 6.0 + 1.0 + i as f32 * 32.0)))
        .collect();
    assert!(hits.contains(&LayerContextAction::DetachComponent));
    assert!(!hits.contains(&LayerContextAction::CreateComponent));
}

#[test]
fn context_menu_detach_instance_dispatch_materializes() {
    use op_editor_core::ui_draft::LayerContextTarget;
    use op_editor_ui::widgets::layer_context_menu::LayerContextAction;
    let mut host = seeded_host();
    host.dispatch_layer_context_action(
        LayerContextAction::DetachInstance,
        LayerContextTarget::Layer(NodeId::new("inst1")),
    );
    assert!(
        op_editor_core::walkers::find_node(
            host.editor_state().active_children(),
            &NodeId::new("inst1")
        )
        .is_none(),
        "context-menu detach replaced the Ref"
    );
}

#[test]
fn context_menu_detach_component_sheds_reusable_flag() {
    use op_editor_core::ui_draft::LayerContextTarget;
    use op_editor_ui::widgets::layer_context_menu::LayerContextAction;
    let mut host = seeded_host();
    host.dispatch_layer_context_action(
        LayerContextAction::DetachComponent,
        LayerContextTarget::Layer(NodeId::new("card")),
    );
    match op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("card"),
    ) {
        Some(PenNode::Frame(f)) => assert_eq!(f.reusable, None, "reusable flag shed"),
        other => panic!("card must stay a Frame, got {other:?}"),
    }
}

#[test]
fn remote_icon_insert_bakes_svg_d_as_path_node() {
    let mut host = seeded_host();
    let state = host.editor_state_mut();
    state.editor_ui.icon_picker.open = true;
    state.editor_ui.icon_picker_replace_selection = false;
    state.editor_ui.icon_picker_panel_pos = Some((0.0, 0.0));
    state.editor_ui.icon_picker_search = "zwxq".to_string();
    state.editor_ui.icon_picker_remote = op_editor_core::IconPickerRemoteState {
        query: "zwxq".to_string(),
        icons: vec![op_editor_core::IconPickerRemoteIcon {
            collection: "mdi".to_string(),
            name: "zwxq-home".to_string(),
            width: 24.0,
            height: 24.0,
            style: "fill".to_string(),
            d: "M3 9l9-7 9 7v11h-6v-7H9v7H3z".to_string(),
        }],
        loading: false,
        next_start: 1,
        total: 1,
        ..Default::default()
    };

    // Find the remote row's hit point through the panel's own
    // hit-test so the test can't drift from the row math.
    let panel_rect = host
        .icon_picker_panel_rect(1200.0, 800.0)
        .expect("picker open");
    let panel = op_editor_ui::widgets::IconPickerPanel::for_editor(host.editor_state())
        .expect("panel builds");
    let mut hit_point = None;
    let mut y = panel_rect.origin.y + 2.0;
    while y < panel_rect.origin.y + panel_rect.size.y {
        if let Some(op_editor_ui::widgets::IconPickerHit::SelectIcon { collection, name }) = panel
            .hit_test(
                panel_rect,
                op_editor_ui::Point2D::new(panel_rect.origin.x + 4.0, y),
            )
        {
            if collection == "mdi" && name == "zwxq-home" {
                hit_point = Some((panel_rect.origin.x + 4.0, y));
                break;
            }
        }
        y += 2.0;
    }
    let (x, y) = hit_point.expect("remote icon row is hit-testable");
    let before = host.editor_state().active_children().len();
    assert!(host.dispatch_icon_picker_press(x, y, 1200.0, 800.0));

    let children = host.editor_state().active_children();
    assert_eq!(children.len(), before + 1, "insert landed");
    // The icon inserts ABOVE the selected node (`inst1`), so it is no
    // longer the last child — locate the baked path by its icon id.
    let path_idx = children
        .iter()
        .position(
            |n| matches!(n, PenNode::Path(p) if p.icon_id.as_deref() == Some("mdi:zwxq-home")),
        )
        .expect("baked remote icon path was inserted");
    let inst1_idx = children
        .iter()
        .position(|n| n.id_str() == "inst1")
        .expect("selection still present");
    assert!(
        path_idx < inst1_idx,
        "the inserted icon sits above the selected node"
    );
    let PenNode::Path(p) = &children[path_idx] else {
        unreachable!("path_idx points at the matched Path");
    };
    assert_eq!(
        p.d.as_deref(),
        Some("M3 9l9-7 9 7v11h-6v-7H9v7H3z"),
        "the remote icon's d is baked (no fallback dot)"
    );
    assert_eq!(p.icon_id.as_deref(), Some("mdi:zwxq-home"));
}

const STROKE_RECT_DOC: &str = r##"{
  "version":"0.8.0",
  "children":[
    {"type":"rectangle","id":"r1","name":"Box","x":0,"y":0,"width":100,"height":50}
  ]
}"##;

fn seeded_rect_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(STROKE_RECT_DOC)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("r1"));
    host
}

/// Repro for the "stroke width reverts to 0 on blur" bug: a shape
/// with NO stroke, focus on the inline `StrokeWidth` field, a typed
/// draft of "1", committed through the host's blur path. The width
/// must persist (cmd_set_node_stroke_width attaches a fresh stroke).
#[test]
fn host_commit_creates_stroke_on_a_bare_shape() {
    let mut host = seeded_rect_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::StrokeWidth);
    host.editor_state_mut().ui.property_input.set_text("1");
    host.commit_property_focus_if_any();
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("r1"),
    )
    .expect("rect present after commit");
    assert_eq!(
        op_editor_core::fills::node_stroke_width(node),
        Some(1.0),
        "stroke width must persist after the host blur commit"
    );
}

/// End-to-end repro of the user-reported bug: a bare shape, the user
/// clicks the inline stroke-width input, types a value, then blurs —
/// the width must persist. Scans the live panel for the hit-testable
/// StrokeWidth rect so it catches a paint/hit-test divergence.
#[test]
fn host_click_type_blur_persists_stroke_width() {
    let mut host = seeded_rect_host();
    let (vw, vh) = (1200.0_f32, 800.0_f32);
    let rect = host.property_rect(vw, vh);
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("property panel for the selected rect");
    // Find the point that hit-tests to the inline StrokeWidth input.
    let mut hit: Option<op_editor_ui::Point2D> = None;
    let mut y = rect.origin.y + 2.0;
    'scan: while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x + 2.0;
        while x < rect.origin.x + rect.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if panel.hit_test(rect, p) == Some(op_editor_core::PropertyFocus::StrokeWidth) {
                hit = Some(p);
                break 'scan;
            }
            x += 3.0;
        }
        y += 3.0;
    }
    let p = hit.expect("inline StrokeWidth input must be hit-testable in Single mode");

    assert!(host.apply_press(p.x, p.y, vw, vh));
    assert_eq!(
        host.editor_state().ui.property_focus,
        Some(op_editor_core::PropertyFocus::StrokeWidth),
        "clicking the inline width must focus StrokeWidth"
    );
    let seeded = host.editor_state().ui.property_input.text().to_owned();
    host.apply_text('5');
    let typed = host.editor_state().ui.property_input.text().to_owned();
    host.commit_property_focus_if_any();
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("r1"),
    )
    .expect("rect present");
    let w = op_editor_core::fills::node_stroke_width(node);
    assert!(
        w.is_some() && w.unwrap() > 0.0,
        "stroke width must persist after click→type→blur; seed={seeded:?} typed={typed:?} got={w:?}"
    );
}

/// The actual user-visible bug: `cmd_set_node_stroke_width` attaches a
/// stroke with `fill: None` (width, no color). The panel snapshot used
/// to gate `stroke` on a parseable solid color, so a colorless stroke's
/// width vanished from the panel and the input read back "0" on blur —
/// even though the model kept the width. The snapshot must surface the
/// width regardless of whether a solid color is set.
#[test]
fn colorless_stroke_width_survives_in_the_panel_snapshot() {
    let mut host = seeded_rect_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::StrokeWidth);
    host.editor_state_mut().ui.property_input.set_text("3");
    host.commit_property_focus_if_any();

    // The model kept the width (no color attached).
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("r1"),
    )
    .expect("rect present");
    assert_eq!(op_editor_core::fills::node_stroke_width(node), Some(3.0));

    // ...and the panel snapshot must reflect it (the display bug).
    let panel =
        op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state()).expect("panel");
    assert_eq!(
        panel.snapshot.stroke_side_widths(),
        [3.0, 3.0, 3.0, 3.0],
        "a colorless stroke's width must still show in the panel"
    );
}

/// Clicking the stroke-width input on a node with NO stroke must seed
/// "0" (the displayed value), not auto-fill "1" — otherwise the value
/// silently jumps on focus. Mirrors the seed-matches-paint invariant.
#[test]
fn stroke_width_seed_matches_display_for_no_stroke() {
    let host = seeded_rect_host();
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("panel for the selected rect");
    let seed = super::press_helpers::property_focus_initial(
        op_editor_core::PropertyFocus::StrokeWidth,
        &panel,
    );
    assert_eq!(
        seed, "0",
        "clicking the width on a no-stroke node must seed the displayed 0, not 1"
    );
}

/// And once a stroke exists, the seed echoes its real (un-rounded)
/// width — never the old `round() as i32` that snapped 2.5 → "3".
#[test]
fn stroke_width_seed_echoes_existing_width() {
    let mut host = seeded_rect_host();
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::StrokeWidth);
    host.editor_state_mut().ui.property_input.set_text("2.5");
    host.commit_property_focus_if_any();
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("panel after commit");
    let seed = super::press_helpers::property_focus_initial(
        op_editor_core::PropertyFocus::StrokeWidth,
        &panel,
    );
    // `format_panel_number` renders fractionals at 2 decimals — the same
    // string the inline input paints. The old `round() as i32` would have
    // snapped this to "3".
    assert_eq!(
        seed, "2.50",
        "the width seed must echo the real width un-rounded"
    );
}

fn scene_fill(host: &mut WidgetHostNative, id: &str) -> Option<op_editor_ui::Color> {
    host.layout_scene()
        .active_page()
        .and_then(|p| p.find(id))
        .and_then(|n| n.fill)
}

/// RemoveFill on a plain filled frame must clear the rendered fill, not
/// just the panel row — the canvas paints the cached scene node's fill.
#[test]
fn remove_fill_clears_scene_fill_on_plain_frame() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"f1","name":"Box","x":0,"y":0,"width":100,"height":50,
           "fill":[{"type":"solid","color":"#ff0000"}]}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));
    host.mark_paint_dirty_for_test();

    assert!(scene_fill(&mut host, "f1").is_some(), "scene starts red");
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::RemoveFill(0));

    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("f1"),
    )
    .unwrap();
    assert!(
        op_editor_core::fills::node_fills(node)
            .map(|f| f.is_empty())
            .unwrap_or(true),
        "doc fill must be empty after RemoveFill"
    );
    assert!(
        scene_fill(&mut host, "f1").is_none(),
        "scene fill must clear after RemoveFill (stale-scene bug if Some)"
    );
}

/// Same, for a child node nested under a frame (the realistic case in
/// the bug report — a search bar inside a screen frame).
#[test]
fn remove_fill_clears_scene_fill_on_nested_child() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"screen","name":"Screen","x":0,"y":0,"width":200,"height":200,
           "children":[
             {"type":"rectangle","id":"bar","name":"Bar","x":10,"y":10,"width":120,"height":40,
              "fill":[{"type":"solid","color":"#ffcccc"}]}
           ]}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("bar"));
    host.mark_paint_dirty_for_test();

    assert!(
        scene_fill(&mut host, "bar").is_some(),
        "child scene starts pink"
    );
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::RemoveFill(0));
    assert!(
        scene_fill(&mut host, "bar").is_none(),
        "child scene fill must clear after RemoveFill"
    );
}

/// Multi-fill: removing every fill row must leave the scene unpainted.
#[test]
fn remove_all_multi_fills_clears_scene_fill() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"f1","name":"Box","x":0,"y":0,"width":100,"height":50,
           "fill":[{"type":"solid","color":"#ff0000"},{"type":"solid","color":"#00ff00"}]}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));
    host.mark_paint_dirty_for_test();

    assert!(scene_fill(&mut host, "f1").is_some());
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::RemoveFill(0));
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::RemoveFill(0));
    assert!(
        scene_fill(&mut host, "f1").is_none(),
        "scene fill must clear after removing every fill row"
    );
}

/// End-to-end repro of the reported bug: a fill bound to a colour
/// variable (`$brand`, as token-based old .op designs use). The scene
/// resolves the colour via `fill_for` (fill ref wins over container.fill),
/// so removing the fill must ALSO drop the `fill_refs` binding or the
/// colour keeps painting. Guards the full chain end-to-end.
#[test]
fn remove_variable_bound_fill_clears_scene_color() {
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"r1","name":"Box","x":0,"y":0,"width":50,"height":50,
           "fill":[{"type":"solid","color":"$brand"}]}
        ]}"##,
    )
    .expect("fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    let st = host.editor_state_mut();
    st.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    st.set_single_selection(NodeId::new("r1"));
    // Mirror the editor's fill→variable binding (what the colour picker
    // and the load-time scan register).
    st.ui
        .variables
        .fill_refs
        .insert(NodeId::new("r1"), "brand".to_string());
    host.mark_paint_dirty_for_test();

    assert!(
        scene_fill(&mut host, "r1").is_some(),
        "the variable-bound fill must render before removal"
    );
    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::RemoveFill(0));
    assert!(
        scene_fill(&mut host, "r1").is_none(),
        "removing the fill must clear the variable colour from the scene"
    );
}
