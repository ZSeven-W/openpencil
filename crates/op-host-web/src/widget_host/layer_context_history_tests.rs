use super::WidgetHost;
use op_editor_core::editor_ui_state::LayerContextMenuState;
use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::widgets::layer_context_menu::{LayerContextAction, LayerContextMenu};
use op_editor_ui::widgets::{DropPosition, LayerPanel, LayerPanelHit};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn seed(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Header","x":0,"y":0,"width":100,"height":50},
            {"type":"rectangle","id":"n2","name":"Card","x":120,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn seed_overlapping_rects(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"n1","name":"Header","x":0,"y":0,"width":100,"height":50},
            {"type":"rectangle","id":"n2","name":"Card","x":40,"y":0,"width":100,"height":50}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn seed_pages(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[],"pages":[
            {"id":"p1","name":"One","children":[
                {"type":"rectangle","id":"r1","name":"r1","x":0,"y":0,"width":10,"height":10}]},
            {"id":"p2","name":"Two","children":[
                {"type":"rectangle","id":"r2","name":"r2","x":0,"y":0,"width":10,"height":10}]},
            {"id":"p3","name":"Three","children":[
                {"type":"rectangle","id":"r3","name":"r3","x":0,"y":0,"width":10,"height":10}]}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn seed_frame(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"Card","x":0,"y":0,
             "width":100,"height":80,"children":[]}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn point_for_layer_row(host: &WidgetHost, id: &str) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let regions = panel.regions(rect);
    let mut y = regions.layers_rows_top + 2.0;
    while y < regions.layers_rows_top + regions.layers_view_h {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if matches!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Layer(node_id)) if node_id == NodeId::new(id)
        ) {
            return point;
        }
        y += 2.0;
    }
    panic!("no layer row point found for {id}");
}

fn point_for_page_row(host: &WidgetHost, page_index: usize) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let regions = panel.regions(rect);
    let mut y = regions.pages_rows_top + 2.0;
    while y < regions.pages_rows_top + regions.pages_view_h {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if matches!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Page(idx)) if idx == page_index
        ) {
            return point;
        }
        y += 2.0;
    }
    panic!("no page row point found for index {page_index}");
}

fn point_for_context_action(
    host: &WidgetHost,
    state: LayerContextMenuState,
    action: LayerContextAction,
) -> Point2D {
    let menu = LayerContextMenu::for_state(&host.editor_state, state);
    let rect = menu.rect();
    let mut y = rect.origin.y + 2.0;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(rect.origin.x + 24.0, y);
        if menu.hit_test(point) == Some(action) {
            return point;
        }
        y += 2.0;
    }
    panic!("no context menu point found for {action:?}");
}

fn node_flag(host: &WidgetHost, id: &str) -> (Option<bool>, Option<bool>) {
    let node =
        op_editor_core::walkers::find_node(host.editor_state.active_children(), &NodeId::new(id))
            .expect("node present");
    (node.base().visible, node.base().locked)
}

fn page_names(host: &WidgetHost) -> Vec<String> {
    host.editor_state
        .doc
        .pages
        .as_ref()
        .map(|pages| pages.iter().map(|page| page.name.clone()).collect())
        .unwrap_or_default()
}

fn history_len(host: &WidgetHost) -> usize {
    host.editor_state.history.past.len()
}

