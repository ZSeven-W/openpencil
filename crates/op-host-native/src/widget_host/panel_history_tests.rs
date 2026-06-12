//! Undo/history coverage of layer-panel + page operations (#13).
//!
//! TS records EVERY one of these (`document-store-node-actions.ts`
//! `mutateWithHistory` for moveNode / toggleVisibility / toggleLock;
//! `document-store-pages.ts` explicit `pushState` for page CRUD).
//! Each test mutates through the real host dispatch path and asserts
//! a single history entry whose undo restores the prior state.

use super::WidgetHostNative;
use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::widgets::layer_context_menu::LayerContextAction as A;
use op_editor_ui::widgets::{LayerPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

const VW: f32 = 1440.0;
const VH: f32 = 900.0;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

/// Two top-level rects on a single implicit page.
fn two_rects_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"a","name":"a","x":0,"y":0,"width":40,"height":40},
          {"type":"rectangle","id":"b","name":"b","x":60,"y":0,"width":40,"height":40}
        ]}"#,
    );
    host
}

/// Three named pages with one rect each.
fn three_pages_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[],"pages":[
          {"id":"p1","name":"One","children":[
            {"type":"rectangle","id":"r1","name":"r1","x":0,"y":0,"width":10,"height":10}]},
          {"id":"p2","name":"Two","children":[
            {"type":"rectangle","id":"r2","name":"r2","x":0,"y":0,"width":10,"height":10}]},
          {"id":"p3","name":"Three","children":[
            {"type":"rectangle","id":"r3","name":"r3","x":0,"y":0,"width":10,"height":10}]}
        ]}"#,
    );
    host
}

fn page_names(host: &WidgetHostNative) -> Vec<String> {
    host.editor_state()
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default()
}

fn history_len(host: &WidgetHostNative) -> usize {
    host.editor_state().history.past.len()
}

fn node_flag(host: &WidgetHostNative, id: &str) -> (Option<bool>, Option<bool>) {
    let n =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &NodeId::new(id))
            .expect("node present");
    (n.base().visible, n.base().locked)
}

// --- Context-menu lock / hide --------------------------------------

#[test]
fn context_menu_toggle_visibility_is_undoable() {
    let mut host = two_rects_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(
        A::ToggleVisibility,
        LayerContextTarget::Layer(NodeId::new("a")),
    );
    assert_eq!(node_flag(&host, "a").0, Some(false), "hidden");
    assert_eq!(history_len(&host), before + 1, "one history entry");
    assert!(host.editor_state_mut().undo());
    assert_ne!(
        node_flag(&host, "a").0,
        Some(false),
        "undo restores visible"
    );
}

#[test]
fn context_menu_toggle_lock_is_undoable() {
    let mut host = two_rects_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::ToggleLock, LayerContextTarget::Layer(NodeId::new("a")));
    assert_eq!(node_flag(&host, "a").1, Some(true), "locked");
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_ne!(node_flag(&host, "a").1, Some(true), "undo unlocks");
}

// --- Layer-panel eye / lock toggles (apply_click path) -------------

