//! Share + Make-one-like-this host wiring: a document that carries a
//! recipe opens in the shared view, the banner pre-fills Home, and Home's
//! send generates a fresh document pinned to the recipe's style.

use super::WidgetHostNative;
use op_editor_core::{
    FileAction, HomeDevice, HomeFamily, InfoKind, ShareRecipe, SlideRatio, Tool, WorkspaceHit,
};
use op_editor_ui::widgets::WorkspaceSurface;
use op_editor_ui::Rect;

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn recipe() -> ShareRecipe {
    ShareRecipe {
        brief: "五页咖啡品牌介绍".to_string(),
        family: HomeFamily::Presentation,
        device: HomeDevice::Mobile,
        ratio: SlideRatio::Classic43,
        info_kind: InfoKind::Data,
        style_guide: Some("editorial-dark".to_string()),
    }
}

/// A host that just opened a two-slide document carrying `recipe`.
fn opened(recipe: Option<ShareRecipe>) -> WidgetHostNative {
    let source = r#"{"version":"1.0.0","children":[
        {"type":"frame","id":"s1","x":0,"y":0,"width":960,"height":540,"children":[]},
        {"type":"frame","id":"s2","x":1000,"y":0,"width":960,"height":540,"children":[]}
    ]}"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let meta = op_pen_loader::EditorMeta {
        share_recipe: recipe,
        pinned_style_guide: Some("editorial-dark".to_string()),
        ..op_pen_loader::EditorMeta::default()
    };
    let mut host = WidgetHostNative::new();
    host.set_now_ms(3_000);
    host.install_open_document(document, Some(meta), Some("shared.op".into()))
        .expect("installs");
    host.editor_state_mut().editor_ui.deck_html_export_supported = true;
    host
}

fn centre(rect: Rect) -> (f32, f32) {
    (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn surface_rect(host: &WidgetHostNative, pick: fn(&WorkspaceSurface<'_>) -> Option<Rect>) -> Rect {
    let surface = WorkspaceSurface::for_editor_at(host.editor_state(), 0).expect("workspace");
    pick(&surface).expect("rect")
}

#[test]
fn a_document_with_a_recipe_opens_in_the_shared_view() {
    let mut host = opened(Some(recipe()));
    host.editor_state_mut().tool = Tool::Select;
    assert!(host.adopt_shared_recipe_view(W, H));
    let workspace = &host.editor_state().editor_ui.workspace;
    assert!(workspace.visible && workspace.shared_view);
    assert!(workspace.make_same_banner_visible());
    assert_eq!(workspace.previous_tool, Some(Tool::Select));
    assert_eq!(host.editor_state().tool, Tool::Hand);
}

#[test]
fn a_plain_document_is_left_alone() {
    let mut host = opened(None);
    assert!(!host.adopt_shared_recipe_view(W, H));
    assert!(!host.editor_state().editor_ui.workspace.visible);
}

#[test]
fn the_banner_prefills_home_and_the_send_keeps_the_style_pinned() {
    let mut host = opened(Some(recipe()));
    host.adopt_shared_recipe_view(W, H);
    let button = surface_rect(&host, |surface| {
        surface.make_same_button(&surface.layout(W, H))
    });
    let (x, y) = centre(button);
    assert!(host.apply_press(x, y, W, H));
    host.apply_release_with_viewport(W, H);

    let ui = &host.editor_state().editor_ui;
    assert!(ui.home.visible, "Home comes back");
    assert_eq!(ui.home.task, HomeFamily::Presentation);
    assert_eq!(ui.home.draft, "五页咖啡品牌介绍");
    assert_eq!(ui.home.task_draft().ratio, SlideRatio::Classic43);
    assert_eq!(ui.pinned_style_guide.as_deref(), Some("editorial-dark"));

    // The recipient edits the brief, then sends: a FRESH document (the
    // shared one is not a blank starter) that still carries the style.
    host.editor_state_mut()
        .editor_ui
        .home
        .insert_text("，改成茶饮", 4_000);
    assert!(host.queue_home_send());
    let state = host.editor_state();
    assert_eq!(
        state.editor_ui.pinned_style_guide.as_deref(),
        Some("editorial-dark"),
        "the fresh document keeps the recipe's style"
    );
    assert!(
        state.editor_ui.home.make_same.is_none(),
        "the stage is spent"
    );
    let workspace = &state.editor_ui.workspace;
    assert!(workspace.visible && !workspace.shared_view);
    assert_eq!(workspace.family, HomeFamily::Presentation);
    assert_eq!(workspace.brief, "五页咖啡品牌介绍，改成茶饮");
    let sent = state
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == op_editor_core::ChatRole::User)
        .map(|message| message.content.clone())
        .unwrap_or_default();
    assert!(
        sent.contains("4:3") && sent.contains("改成茶饮"),
        "the recipe's options and the edited brief shape the prompt: {sent}"
    );
    // The new run is the recipe now: sharing it re-captures the style.
    let next = state.editor_ui.share_recipe().expect("live run recipe");
    assert_eq!(next.style_guide.as_deref(), Some("editorial-dark"));
}

#[test]
fn the_header_share_button_queues_the_share_action() {
    let mut host = opened(Some(recipe()));
    host.adopt_shared_recipe_view(W, H);
    let share = surface_rect(&host, |surface| surface.share_button(&surface.layout(W, H)));
    let (x, y) = centre(share);
    assert!(host.apply_press(x, y, W, H));
    host.apply_release_with_viewport(W, H);
    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(FileAction::Share)
    );
}

#[test]
fn hosts_that_cannot_write_the_page_show_no_share_button() {
    let mut host = opened(Some(recipe()));
    host.editor_state_mut().editor_ui.deck_html_export_supported = false;
    host.adopt_shared_recipe_view(W, H);
    let surface = WorkspaceSurface::for_editor_at(host.editor_state(), 0).expect("workspace");
    assert_eq!(surface.share_button(&surface.layout(W, H)), None);
    let focus: Vec<WorkspaceHit> = surface
        .focus_order(&surface.layout(W, H))
        .into_iter()
        .map(|(hit, _)| hit)
        .collect();
    assert!(!focus.contains(&WorkspaceHit::Share));
    assert!(focus.contains(&WorkspaceHit::MakeSame));
}
