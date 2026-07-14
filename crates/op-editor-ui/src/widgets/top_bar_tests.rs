use super::top_bar::*;
use crate::theme::Theme;
use crate::widgets::icons::Icon;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect};
use op_editor_core::editor_ui_state::EditorUiState;

fn nearly_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn untitled_carries_default_chinese_label() {
    let bar = TopBar::untitled();
    assert_eq!(bar.file_name, "未命名");
}

#[test]
fn layout_reports_full_width_and_top_bar_height() {
    let bar = TopBar::untitled();
    let cx = super::LayoutCx {
        available_width: 1000.0,
        dpi: 1.0,
    };
    let lb = bar.layout(&cx);
    assert_eq!(lb.rect.size.x, 1000.0);
    assert_eq!(lb.rect.size.y, TOP_BAR_HEIGHT);
}

#[test]
fn access_node_advertises_header_role() {
    let node = TopBar::untitled().access_node();
    assert_eq!(node.role(), accesskit::Role::Header);
}

#[test]
fn for_editor_ui_picks_up_button_hover() {
    let ui = EditorUiState {
        topbar_button_hover: Some(op_editor_core::TopBarButton::ToggleTheme),
        ..Default::default()
    };
    let bar = TopBar::for_editor_ui(&ui);
    assert!(bar.is_hovered(op_editor_core::TopBarButton::ToggleTheme));
    assert!(!bar.is_hovered(op_editor_core::TopBarButton::ToggleSidebar));
}

#[test]
fn for_editor_ui_picks_up_button_press() {
    let ui = EditorUiState {
        pressed_button: Some(op_editor_core::ButtonPressTarget::TopBar(
            op_editor_core::TopBarButton::ToggleTheme,
        )),
        ..Default::default()
    };
    let bar = TopBar::for_editor_ui(&ui);
    assert!(bar.is_pressed(op_editor_core::TopBarButton::ToggleTheme));
    assert!(!bar.is_pressed(op_editor_core::TopBarButton::ToggleSidebar));
}

#[test]
fn agent_chip_hit_area_tracks_measured_text_width() {
    // Regression: the chip hit area used a 12 px/char estimate (+16 px
    // slop) that ballooned the target left across the file-name gap.
    // With a host-measured text width it tracks the painted chip —
    // narrower text → narrower hit area, anchored to the globe on the
    // right. A probe well left of a narrow chip's right edge stays
    // inside a wide chip but falls outside the narrow one.
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1200.0, TOP_BAR_HEIGHT),
    };
    let chip = |text_w: f32| {
        let mut bar = TopBar::new("food.op");
        bar.agent_count = 4;
        bar.mcp_count = 2;
        bar.chip_text_w = Some(text_w);
        bar
    };
    let globe = TopBar::new("food.op").globe_rect(rect);
    let gap = DIVIDER_GAP * 2.0 + DIVIDER_W;
    // 150 px left of the chip's right edge (anchored at globe - gap).
    let probe = Point2D::new(globe.origin.x - gap - 150.0, TOP_BAR_HEIGHT / 2.0);
    assert_eq!(
        chip(250.0).hit_test(rect, probe),
        Some(TopBarHit::OpenAgentSettings),
        "a wide chip still covers the probe",
    );
    assert_ne!(
        chip(10.0).hit_test(rect, probe),
        Some(TopBarHit::OpenAgentSettings),
        "a narrow measured chip must NOT reach 150px left into the gap",
    );
}

#[test]
fn maximize_button_hit_tests_to_toggle_fullscreen() {
    // The Play button only appears with experimental features on; this
    // test asserts the full Maximize | Play | Sun cluster layout.
    let mut bar = TopBar::untitled();
    bar.experimental_enabled = true;
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let cy = 8.0 + ICON_BUTTON / 2.0;
    // Right cluster (right -> left): Maximize | Play | Sun.
    // Rightmost icon (Maximize) -> ToggleFullscreen.
    let fs_cx = 1000.0 - PAD - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(fs_cx, cy)),
        Some(TopBarHit::ToggleFullscreen),
    );
    // 2nd from right (Play) -> TogglePreview.
    let play_cx = 1000.0 - PAD - ICON_BUTTON - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(play_cx, cy)),
        Some(TopBarHit::TogglePreview),
    );
    // 3rd from right (Sun) -> ToggleTheme.
    let sun_cx = 1000.0 - PAD - 2.0 * ICON_BUTTON - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(sun_cx, cy)),
        Some(TopBarHit::ToggleTheme),
    );
}