#[test]
fn eye_and_lock_toggles_via_click_are_undoable() {
    let mut host = two_rects_host();
    // The eye / lock affordances are hover-revealed — set the hover
    // and scan the panel for the hit the click path will resolve.
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("a"));
    host.mark_paint_dirty_for_test();
    let layer_rect = Rect {
        origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
        size: Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            VH - TOP_BAR_HEIGHT,
        ),
    };
    let find = |host: &WidgetHostNative, want_eye: bool| -> Option<(f32, f32)> {
        let panel = LayerPanel::from_editor(host.editor_state());
        let mut y = layer_rect.origin.y;
        while y < layer_rect.origin.y + layer_rect.size.y {
            let mut x = layer_rect.origin.x;
            while x < layer_rect.origin.x + layer_rect.size.x {
                match panel.hit_test(layer_rect, Point2D::new(x, y)) {
                    Some(op_editor_ui::widgets::LayerPanelHit::ToggleHidden(id))
                        if want_eye && id.as_str() == "a" =>
                    {
                        return Some((x, y))
                    }
                    Some(op_editor_ui::widgets::LayerPanelHit::ToggleLocked(id))
                        if !want_eye && id.as_str() == "a" =>
                    {
                        return Some((x, y))
                    }
                    _ => {}
                }
                x += 2.0;
            }
            y += 2.0;
        }
        None
    };
    let (ex, ey) = find(&host, true).expect("eye toggle found");
    let before = history_len(&host);
    assert!(host.apply_click(ex, ey, VW, VH));
    assert_eq!(node_flag(&host, "a").0, Some(false), "hidden via eye");
    assert_eq!(history_len(&host), before + 1, "eye toggle pushes history");
    assert!(host.editor_state_mut().undo());
    assert_ne!(node_flag(&host, "a").0, Some(false));

    let (lx, ly) = find(&host, false).expect("lock toggle found");
    let before = history_len(&host);
    assert!(host.apply_click(lx, ly, VW, VH));
    assert_eq!(node_flag(&host, "a").1, Some(true), "locked via padlock");
    assert_eq!(history_len(&host), before + 1, "lock toggle pushes history");
    assert!(host.editor_state_mut().undo());
    assert_ne!(node_flag(&host, "a").1, Some(true));
}

// --- Layer drag reorder / reparent ---------------------------------

#[test]
fn layer_drag_reorder_is_undoable() {
    let mut host = two_rects_host();
    host.mark_paint_dirty_for_test();
    host.refresh_layout_scene();
    let layer_rect = Rect {
        origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
        size: Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            VH - TOP_BAR_HEIGHT,
        ),
    };
    // Scan for a cursor point whose drop target moves "a" after "b".
    let panel = LayerPanel::from_editor_with_drag_source(host.editor_state(), &NodeId::new("a"));
    let mut drop_at: Option<(f32, f32)> = None;
    let mut y = layer_rect.origin.y;
    'outer: while y < layer_rect.origin.y + layer_rect.size.y {
        let p = Point2D::new(layer_rect.origin.x + 40.0, y);
        if let Some(t) = panel.drop_target_at(layer_rect, p) {
            if t.anchor.as_str() == "b"
                && matches!(t.position, op_editor_ui::widgets::DropPosition::After)
            {
                drop_at = Some((p.x, p.y));
                break 'outer;
            }
        }
        y += 1.0;
    }
    let (dx, dy) = drop_at.expect("after-b drop target found");
    let before = history_len(&host);
    let drag = crate::widget_host::LayerDragState {
        source: NodeId::new("a"),
        start_y: dy - 60.0,
        current_x: dx,
        current_y: dy,
        active: true,
    };
    assert!(host.commit_layer_drag(drag, VH));
    let order: Vec<String> = host
        .editor_state()
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    assert_eq!(order, vec!["b".to_string(), "a".to_string()], "reordered");
    assert_eq!(history_len(&host), before + 1, "drag commit pushes history");
    assert!(host.editor_state_mut().undo());
    let order: Vec<String> = host
        .editor_state()
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    assert_eq!(
        order,
        vec!["a".to_string(), "b".to_string()],
        "undo restores order"
    );
}

// --- Page CRUD ------------------------------------------------------

#[test]
fn add_page_via_click_is_undoable() {
    let mut host = three_pages_host();
    host.mark_paint_dirty_for_test();
    // The `+` affordance sits top-right of the Pages header:
    // plus_x = panel_w - ROW_PAD_X(12) - 12, a 14 px box.
    let panel_w = host.editor_state().editor_ui.layer_panel_width;
    let plus_x = panel_w - 12.0 - 12.0 + 7.0;
    let plus_y = TOP_BAR_HEIGHT + 8.0 + 7.0;
    let before = history_len(&host);
    assert!(host.apply_click(plus_x, plus_y, VW, VH));
    assert_eq!(page_names(&host).len(), 4, "page added");
    assert_eq!(history_len(&host), before + 1, "add-page pushes history");
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host).len(), 3, "undo removes the page");
}

