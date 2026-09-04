use super::top_bar::*;
use crate::{Point2D, Rect};

#[test]
fn vscode_embed_hides_file_figma_title_and_fullscreen() {
    let mut bar = TopBar::new("Untitled").with_traffic_controls(false);
    bar.embed = op_editor_core::EmbedHost::VsCode;
    // No hit anywhere may resolve to the hidden controls.
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1200.0, TOP_BAR_HEIGHT),
    };
    for x in 0..1200 {
        let hit = bar.hit_test(rect, Point2D::new(x as f32, TOP_BAR_HEIGHT / 2.0));
        assert!(
            !matches!(
                hit,
                Some(TopBarHit::ToggleFileMenu)
                    | Some(TopBarHit::OpenImportMenu)
                    | Some(TopBarHit::ToggleFullscreen)
            ),
            "hidden control hit at x={x}: {hit:?}"
        );
    }
}

#[test]
fn vscode_embed_keeps_preview_account_collaboration_and_shared_chrome() {
    let mut bar = TopBar::new("Untitled").with_traffic_controls(false);
    bar.embed = op_editor_core::EmbedHost::VsCode;
    bar.account_button_visible = true;
    bar.collab.visible = true;
    bar.collab.enabled = true;
    bar.collab.label = "Collaborate".to_string();
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1200.0, TOP_BAR_HEIGHT),
    };
    let hits: Vec<_> = (0..1200)
        .filter_map(|x| bar.hit_test(rect, Point2D::new(x as f32, TOP_BAR_HEIGHT / 2.0)))
        .collect();
    for expect in [
        TopBarHit::ToggleSidebar,
        TopBarHit::ToggleLocale,
        TopBarHit::ToggleTheme,
        TopBarHit::OpenAgentSettings,
        TopBarHit::TogglePreview,
        TopBarHit::Account,
        TopBarHit::Collaboration,
    ] {
        assert!(hits.contains(&expect), "missing {expect:?}");
    }
}
