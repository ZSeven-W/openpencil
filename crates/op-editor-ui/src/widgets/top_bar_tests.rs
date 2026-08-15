use super::top_bar::*;
use super::top_bar_title::{elide_filename_to_width, TopBarTitleLayout};
use crate::theme::Theme;
use crate::widgets::icons::Icon;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect};
use op_editor_core::editor_ui_state::EditorUiState;

fn nearly_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn title_test_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { size * 0.68 } else { size })
        .sum()
}

#[test]
fn long_file_name_middle_elides_and_preserves_extension() {
    let input = "openpencil-memory-repro (1).op";
    let output = elide_filename_to_width(input, 92.0, |text| title_test_width(text, 13.0));

    assert_ne!(output, input);
    assert!(output.contains('…'), "got {output:?}");
    assert!(
        output.ends_with(".op"),
        "extension must survive: {output:?}"
    );
    assert!(title_test_width(&output, 13.0) <= 92.0);
}

#[test]
fn long_dirty_title_stays_between_left_and_right_controls() {
    let mut bar = TopBar::new("openpencil-memory-repro (1).op");
    bar.edited = true;
    bar.label_edited = "— 已编辑";
    bar.agent_count = 0;
    bar.mcp_count = 0;
    bar.label_agents_and_mcp = "Agents & MCP";
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(760.0, TOP_BAR_HEIGHT),
    };

    let layout = bar.title_layout(rect, title_test_width);
    let slot_right = layout.slot.origin.x + layout.slot.size.x;
    assert!(layout.file_name.ends_with(".op"));
    assert!(layout.file_name.contains('…'));
    assert!(
        layout.edited_x.is_some(),
        "dirty marker must remain allocated"
    );
    let git = layout.git_rect.expect("760px window still fits Git icon");
    assert!(layout.file_x >= layout.slot.origin.x);
    assert!(git.origin.x + git.size.x <= slot_right + 0.01);
    assert_eq!(
        bar.hit_test(
            rect,
            Point2D::new(
                bar.git_button_rect(rect).origin.x + ICON_BUTTON / 2.0,
                TOP_BAR_HEIGHT / 2.0,
            ),
        ),
        Some(TopBarHit::ToggleGitPanel),
    );
}

fn title_group_center_x(bar: &TopBar, layout: &TopBarTitleLayout) -> f32 {
    let group_right = layout
        .git_rect
        .map(|git| git.origin.x + git.size.x)
        .or_else(|| {
            layout
                .edited_x
                .map(|edited_x| edited_x + title_test_width(bar.label_edited, 11.0))
        })
        .unwrap_or_else(|| layout.file_x + title_test_width(&layout.file_name, 13.0));
    (layout.file_x + group_right) / 2.0
}

#[test]
fn title_group_stays_at_window_center_for_short_and_long_names() {
    let make_bar = |file_name: &str| {
        let mut bar = TopBar::new(file_name);
        bar.edited = true;
        bar.label_edited = "— 已编辑";
        bar.agent_count = 0;
        bar.mcp_count = 0;
        bar.label_agents_and_mcp = "Agents & MCP";
        bar
    };
    let short = make_bar("test.op");
    let long = make_bar("openpencil-super-long-project-file-name-for-title-overflow-check (1).op");
    let rect = Rect {
        origin: Point2D::new(37.0, 0.0),
        size: Point2D::new(1_400.0, TOP_BAR_HEIGHT),
    };
    let short_layout = short.title_layout(rect, title_test_width);
    let long_layout = long.title_layout(rect, title_test_width);
    let window_center = rect.origin.x + rect.size.x / 2.0;

    assert!(short_layout.git_rect.is_some());
    assert!(long_layout.git_rect.is_some());
    assert!(nearly_eq(
        title_group_center_x(&short, &short_layout),
        window_center
    ));
    assert!(nearly_eq(
        title_group_center_x(&long, &long_layout),
        window_center
    ));
    assert!(long_layout.file_name.contains('…'), "{long_layout:?}");
    assert!(long_layout.file_name.ends_with(".op"));
}

