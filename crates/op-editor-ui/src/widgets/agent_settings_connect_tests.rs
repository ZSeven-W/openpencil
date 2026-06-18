//! Provider-card connect-lifecycle rendering tests — the probe
//! states ported from the TS providers tab
//! (`agent-settings-providers-tab.tsx:242-269`).

use crate::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
use crate::widgets::agent_settings_panel_geometry::{agent_card_rect_in, connect_btn_rect_at};
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::{AcpAgentConnectOutcome, ProviderConnectPhase};
use op_editor_core::{AgentProvider, EditorState};

/// Capture backend recording every text run's content + color so
/// status-line strings and their TS palette colors are assertable.
#[derive(Default)]
struct TextCapture {
    texts: Vec<(String, jian_core::scene::Color)>,
}

impl RenderBackend for TextCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        for run in layout.runs() {
            self.texts.push((run.content.clone(), run.color));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn paint_panel(state: &mut EditorState) -> TextCapture {
    // The default locale is ZhCn — pin EN so the status-line
    // assertions match the English table.
    state.editor_ui.locale = op_i18n::Locale::EnUs;
    let panel = AgentSettingsPanel::for_editor(state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = TextCapture::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    backend
}

fn find_text<'a>(
    capture: &'a TextCapture,
    needle: &str,
) -> Option<&'a (String, jian_core::scene::Color)> {
    capture.texts.iter().find(|(text, _)| text.contains(needle))
}

#[test]
fn probing_card_paints_connecting_status() {
    let mut state = EditorState::default();
    state.editor_ui.agent_settings.provider_connection[0].phase = ProviderConnectPhase::Probing;
    let capture = paint_panel(&mut state);
    assert!(
        find_text(&capture, "Connecting…").is_some(),
        "probing card should show the Connecting… status line"
    );
    // The Connect button label swaps for the in-flight marker
    // (a standalone "…" run).
    assert!(capture.texts.iter().any(|(text, _)| text == "…"));
}

#[test]
fn connected_card_paints_probe_connection_info_in_green() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.connected[0] = true;
    settings.provider_connection[0].phase = ProviderConnectPhase::Connected;
    settings.provider_connection[0].info = Some("Connected via pro (a@b.c)".to_string());
    let capture = paint_panel(&mut state);
    let (_, color) = find_text(&capture, "✓ Connected via pro (a@b.c)")
        .expect("probe connectionInfo should replace the static subtitle");
    let green = (Color {
        r: 0.34,
        g: 0.78,
        b: 0.45,
        a: 1.0,
    })
    .to_jian();
    assert_eq!(*color, green, "connected info renders in the TS green");
}

#[test]
fn persisted_connected_flag_without_probe_does_not_paint_connected_state() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.connected[0] = true;
    settings.provider_connection[0].phase = ProviderConnectPhase::Idle;

    let capture = paint_panel(&mut state);

    assert!(
        find_text(&capture, "✓").is_none(),
        "a stale persisted connected flag without a verified probe must not paint as connected"
    );
    assert!(
        find_text(&capture, "Connect").is_some(),
        "unverified providers should still show the Connect action"
    );
}

#[test]
fn acp_stale_connected_flag_without_probe_does_not_paint_disconnect_state() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.add_acp_agent();
    settings.acp_agents[0].display_name = "Claude Code".into();
    settings.acp_agents[0].command = "claude".into();
    settings.acp_agents[0].connected = true;

    let capture = paint_panel(&mut state);

    assert!(
        find_text(&capture, "Disconnect").is_none(),
        "a stale ACP connected flag must not paint a verified connected action"
    );

    state
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            "acp-1",
            AcpAgentConnectOutcome {
                connected: true,
                info: Some("Claude Code".into()),
                error: None,
            },
        );
    let capture = paint_panel(&mut state);
    assert!(
        find_text(&capture, "Disconnect").is_some(),
        "a successful ACP probe should paint the disconnect action"
    );
}

#[test]
fn failed_probe_paints_error_in_destructive_red() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.provider_connection[3].phase = ProviderConnectPhase::Error;
    settings.provider_connection[3].error =
        Some("Not authenticated. Run \"copilot login\" first.".to_string());
    let capture = paint_panel(&mut state);
    let (_, color) =
        find_text(&capture, "Not authenticated").expect("probe error should paint on the card");
    let destructive = (crate::theme::Theme::dark().destructive).to_jian();
    assert_eq!(*color, destructive);
}

#[test]
fn not_installed_card_paints_amber_install_guidance() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.provider_connection[4].phase = ProviderConnectPhase::Error;
    settings.provider_connection[4].not_installed = true;
    settings.provider_connection[4].install_command =
        Some("npm install -g @anthropic-ai/gemini-cli".to_string());
    let capture = paint_panel(&mut state);
    let (text, color) = find_text(&capture, "Not installed")
        .expect("not-installed card should show install guidance");
    assert!(
        text.contains("npm install -g @anthropic-ai/gemini-cli"),
        "guidance line should carry the manual install command, got: {text}"
    );
    let amber = (Color {
        r: 0.96,
        g: 0.62,
        b: 0.04,
        a: 1.0,
    })
    .to_jian();
    assert_eq!(*color, amber);
}

#[test]
fn connected_card_with_warning_paints_amber_warning() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.connected[1] = true;
    settings.provider_connection[1].phase = ProviderConnectPhase::Connected;
    settings.provider_connection[1].info = Some("Connected via Codex CLI".to_string());
    settings.provider_connection[1].warning =
        Some("No models found. Try running codex once to populate the model cache.".to_string());
    let capture = paint_panel(&mut state);
    // Single-line card: the actionable warning wins over the green
    // info line (TS renders both on separate lines).
    let (_, color) =
        find_text(&capture, "No models found").expect("warning should paint on the card");
    let amber = (Color {
        r: 0.96,
        g: 0.62,
        b: 0.04,
        a: 1.0,
    })
    .to_jian();
    assert_eq!(*color, amber);
    assert!(find_text(&capture, "✓ Connected via Codex CLI").is_none());
}

#[test]
fn long_status_line_truncates_with_ellipsis() {
    let mut state = EditorState::default();
    let settings = &mut state.editor_ui.agent_settings;
    settings.provider_connection[0].phase = ProviderConnectPhase::Error;
    settings.provider_connection[0].error = Some("x".repeat(400));
    let capture = paint_panel(&mut state);
    let (text, _) = capture
        .texts
        .iter()
        .find(|(text, _)| text.starts_with("xxx"))
        .expect("truncated error should still paint");
    assert!(text.ends_with('…'), "overflowing status must truncate");
    assert!(text.chars().count() < 400);
}

#[test]
fn connect_button_hit_still_resolves_for_idle_card() {
    let state = EditorState::default();
    let panel = AgentSettingsPanel::for_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    // scroll_y defaults to 0, so the scrolled card rect is also the
    // screen-space rect.
    let card = agent_card_rect_in(rect, 0, &state.editor_ui.agent_settings);
    let btn = connect_btn_rect_at(card);
    let point = Point2D::new(
        btn.origin.x + btn.size.x / 2.0,
        btn.origin.y + btn.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(rect, point),
        AgentSettingsHit::Connect(AgentProvider::ClaudeCode)
    );
}
