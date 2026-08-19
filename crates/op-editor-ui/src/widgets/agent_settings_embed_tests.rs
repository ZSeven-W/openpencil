//! Settings-dialog gating for the VS Code / Cursor custom-editor embed.
//!
//! Product intent (binding): when `ui.embed == EmbedHost::VsCode` the
//! `Cmd+,` dialog exposes ONLY the MCP section (server info + CLI
//! integrations). Built-in API-key agent cards, CLI/ACP agent config,
//! image-provider profiles, and system/experimental sections must
//! neither paint nor hit-test. Outside the embed, behavior is unchanged
//! (covered by every other suite in `agent_settings_panel_tests.rs`).

use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_mcp;
use crate::widgets::agent_settings_panel::AgentSettingsPanel;
use crate::widgets::agent_settings_panel_geometry::nav_item_rect;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::agent_settings::AgentSettingsTab;
use op_editor_core::{EditorState, EmbedHost};

/// Capture backend recording every text run's content so nav-label
/// paint calls are assertable.
#[derive(Default)]
struct TextCapture {
    texts: Vec<String>,
}

impl RenderBackend for TextCapture {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        for run in layout.runs() {
            self.texts.push(run.content.clone());
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

fn vscode_embed_state() -> EditorState {
    let mut state = EditorState::default();
    // The default locale is ZhCn — pin EN so label comparisons match
    // the English table regardless of which table this test runs under.
    state.editor_ui.locale = op_i18n::Locale::EnUs;
    state.editor_ui.embed = EmbedHost::VsCode;
    state
}

/// Centre of tab pill `index` in a strip sized for the VS Code embed's
/// single visible tab — index 1 therefore lands beside the strip, which
/// is exactly what the "no second tab" assertion needs.
fn nav_row_center(panel_rect: Rect, index: usize) -> Point2D {
    let row = nav_item_rect(panel_rect, index, 1);
    Point2D::new(
        row.origin.x + row.size.x / 2.0,
        row.origin.y + row.size.y / 2.0,
    )
}

#[test]
fn vscode_embed_nav_strip_has_only_the_mcp_row() {
    let state = vscode_embed_state();
    let panel = AgentSettingsPanel::for_web_editor(&state);
    let rect = panel.rect(1200.0, 800.0);

    assert_eq!(
        panel.nav_at(rect, nav_row_center(rect, 0)),
        Some(AgentSettingsTab::Mcp),
        "the single visible nav row must be MCP"
    );
    assert_eq!(
        panel.nav_at(rect, nav_row_center(rect, 1)),
        None,
        "no second nav row (Agents/Images/System/Account) should hit-test in the VS Code embed"
    );
}

#[test]
fn vscode_embed_opens_directly_on_mcp_even_though_settings_tab_is_agents() {
    let state = vscode_embed_state();
    assert_eq!(
        state.editor_ui.agent_settings.tab,
        AgentSettingsTab::Agents,
        "sanity: the persisted/default settings.tab is Agents"
    );

    let panel = AgentSettingsPanel::for_web_editor(&state);

    // `content_total_height` is keyed off the panel's resolved active
    // tab; matching it against the MCP tab's own content height proves
    // the dialog renders MCP content directly instead of falling back
    // to (or ever showing) Agents.
    let expected = agent_settings_mcp::content_height(
        &state.editor_ui.agent_settings,
        state.editor_ui.external_cli_available,
    );
    assert_eq!(panel.content_total_height(), expected);
}

#[test]
fn vscode_embed_sidebar_paints_only_the_mcp_nav_label() {
    let state = vscode_embed_state();
    let panel = AgentSettingsPanel::for_web_editor(&state);
    let rect = panel.rect(1200.0, 800.0);
    let mut backend = TextCapture::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let mcp_label = t_settings(&state.editor_ui, "settings.tab.mcp");
    let agents_label = t_settings(&state.editor_ui, "settings.tab.agents");
    let images_label = t_settings(&state.editor_ui, "settings.tab.images");
    let system_label = t_settings(&state.editor_ui, "settings.tab.system");

    assert!(
        backend.texts.iter().any(|s| s.as_str() == mcp_label),
        "MCP nav label should paint"
    );
    assert!(
        !backend.texts.iter().any(|s| s.as_str() == agents_label),
        "Agents nav label must not paint in the VS Code embed"
    );
    assert!(
        !backend.texts.iter().any(|s| s.as_str() == images_label),
        "Images nav label must not paint in the VS Code embed"
    );
    assert!(
        !backend.texts.iter().any(|s| s.as_str() == system_label),
        "System nav label must not paint in the VS Code embed"
    );
}