#[test]
fn title_group_center_is_independent_of_asymmetric_chrome() {
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(1_600.0, TOP_BAR_HEIGHT),
    };
    let mut compact = TopBar::new("Untitled");
    compact.git_branch = Some("main".to_string());

    let mut crowded = TopBar::new("Untitled");
    crowded.git_branch = Some("main".to_string());
    crowded.agent_count = 6;
    crowded.connected = [true; 7];
    crowded.mcp_count = 8;
    crowded.account_button_visible = true;
    crowded.collab.visible = true;
    crowded.collab.label = "Connected".to_string();

    for bar in [&compact, &crowded] {
        let layout = bar.title_layout(rect, title_test_width);
        assert!(layout.git_rect.is_some(), "{layout:?}");
        assert!(nearly_eq(
            title_group_center_x(bar, &layout),
            rect.size.x / 2.0
        ));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn measured_git_hit_tracks_the_painted_centered_group() {
    let bar = TopBar::new("centered-document.op");
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(1_200.0, TOP_BAR_HEIGHT),
    };
    let compact_metric = |text: &str, size: f32| text.chars().count() as f32 * size * 0.2;
    let measured = bar.git_button_rect_with_measure(rect, compact_metric);
    let estimated = bar.git_button_rect(rect);
    let point = Point2D::new(
        measured.origin.x + measured.size.x / 2.0,
        TOP_BAR_HEIGHT / 2.0,
    );

    assert!(
        (measured.origin.x - estimated.origin.x).abs() > ICON_BUTTON,
        "fixture must distinguish exact and fallback geometry"
    );
    assert_eq!(
        bar.hit_test_with_measure(rect, point, compact_metric),
        Some(TopBarHit::ToggleGitPanel)
    );
    assert_ne!(
        bar.hit_test(rect, point),
        Some(TopBarHit::ToggleGitPanel),
        "fallback geometry intentionally sits elsewhere in this fixture"
    );
}

#[test]
fn narrow_dirty_title_drops_git_before_filename_extension_or_status() {
    let mut bar = TopBar::new("openpencil-memory-repro (1).op");
    bar.edited = true;
    bar.label_edited = "— 已编辑";
    bar.agent_count = 0;
    bar.mcp_count = 0;
    bar.label_agents_and_mcp = "Agents & MCP";
    // Two ICON_BUTTONs wider than the fixture used before the export and
    // Asset Center buttons joined the right cluster: the title slot ends
    // where that cluster begins, so the window that produces "Git dropped,
    // name intact" moved right by exactly the width of the new buttons.
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(620.0 + ICON_BUTTON * 2.0, TOP_BAR_HEIGHT),
    };

    let layout = bar.title_layout(rect, title_test_width);
    assert!(layout.file_name.ends_with(".op"), "{layout:?}");
    assert!(layout.edited_x.is_some());
    assert!(
        layout.git_rect.is_none(),
        "Git yields before identity/status"
    );
}

#[test]
fn short_file_name_remains_unelided_and_keeps_exact_git_gap() {
    let bar = TopBar::new("test.op");
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(1200.0, TOP_BAR_HEIGHT),
    };
    let layout = bar.title_layout(rect, title_test_width);
    let file_w = title_test_width(&layout.file_name, 13.0);
    let git = layout.git_rect.expect("wide title bar shows Git");

    assert_eq!(layout.file_name, "test.op");
    assert!(nearly_eq(git.origin.x - (layout.file_x + file_w), 10.0));
}

#[test]
fn zero_width_title_slot_paints_no_filename_or_git_target() {
    let mut bar = TopBar::new("very-long-file-name.op");
    bar.edited = true;
    bar.label_edited = "— Edited";
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(260.0, TOP_BAR_HEIGHT),
    };
    let layout = bar.title_layout(rect, title_test_width);

    assert_eq!(layout.slot.size.x, 0.0);
    assert!(layout.file_name.is_empty());
    assert!(layout.git_rect.is_none());
}

#[test]
fn new_carries_the_supplied_file_name() {
    let bar = TopBar::new("Untitled");
    assert_eq!(bar.file_name, "Untitled");
}

