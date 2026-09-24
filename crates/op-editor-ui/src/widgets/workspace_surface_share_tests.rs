//! The workspace's share affordances: the header 分享 button and the
//! shared document's Make-one-like-this banner.

use super::*;
use op_editor_core::{HomeDevice, InfoKind, ShareRecipe, SlideRatio};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn shared_state(can_share: bool) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.deck_html_export_supported = can_share;
    state.editor_ui.home.recipe = Some(ShareRecipe {
        brief: "brief".into(),
        family: HomeFamily::KnowledgeCards,
        device: HomeDevice::Mobile,
        ratio: SlideRatio::Wide169,
        info_kind: InfoKind::Data,
        style_guide: None,
    });
    assert!(state.editor_ui.open_workspace_for_shared_recipe(1));
    state
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

#[test]
fn the_share_button_sits_left_of_export_and_hits_share() {
    let state = shared_state(true);
    let surface = WorkspaceSurface::for_editor(&state).expect("workspace");
    let layout = surface.layout(W, H);
    let share = surface.share_button(&layout).expect("share offered");
    assert!(share.origin.x + share.size.x <= layout.export.origin.x - 7.9);
    assert_eq!(share.origin.y, layout.export.origin.y);
    assert_eq!(
        surface.hit_test_layout(&layout, centre(share)),
        Some(WorkspaceHit::Share)
    );
    assert_eq!(
        surface.hit_test_layout(&layout, centre(layout.export)),
        Some(WorkspaceHit::Export)
    );
}

#[test]
fn hosts_that_cannot_write_the_page_get_no_share_button() {
    let state = shared_state(false);
    let surface = WorkspaceSurface::for_editor(&state).expect("workspace");
    assert_eq!(surface.share_button(&surface.layout(W, H)), None);
}

#[test]
fn the_make_same_banner_owns_its_button_only_in_the_shared_view() {
    let state = shared_state(true);
    let surface = WorkspaceSurface::for_editor(&state).expect("workspace");
    let layout = surface.layout(W, H);
    let button = surface.make_same_button(&layout).expect("banner up");
    assert_eq!(
        surface.hit_test_layout(&layout, centre(button)),
        Some(WorkspaceHit::MakeSame)
    );

    let mut own_run = shared_state(true);
    own_run.editor_ui.open_workspace_for_generation(
        HomeFamily::Web,
        "mine",
        op_editor_core::TaskDraft::default(),
        0,
        2,
        None,
    );
    let surface = WorkspaceSurface::for_editor(&own_run).expect("workspace");
    assert_eq!(surface.make_same_button(&surface.layout(W, H)), None);
}

#[test]
fn header_buttons_grow_for_long_labels_but_never_shrink_below_export() {
    assert_eq!(header_button_width("分享"), 68.0);
    assert!(header_button_width("Поделиться") > 68.0);
    assert!(header_button_width("これと同じものを作る") > header_button_width("分享"));
}