fn active_order(host: &WidgetHost) -> Vec<String> {
    host.editor_state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

fn click_layer_context_action_for(host: &mut WidgetHost, id: &str, action: LayerContextAction) {
    let row = point_for_layer_row(host, id);
    assert!(host.apply_right_press(row.x, row.y, VIEWPORT_W, VIEWPORT_H));
    let state = host
        .editor_state
        .editor_ui
        .layer_context_menu
        .clone()
        .expect("right press opens layer context menu");
    assert_eq!(state.target, LayerContextTarget::Layer(NodeId::new(id)));

    let action_point = point_for_context_action(host, state, action);
    assert!(host.apply_press(action_point.x, action_point.y, VIEWPORT_W, VIEWPORT_H,));
}

fn click_layer_context_action(host: &mut WidgetHost, action: LayerContextAction) {
    click_layer_context_action_for(host, "n1", action);
}

fn click_page_context_action(host: &mut WidgetHost, page_index: usize, action: LayerContextAction) {
    let row = point_for_page_row(host, page_index);
    assert!(host.apply_right_press(row.x, row.y, VIEWPORT_W, VIEWPORT_H));
    let state = host
        .editor_state
        .editor_ui
        .layer_context_menu
        .clone()
        .expect("right press opens page context menu");
    assert_eq!(state.target, LayerContextTarget::Page(page_index));

    let action_point = point_for_context_action(host, state, action);
    assert!(host.apply_press(action_point.x, action_point.y, VIEWPORT_W, VIEWPORT_H,));
}

fn point_for_layer_toggle(host: &WidgetHost, id: &str, hidden: bool) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            match panel.hit_test(rect, point) {
                Some(LayerPanelHit::ToggleHidden(node_id))
                    if hidden && node_id == NodeId::new(id) =>
                {
                    return point;
                }
                Some(LayerPanelHit::ToggleLocked(node_id))
                    if !hidden && node_id == NodeId::new(id) =>
                {
                    return point;
                }
                _ => {}
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no layer toggle point found for {id}");
}

fn point_for_add_page(host: &WidgetHost) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if matches!(panel.hit_test(rect, point), Some(LayerPanelHit::AddPage)) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no add-page point found");
}

fn point_for_layer_drop_after(host: &WidgetHost, source: &str, anchor: &str) -> Point2D {
    let panel = LayerPanel::from_editor_with_drag_source(&host.editor_state, &NodeId::new(source));
    let rect = host.layer_panel_rect(VIEWPORT_H);
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(rect.origin.x + 40.0, y);
        if let Some(drop) = panel.drop_target_at(rect, point) {
            if drop.anchor == NodeId::new(anchor) && matches!(drop.position, DropPosition::After) {
                return point;
            }
        }
        y += 1.0;
    }
    panic!("no after-drop target found for {source} after {anchor}");
}

#[test]
fn layer_context_toggle_visibility_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    click_layer_context_action(&mut host, LayerContextAction::ToggleVisibility);

    assert_eq!(node_flag(&host, "n1").0, Some(false), "hidden");
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_ne!(
        node_flag(&host, "n1").0,
        Some(false),
        "undo restores visible"
    );
}

#[test]
fn layer_panel_eye_and_lock_clicks_are_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);
    host.editor_state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));

    let eye = point_for_layer_toggle(&host, "n1", true);
    let before = history_len(&host);
    assert!(host.apply_click(eye.x, eye.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(node_flag(&host, "n1").0, Some(false), "hidden via eye");
    assert_eq!(history_len(&host), before + 1, "eye pushes history");
    assert!(host.editor_state.undo());
    assert_ne!(node_flag(&host, "n1").0, Some(false));

    let lock = point_for_layer_toggle(&host, "n1", false);
    let before = history_len(&host);
    assert!(host.apply_click(lock.x, lock.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(node_flag(&host, "n1").1, Some(true), "locked");
    assert_eq!(history_len(&host), before + 1, "lock pushes history");
    assert!(host.editor_state.undo());
    assert_ne!(node_flag(&host, "n1").1, Some(true));
}

#[test]
fn layer_panel_add_page_click_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_pages(&mut host);

    let add = point_for_add_page(&host);
    let before = history_len(&host);
    assert!(host.apply_click(add.x, add.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(page_names(&host).len(), 4, "page added");
    assert_eq!(history_len(&host), before + 1, "add-page pushes history");
    assert!(host.editor_state.undo());
    assert_eq!(page_names(&host).len(), 3, "undo removes the page");
}

#[test]
fn layer_panel_drag_reorder_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    let drop = point_for_layer_drop_after(&host, "n1", "n2");
    let drag = crate::widget_host::LayerDragState {
        source: NodeId::new("n1"),
        start_y: drop.y - 60.0,
        current_x: drop.x,
        current_y: drop.y,
        active: true,
    };
    let before = history_len(&host);
    assert!(host.commit_layer_drag(drag, VIEWPORT_H));
    assert_eq!(
        active_order(&host),
        vec!["n2".to_string(), "n1".to_string()],
        "reordered"
    );
    assert_eq!(history_len(&host), before + 1, "drag pushes history");
    assert!(host.editor_state.undo());
    assert_eq!(
        active_order(&host),
        vec!["n1".to_string(), "n2".to_string()],
        "undo restores order"
    );
}

#[test]
fn layer_context_toggle_lock_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    click_layer_context_action(&mut host, LayerContextAction::ToggleLock);

    assert_eq!(node_flag(&host, "n1").1, Some(true), "locked");
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_ne!(node_flag(&host, "n1").1, Some(true), "undo unlocks");
}

#[test]
fn layer_context_group_selection_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);
    host.editor_state.set_single_selection(NodeId::new("n1"));
    host.editor_state.toggle_selection(NodeId::new("n2"));

    click_layer_context_action(&mut host, LayerContextAction::GroupSelection);

    assert_eq!(
        host.editor_state.active_children().len(),
        1,
        "selected layers are wrapped into one group"
    );
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_eq!(
        host.editor_state.active_children().len(),
        2,
        "undo restores both layers"
    );
}