#[test]
fn layout_reports_full_width_and_top_bar_height() {
    let bar = TopBar::new("Untitled");
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
    let node = TopBar::new("Untitled").access_node();
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
fn for_editor_ui_uses_transient_host_theme_for_paint() {
    let ui = EditorUiState {
        theme_mode: op_editor_core::ThemeMode::Light,
        host_theme_override: Some(op_editor_core::ThemeMode::Dark),
        ..Default::default()
    };
    let bar = TopBar::for_editor_ui(&ui);
    assert_eq!(bar.theme_mode, op_editor_core::ThemeMode::Dark);
    assert!(bar.theme.background.r < 0.1);
    assert_eq!(ui.theme_mode, op_editor_core::ThemeMode::Light);
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
fn collaboration_chip_is_a_real_hit_target_only_when_available() {
    let rect = Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(1200.0, TOP_BAR_HEIGHT),
    };
    let hidden = TopBar::for_editor_ui(&EditorUiState::default());
    assert!(!hidden.collab.visible);

    let mut ui = EditorUiState::default();
    ui.collab.availability = op_editor_core::CollabAvailability::Ready;
    let bar = TopBar::for_editor_ui(&ui);
    let chip = bar.collaboration_chip_rect_estimated(rect);
    assert!(chip.size.x > 0.0);
    assert_eq!(
        bar.hit_test(
            rect,
            Point2D::new(
                chip.origin.x + chip.size.x / 2.0,
                chip.origin.y + chip.size.y / 2.0,
            ),
        ),
        Some(TopBarHit::Collaboration)
    );
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
    // The Play button is unconditionally available (desktop-only, not
    // experimental-gated); this test asserts the full Maximize | Play |
    // Download | Sun cluster layout.
    let bar = TopBar::new("Untitled");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let cy = 8.0 + ICON_BUTTON / 2.0;
    // Right cluster (right -> left): Maximize | Play | Download | Sun.
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
    // 3rd from right (Download) -> OpenExportMenu.
    let export_cx = 1000.0 - PAD - 2.0 * ICON_BUTTON - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(export_cx, cy)),
        Some(TopBarHit::OpenExportMenu),
    );
    // 4th from right (Palette) -> OpenAssetCenter.
    let asset_cx = 1000.0 - PAD - 3.0 * ICON_BUTTON - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(asset_cx, cy)),
        Some(TopBarHit::OpenAssetCenter),
    );
    // 5th from right (Sun) -> ToggleTheme.
    let sun_cx = 1000.0 - PAD - 4.0 * ICON_BUTTON - ICON_BUTTON / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(sun_cx, cy)),
        Some(TopBarHit::ToggleTheme),
    );
}

/// The export button anchors its own dropdown, so paint, hit-test and the
/// anchor must all agree on where it is — and it must not steal the Play
/// button's slot on a host that hides Play (web).
#[test]
fn export_button_sits_left_of_play_and_right_of_theme() {
    let bar = TopBar::new("Untitled");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let export = bar.export_button_rect(rect);
    let cy = export.origin.y + export.size.y / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(export.origin.x + 2.0, cy)),
        Some(TopBarHit::OpenExportMenu),
        "left edge of the painted button is inside its hit area",
    );
    assert_eq!(
        bar.hit_test(
            rect,
            Point2D::new(export.origin.x + export.size.x - 2.0, cy)
        ),
        Some(TopBarHit::OpenExportMenu),
        "right edge of the painted button is inside its hit area",
    );
    // Neighbours: Play claims the slot to the right, theme the one left.
    if bar.preview_button_visible() {
        assert_eq!(
            bar.hit_test(
                rect,
                Point2D::new(export.origin.x + export.size.x + ICON_BUTTON / 2.0, cy)
            ),
            Some(TopBarHit::TogglePreview),
        );
    }
    assert_eq!(
        bar.hit_test(rect, Point2D::new(export.origin.x - ICON_BUTTON / 2.0, cy)),
        Some(TopBarHit::OpenAssetCenter),
    );
    assert_eq!(
        bar.hit_test(
            rect,
            Point2D::new(export.origin.x - ICON_BUTTON - ICON_BUTTON / 2.0, cy)
        ),
        Some(TopBarHit::ToggleTheme),
    );
}