#[test]
fn preview_button_hidden_unless_experimental_enabled() {
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let cy = 8.0 + ICON_BUTTON / 2.0;
    // The slot 2nd-from-right (where Play sits when shown).
    let play_cx = 1000.0 - PAD - ICON_BUTTON - ICON_BUTTON / 2.0;
    let probe = Point2D::new(play_cx, cy);

    // Default (gate off): no Play button, so the cluster collapses to
    // put the theme toggle in that slot.
    let off = TopBar::untitled();
    assert!(!off.preview_button_visible());
    assert_eq!(off.hit_test(rect, probe), Some(TopBarHit::ToggleTheme));

    // Gate on: the Play button occupies the slot.
    let mut on = TopBar::untitled();
    on.experimental_enabled = true;
    assert!(on.preview_button_visible());
    assert_eq!(on.hit_test(rect, probe), Some(TopBarHit::TogglePreview));
}

#[test]
fn icon_only_git_button_centers_glyph_in_hover_rect() {
    let mut bar = TopBar::untitled();
    bar.git_branch = None;
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let git_rect = bar.git_button_rect(rect);
    let icon_left = TopBar::git_icon_left(git_rect);
    let icon_center = icon_left + ICON_SIZE / 2.0;
    let hover_center = git_rect.origin.x + git_rect.size.x / 2.0;

    assert!(
        nearly_eq(icon_center, hover_center),
        "icon-only git button should center the branch glyph in its hover rect"
    );
}

#[derive(Default)]
struct SvgColorCapture {
    svgs: Vec<Color>,
}

impl crate::RenderBackend for SvgColorCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, color: Color, _: f32) {
        self.svgs.push(color);
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 0.001
        && (a.g - b.g).abs() < 0.001
        && (a.b - b.b).abs() < 0.001
        && (a.a - b.a).abs() < 0.001
}

#[test]
fn compound_icon_button_grays_at_rest_and_darkens_on_hover() {
    let theme = Theme::dark();
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
    };

    let mut rest = SvgColorCapture::default();
    paint_compound_icon_button(
        &mut PaintCx { backend: &mut rest },
        &theme,
        rect,
        Icon::FolderOpen,
        false,
        false,
    );
    assert!(
        !rest.svgs.is_empty(),
        "compound button should stroke glyphs"
    );
    assert!(
        rest.svgs
            .iter()
            .all(|c| color_eq(*c, theme.muted_foreground)),
        "folder + chevron should be muted at rest"
    );

    let mut hover = SvgColorCapture::default();
    paint_compound_icon_button(
        &mut PaintCx {
            backend: &mut hover,
        },
        &theme,
        rect,
        Icon::FolderOpen,
        true,
        false,
    );
    assert!(
        hover.svgs.iter().all(|c| color_eq(*c, theme.foreground)),
        "folder + chevron should darken to foreground on hover"
    );
}

#[test]
fn agent_chip_counts_ready_builtin_agents() {
    // A ready API-key builtin (MiniMax/GLM) is as much an agent as a
    // connected CLI provider — the chip's count must include it.
    use op_editor_core::agent_settings::{BuiltinAgentConfig, BuiltinAgentKind};
    use op_editor_core::agent_settings_builtin_presets::BuiltinAgentPresetKey;
    let mut ui = EditorUiState::default();
    ui.agent_settings.builtin_agents.push(BuiltinAgentConfig {
        id: "bi-1".into(),
        preset: BuiltinAgentPresetKey::Custom,
        display_name: "MiniMax M3".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "sk-test".into(),
        model: "MiniMax-M3".into(),
        base_url: "https://api.minimaxi.com/v1".into(),
        enabled: true,
    });
    // A second builtin that is NOT ready (no key) must not count.
    ui.agent_settings.builtin_agents.push(BuiltinAgentConfig {
        id: "bi-2".into(),
        preset: BuiltinAgentPresetKey::Custom,
        display_name: "Unconfigured".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "".into(),
        model: "x".into(),
        base_url: "".into(),
        enabled: true,
    });
    let bar = TopBar::for_editor_ui(&ui);
    assert_eq!(
        bar.agent_count, 1,
        "one ready builtin counts; the unconfigured one does not"
    );
}

/// Built-in (API-key) agents count toward the chip label but paint no brand
/// icon — the chip must hug `[pad][dot][text][pad]` with no phantom icon
/// cluster (measured: "4 agents · 2 MCP" with zero CLI providers reserved a
/// blank 4-icon span, user screenshot 2026-07-11).
#[test]
fn chip_with_only_builtin_agents_reserves_no_icon_cluster() {
    let mut bar = TopBar::new("test.op");
    bar.agent_count = 4;
    bar.connected = [false; 7];
    assert_eq!(bar.agent_icons_span(), 0.0, "no painted icons, no span");

    bar.connected = [true, false, false, false, false, false, false];
    assert!(
        bar.agent_icons_span() > 0.0,
        "a connected CLI provider brings the cluster back"
    );
}
