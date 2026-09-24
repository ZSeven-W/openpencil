//! Side-by-side directions on the native host: the Home toggle that asks
//! for them, the send route it pins, and the workspace's "use this" pick.

use super::WidgetHostNative;
use op_editor_core::{
    HomeHit, LaunchRoute, PenNodeExt, WorkspaceHit, WorkspacePhase, WorkspaceVariant, WorkspaceView,
};
use op_editor_ui::widgets::{HomeSurface, WorkspaceSurface};
use op_editor_ui::Point2D;

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn centre(rect: op_editor_ui::Rect) -> (f32, f32) {
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn the_home_toggle_turns_a_send_into_a_variants_run() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    let toggle = {
        let home = HomeSurface::for_editor(host.editor_state()).expect("home");
        let layout = home.layout(W, H);
        assert!(
            layout.variants.size.x > 0.0,
            "desktop Home offers the toggle"
        );
        assert_eq!(
            home.hit_test(
                W,
                H,
                Point2D::new(centre(layout.variants).0, centre(layout.variants).1)
            ),
            Some(HomeHit::Variants)
        );
        layout.variants
    };
    let (x, y) = centre(toggle);
    assert!(host.apply_press(x, y, W, H));
    assert!(host.editor_state().editor_ui.home.variants_on);

    for character in "记账 app".chars() {
        assert!(host.apply_text(character));
    }
    assert!(host.apply_send());
    let state = host.editor_state();
    assert_eq!(
        state.chat.launch_route,
        LaunchRoute::Variants(op_editor_core::DEFAULT_VARIANT_COUNT)
    );
    assert!(state.chat.pending_send.is_some());
    let workspace = &state.editor_ui.workspace;
    assert!(workspace.active);
    assert_eq!(
        workspace.variant_count,
        op_editor_core::DEFAULT_VARIANT_COUNT
    );
    assert_eq!(workspace.view, WorkspaceView::AllBoards);

    // Pressing it again turns it back off: the next send is one design.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    assert!(host.apply_press(x, y, W, H));
    assert!(host.apply_press(x, y, W, H));
    assert!(!host.editor_state().editor_ui.home.variants_on);
    for character in "记账 app".chars() {
        assert!(host.apply_text(character));
    }
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().chat.launch_route,
        LaunchRoute::Orchestrator
    );
    assert!(!host.editor_state().editor_ui.workspace.is_variants_run());
}

/// A workspace whose run landed three one-board directions and settled.
fn settled_variants_host() -> WidgetHostNative {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "a", "name": "方案 A · Zen · Home", "x": 0, "y": 0,
          "width": 375, "height": 812, "children": [] },
        { "type": "frame", "id": "b", "name": "方案 B · Noir · Home", "x": 615, "y": 0,
          "width": 375, "height": 812, "children": [] },
        { "type": "frame", "id": "c", "name": "方案 C · Pastel · Home", "x": 1230, "y": 0,
          "width": 375, "height": 812, "children": [] }
    ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(op_editor_core::EditorState::from_document(document));
    let editor = host.editor_state_mut();
    editor.editor_ui.open_workspace_for_generation(
        op_editor_core::HomeFamily::AppUi,
        "记账 app",
        op_editor_core::TaskDraft::default(),
        0,
        1_000,
        None,
    );
    let workspace = &mut editor.editor_ui.workspace;
    workspace.begin_variants(3);
    for (index, (id, style)) in [("a", "Zen"), ("b", "Noir"), ("c", "Pastel")]
        .into_iter()
        .enumerate()
    {
        let letter = op_editor_core::variant_letter(index);
        workspace.record_variant(WorkspaceVariant {
            index,
            name: format!("方案 {letter}"),
            style_guide: style.to_lowercase(),
            style_label: style.into(),
            name_prefix: format!("方案 {letter} · {style} · "),
            root_ids: vec![id.into()],
            variables: None,
            themes: None,
        });
    }
    host
}

#[test]
fn the_bar_waits_for_the_run_to_settle() {
    let host = settled_variants_host();
    let surface = WorkspaceSurface::for_editor(host.editor_state()).expect("workspace");
    let layout = surface.layout(W, H);
    assert!(
        surface.variant_bar(&layout).is_empty(),
        "no pick while the directions are still being generated"
    );
}

#[test]
fn use_this_keeps_one_direction_and_parks_the_others() {
    let mut host = settled_variants_host();
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    let button = {
        let surface = WorkspaceSurface::for_editor(host.editor_state()).expect("workspace");
        let layout = surface.layout(W, H);
        let bar = surface.variant_bar(&layout);
        assert_eq!(bar.len(), 3);
        assert!(bar[0].label.starts_with("方案 A"));
        // The pills sit inside the canvas, left to right in slot order.
        for pair in bar.windows(2) {
            assert!(pair[0].pill.origin.x + pair[0].pill.size.x <= pair[1].pill.origin.x);
        }
        for item in &bar {
            assert!(layout
                .canvas
                .contains(Point2D::new(item.pill.origin.x, item.pill.origin.y)));
        }
        let (x, y) = centre(bar[1].button);
        assert_eq!(
            surface.hit_test(W, H, Point2D::new(x, y)),
            Some(WorkspaceHit::UseVariant(1))
        );
        bar[1].button
    };
    let (x, y) = centre(button);
    assert!(host.apply_press(x, y, W, H));

    let state = host.editor_state();
    let names: Vec<String> = state
        .active_children()
        .iter()
        .map(|node| node.base().name.clone().unwrap_or_default())
        .collect();
    assert_eq!(names, vec!["Home".to_string()], "B stays, prefix dropped");
    let pages = state.doc.pages.as_ref().expect("the others got a page");
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[1].children.len(),
        2,
        "A and C were moved, not deleted"
    );
    assert!(state.editor_ui.workspace.variants.is_empty());
}

#[test]
fn tab_reaches_the_directions_toggle_and_every_use_this_button() {
    let mut home = WidgetHostNative::new();
    home.editor_state_mut().editor_ui.home.visible = true;
    let order = HomeSurface::for_editor(home.editor_state())
        .expect("home")
        .focus_order(W, H);
    assert!(order.contains(&HomeHit::Variants), "{order:?}");

    let mut host = settled_variants_host();
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    let surface = WorkspaceSurface::for_editor(host.editor_state()).expect("workspace");
    let layout = surface.layout(W, H);
    let order: Vec<WorkspaceHit> = surface
        .focus_order(&layout)
        .into_iter()
        .map(|(hit, _)| hit)
        .collect();
    for index in 0..3 {
        assert!(
            order.contains(&WorkspaceHit::UseVariant(index)),
            "{index}: {order:?}"
        );
    }
}