#[test]
fn context_menu_delete_page_is_undoable() {
    let mut host = three_pages_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::DeletePage, LayerContextTarget::Page(1));
    assert_eq!(page_names(&host), vec!["One", "Three"]);
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"]);
}

#[test]
fn delete_last_page_guard_pushes_no_history() {
    // TS removePage early-returns before pushState on the last page
    // (document-store-pages.ts:54) — the guard must not pollute undo.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[],"pages":[
          {"id":"p1","name":"Only","children":[]}
        ]}"#,
    );
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::DeletePage, LayerContextTarget::Page(0));
    assert_eq!(page_names(&host), vec!["Only"], "guard keeps the page");
    assert_eq!(history_len(&host), before, "rejected delete pushes nothing");
}

#[test]
fn context_menu_duplicate_page_is_undoable() {
    let mut host = three_pages_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::DuplicatePage, LayerContextTarget::Page(0));
    assert_eq!(page_names(&host), vec!["One", "One copy", "Two", "Three"]);
    assert_eq!(
        host.editor_state().ui.active_page_index,
        1,
        "duplicate switches to the clone"
    );
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"]);
    assert_eq!(
        host.editor_state().ui.active_page_index,
        0,
        "undo restores the active page"
    );
}

#[test]
fn context_menu_move_page_up_and_down_are_undoable() {
    let mut host = three_pages_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::MovePageDown, LayerContextTarget::Page(0));
    assert_eq!(page_names(&host), vec!["Two", "One", "Three"]);
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"]);

    let before = history_len(&host);
    host.dispatch_layer_context_action(A::MovePageUp, LayerContextTarget::Page(2));
    assert_eq!(page_names(&host), vec!["One", "Three", "Two"]);
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"]);
}

#[test]
fn move_page_up_at_top_pushes_no_history() {
    let mut host = three_pages_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::MovePageUp, LayerContextTarget::Page(0));
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"], "no-op");
    assert_eq!(history_len(&host), before, "rejected move pushes nothing");
}

// --- Context-menu duplicate / delete / group ------------------------

#[test]
fn context_menu_duplicate_layer_is_undoable() {
    // TS duplicateNode runs through mutateWithHistory
    // (document-store-node-actions.ts:190).
    let mut host = two_rects_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::Duplicate, LayerContextTarget::Layer(NodeId::new("a")));
    assert_eq!(
        host.editor_state().active_children().len(),
        3,
        "clone added"
    );
    assert_eq!(history_len(&host), before + 1, "one history entry");
    assert!(host.editor_state_mut().undo());
    assert_eq!(host.editor_state().active_children().len(), 2);
}

#[test]
fn context_menu_delete_layer_is_undoable() {
    // TS removeNode runs through mutateWithHistory
    // (document-store-node-actions.ts:95).
    let mut host = two_rects_host();
    let before = history_len(&host);
    host.dispatch_layer_context_action(A::Delete, LayerContextTarget::Layer(NodeId::new("a")));
    assert_eq!(host.editor_state().active_children().len(), 1, "removed");
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(host.editor_state().active_children().len(), 2);
}

#[test]
fn context_menu_group_selection_is_undoable() {
    // TS groupNodes runs through mutateWithHistory
    // (document-store-node-actions.ts:244-305).
    let mut host = two_rects_host();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("a"));
    host.editor_state_mut().toggle_selection(NodeId::new("b"));
    let before = history_len(&host);
    host.dispatch_layer_context_action(
        A::GroupSelection,
        LayerContextTarget::Layer(NodeId::new("a")),
    );
    assert_eq!(
        host.editor_state().active_children().len(),
        1,
        "two rects wrapped into one group"
    );
    assert_eq!(history_len(&host), before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(host.editor_state().active_children().len(), 2);
}