#[test]
fn layer_context_create_and_detach_component_are_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_frame(&mut host);

    click_layer_context_action_for(&mut host, "f1", LayerContextAction::CreateComponent);

    assert_eq!(host.editor_state.components.len(), 1, "registered");

    click_layer_context_action_for(&mut host, "f1", LayerContextAction::DetachComponent);

    assert_eq!(host.editor_state.components.len(), 0, "deregistered");
    assert!(
        host.editor_state.undo(),
        "detach should push one history entry"
    );
    assert_eq!(
        host.editor_state.components.len(),
        1,
        "undo restores the registration"
    );
    assert!(
        host.editor_state.undo(),
        "create should push one history entry"
    );
    assert_eq!(
        host.editor_state.components.len(),
        0,
        "second undo reverts the create"
    );
}

#[test]
fn layer_context_boolean_union_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_overlapping_rects(&mut host);
    host.editor_state.set_single_selection(NodeId::new("n1"));
    host.editor_state.toggle_selection(NodeId::new("n2"));

    click_layer_context_action(&mut host, LayerContextAction::BooleanUnion);

    let children = host.editor_state.active_children();
    assert_eq!(children.len(), 1, "operands are replaced by the result");
    assert!(
        matches!(children[0], jian_ops_schema::node::PenNode::Path(_)),
        "union result is a Path node"
    );
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_eq!(
        host.editor_state.active_children().len(),
        2,
        "undo restores both operands"
    );
}

#[test]
fn page_context_duplicate_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_pages(&mut host);
    let before = page_names(&host);

    click_page_context_action(&mut host, 0, LayerContextAction::DuplicatePage);

    assert_ne!(page_names(&host), before, "page duplicated");
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_eq!(page_names(&host), before, "undo restores pages");
}

#[test]
fn page_context_move_down_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_pages(&mut host);
    let before = page_names(&host);

    click_page_context_action(&mut host, 0, LayerContextAction::MovePageDown);

    assert_eq!(page_names(&host), vec!["Two", "One", "Three"]);
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_eq!(page_names(&host), before, "undo restores page order");
}

#[test]
fn page_context_delete_is_undoable_like_native() {
    let mut host = WidgetHost::new();
    seed_pages(&mut host);
    let before = page_names(&host);

    click_page_context_action(&mut host, 1, LayerContextAction::DeletePage);

    assert_eq!(page_names(&host), vec!["One", "Three"]);
    assert!(
        host.editor_state.undo(),
        "context action should push history"
    );
    assert_eq!(page_names(&host), before, "undo restores deleted page");
}