/// The whole cluster is derived from one walker, so hiding Play (web) or
/// Maximize (VS Code embed) must slide Download, Sun and Globe right by
/// exactly one slot each and leave no gap.
#[test]
fn right_cluster_stays_gapless_when_neighbours_are_hidden() {
    let mut bar = TopBar::new("Untitled");
    bar.embed = op_editor_core::EmbedHost::VsCode;
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let export = bar.export_button_rect(rect);
    let asset_center = bar.asset_center_button_rect(rect);
    assert_eq!(asset_center.origin.x + ICON_BUTTON, export.origin.x);
    let theme = Rect {
        origin: Point2D::new(asset_center.origin.x - ICON_BUTTON, export.origin.y),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    let globe = bar.globe_rect(rect);
    assert_eq!(globe.origin.x + globe.size.x, theme.origin.x);
    let cy = export.origin.y + export.size.y / 2.0;
    assert_eq!(
        bar.hit_test(rect, Point2D::new(theme.origin.x + 2.0, cy)),
        Some(TopBarHit::ToggleTheme),
    );
    assert_eq!(
        bar.hit_test(rect, Point2D::new(globe.origin.x + 2.0, cy)),
        Some(TopBarHit::ToggleLocale),
    );
}

/// Preview graduated out of the experimental-features gate (2026-07):
/// the Play button is now a regular always-on affordance on any host that
/// has `PREVIEW_BUTTON_AVAILABLE` (desktop, not wasm) — no
/// `EditorUiState.agent_settings.experimental_features_enabled` opt-in
/// required. `TopBar` no longer even carries an `experimental_enabled`
/// field; a fresh `TopBar` shows the button unconditionally.
#[test]
fn preview_button_is_always_visible_regardless_of_experimental_toggle() {
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let cy = 8.0 + ICON_BUTTON / 2.0;
    // The slot 2nd-from-right (where Play sits).
    let play_cx = 1000.0 - PAD - ICON_BUTTON - ICON_BUTTON / 2.0;
    let probe = Point2D::new(play_cx, cy);

    let bar = TopBar::new("Untitled");
    assert!(bar.preview_button_visible());
    assert_eq!(bar.hit_test(rect, probe), Some(TopBarHit::TogglePreview));
}

#[test]
fn icon_only_git_button_centers_glyph_in_hover_rect() {
    let mut bar = TopBar::new("Untitled");
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
    fills: Vec<(String, Point2D, f32, f32, Color)>,
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
    fn fill_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, viewbox: f32, color: Color) {
        self.fills
            .push((d.to_owned(), top_left, size, viewbox, color));
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
fn import_button_uses_supplied_filled_svg_and_keeps_dropdown_chevron() {
    let theme = Theme::dark();
    let mut capture = SvgColorCapture::default();
    paint_import_button(
        &mut PaintCx {
            backend: &mut capture,
        },
        &theme,
        0.0,
        ICON_BUTTON / 2.0,
        false,
        false,
    );

    assert_eq!(capture.fills.len(), 2, "the supplied SVG has two paths");
    assert_eq!(
        capture.svgs.len(),
        1,
        "the dropdown chevron remains stroked"
    );
    let expected_y = (ICON_BUTTON - (ICON_SIZE + 1.0)) / 2.0
        + ((ICON_SIZE + 1.0) - (ICON_SIZE + 1.0) * 1024.0 / 1201.0) / 2.0;
    for (path, origin, size, viewbox, color) in &capture.fills {
        assert!(path.starts_with('M'));
        assert!(nearly_eq(origin.x, 8.0));
        assert!(nearly_eq(origin.y, expected_y));
        assert!(nearly_eq(*size, ICON_SIZE + 1.0));
        assert!(nearly_eq(*viewbox, 1201.0));
        assert!(color_eq(*color, theme.muted_foreground));
    }
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
        models: vec!["MiniMax-M3".into()],
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
        models: vec!["x".into()],
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

#[test]
fn account_button_gate_off_removes_hit_target_and_layout_slot() {
    let bar = TopBar::new("Untitled");
    assert!(
        !bar.account_button_visible,
        "the account button must default to hidden"
    );
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let account = bar.account_button_rect(rect);
    let globe = bar.globe_rect(rect);
    assert!(
        nearly_eq(account.origin.x + account.size.x, globe.origin.x),
        "dormant avatar geometry should stay ready for the gated-on state"
    );
    assert!(
        nearly_eq(bar.chip_right_anchor_x(rect), globe.origin.x),
        "hiding the avatar must also collapse its layout slot"
    );
    for x in 0..1000 {
        let hit = bar.hit_test(rect, Point2D::new(x as f32, TOP_BAR_HEIGHT / 2.0));
        assert_ne!(
            hit,
            Some(TopBarHit::Account),
            "hidden account control hit at x={x}"
        );
    }
}

#[test]
fn account_button_gate_on_restores_hit_target_and_layout_slot() {
    let mut bar = TopBar::new("Untitled");
    bar.account_button_visible = true;
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(1000.0, TOP_BAR_HEIGHT),
    };
    let account = bar.account_button_rect(rect);
    assert!(
        nearly_eq(bar.chip_right_anchor_x(rect), account.origin.x),
        "an enabled avatar must reclaim its layout slot"
    );
    let center = Point2D::new(
        account.origin.x + account.size.x / 2.0,
        account.origin.y + account.size.y / 2.0,
    );
    assert_eq!(bar.hit_test(rect, center), Some(TopBarHit::Account));
}

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
fn vscode_embed_keeps_sidebar_locale_theme_and_chip() {
    let mut bar = TopBar::new("Untitled").with_traffic_controls(false);
    bar.embed = op_editor_core::EmbedHost::VsCode;
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
    ] {
        assert!(hits.contains(&expect), "missing {expect:?}");
    }
}

#[test]
fn for_editor_ui_carries_signed_in_account() {
    let ui = EditorUiState {
        account: op_editor_core::AccountState::SignedIn {
            display_name: "Fini".to_string(),
            username: "fini".to_string(),
        },
        ..EditorUiState::default()
    };
    let bar = TopBar::for_editor_ui(&ui);
    assert!(bar.account.is_signed_in());
    assert_eq!(bar.account.initial(), 'F');
}