#[test]
fn context_menu_boolean_union_is_undoable() {
    // TS boolean rows push history explicitly before swapping the
    // operands for the result node (`layer-panel.tsx:389-407`).
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"a","name":"a","x":0,"y":0,"width":40,"height":40},
          {"type":"rectangle","id":"b","name":"b","x":20,"y":0,"width":40,"height":40}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("a"));
    host.editor_state_mut().toggle_selection(NodeId::new("b"));
    let before = history_len(&host);
    host.dispatch_layer_context_action(
        A::BooleanUnion,
        LayerContextTarget::Layer(NodeId::new("a")),
    );
    let children = host.editor_state().active_children();
    assert_eq!(children.len(), 1, "operands replaced by the result");
    assert!(
        matches!(children[0], jian_ops_schema::node::PenNode::Path(_)),
        "union result is a Path node"
    );
    assert_eq!(history_len(&host), before + 1, "one history entry");
    assert!(host.editor_state_mut().undo());
    assert_eq!(
        host.editor_state().active_children().len(),
        2,
        "undo restores both operands"
    );
}

// --- Context-menu create / detach component -------------------------

#[test]
fn context_menu_create_and_detach_component_are_undoable() {
    // TS makeReusable / detachComponent run through mutateWithHistory
    // (document-store-component-actions.ts).
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"f1","name":"Card","x":0,"y":0,
           "width":100,"height":80,"children":[]}
        ]}"#,
    );
    let before = history_len(&host);
    host.dispatch_layer_context_action(
        A::CreateComponent,
        LayerContextTarget::Layer(NodeId::new("f1")),
    );
    assert_eq!(host.editor_state().components.len(), 1, "registered");
    assert_eq!(history_len(&host), before + 1, "create pushes one entry");

    let before = history_len(&host);
    host.dispatch_layer_context_action(
        A::DetachComponent,
        LayerContextTarget::Layer(NodeId::new("f1")),
    );
    assert_eq!(host.editor_state().components.len(), 0, "deregistered");
    assert_eq!(history_len(&host), before + 1, "detach pushes one entry");
    assert!(host.editor_state_mut().undo());
    assert_eq!(
        host.editor_state().components.len(),
        1,
        "undo restores the registration"
    );
    assert!(host.editor_state_mut().undo());
    assert_eq!(
        host.editor_state().components.len(),
        0,
        "second undo reverts the create"
    );
}

// --- Rename (draft commit path) --------------------------------------

#[test]
fn layer_rename_commit_is_undoable() {
    // TS layer rename routes through `updateNode` →
    // `mutateWithHistory` (`layer-panel.tsx:272-276`). The
    // context-menu row only STARTS the draft; history lands on
    // commit, and only when the name actually changed.
    let mut host = two_rects_host();
    host.dispatch_layer_context_action(A::RenameLayer, LayerContextTarget::Layer(NodeId::new("a")));
    assert!(host.editor_state().ui.layer_rename.is_some(), "draft open");
    let before = history_len(&host);
    assert!(host.editor_state_mut().rename_append("!"));
    assert!(host.editor_state_mut().rename_commit());
    let name = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("a"),
    )
    .and_then(|n| n.base().name.clone());
    assert_eq!(name.as_deref(), Some("a!"));
    assert_eq!(history_len(&host), before + 1, "rename pushes one entry");
    assert!(host.editor_state_mut().undo());
    let name = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("a"),
    )
    .and_then(|n| n.base().name.clone());
    assert_eq!(name.as_deref(), Some("a"), "undo restores the name");
}

#[test]
fn page_rename_commit_is_undoable() {
    // TS renamePage pushes history (document-store-pages.ts:65-77).
    // The context-menu row only STARTS the draft; the history entry
    // lands on commit, and only when the name actually changed.
    let mut host = three_pages_host();
    host.dispatch_layer_context_action(A::RenamePage, LayerContextTarget::Page(1));
    assert!(host.editor_state().ui.layer_rename.is_some(), "draft open");
    let before = history_len(&host);
    assert!(host.editor_state_mut().rename_append("!"));
    assert!(host.editor_state_mut().rename_commit());
    assert_eq!(page_names(&host), vec!["One", "Two!", "Three"]);
    assert_eq!(history_len(&host), before + 1, "rename pushes one entry");
    assert!(host.editor_state_mut().undo());
    assert_eq!(page_names(&host), vec!["One", "Two", "Three"]);
}
