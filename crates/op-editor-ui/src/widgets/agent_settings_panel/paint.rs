//! Paint pass for [`AgentSettingsPanel`] — the modal frame, close
//! button, Agents tab body, and the shared section-header primitives,
//! plus the `Widget` impl and the drag hook. The top tab strip lives in
//! the `tabs` sibling and the Agents headline in `hero`. Carved off
//! `agent_settings_panel.rs` to keep every file under the 800-line cap.

use super::*;
use crate::widgets::text_metrics;

pub(super) fn agents_content_height(
    settings: &AgentSettings,
    mode: AgentSettingsPanelMode,
    ui: &EditorUiState,
) -> f32 {
    let builtin = AGENTS_HERO_HEIGHT
        + agent_settings_builtin::content_height_for_ui(settings, ui.touch_chrome());
    if !mode.shows_external_agents(ui) {
        return builtin + CONTENT_TAIL_PAD;
    }
    // Walk the same ladder the row geometry does, so the scroll range
    // always ends one tail pad under the last provider row instead of
    // flush against it (or, as it once did, four pixels short of it).
    // `provider_rows_top` reads only its argument's `origin.y`, so a rect
    // at the origin turns it into a pure offset from the content top.
    let at_origin = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(0.0, 0.0),
    };
    let last = AgentProvider::DISPLAY.len() - 1;
    provider_rows_top(at_origin, settings)
        + last as f32 * (CARD_HEIGHT + CARD_GAP)
        + CARD_HEIGHT
        + if settings.provider_verified_connected_at(0) {
            CLAUDE_HINT_H
        } else {
            0.0
        }
        + CONTENT_TAIL_PAD
}

struct PanelPaintArgs<'a> {
    theme: &'a Theme,
    settings: &'a AgentSettings,
    ui: &'a EditorUiState,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
    layout: AgentSettingsLayout,
    scroll: f32,
}

impl<'a> Widget for AgentSettingsPanel<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &crate::widgets::LayoutCx) -> crate::widgets::LayoutBox {
        crate::widgets::LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(PANEL_WIDTH, PANEL_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let layout = self.resolved_layout(rect);
        let scroll = self.effective_scroll(rect);
        paint_panel(
            cx,
            rect,
            PanelPaintArgs {
                theme: &self.theme,
                settings: &self.settings,
                ui: self.ui,
                now_ms: self.now_ms,
                mode: self.mode,
                layout,
                scroll,
            },
        );
        if self.mode.active_tab(&self.settings, self.ui) == AgentSettingsTab::Fonts {
            agent_settings_fonts::paint_picker(
                cx,
                &self.theme,
                rect,
                hero_body_rect_for_ui(layout.content, self.ui.touch_chrome()),
                self.ui,
                scroll,
                self.now_ms,
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label("Settings");
        node
    }
}

fn paint_panel(cx: &mut PaintCx<'_>, panel: Rect, args: PanelPaintArgs<'_>) {
    let PanelPaintArgs {
        theme,
        settings,
        ui,
        now_ms,
        mode,
        layout,
        scroll,
    } = args;
    match layout.kind {
        AgentSettingsSurfaceKind::Compact => cx.backend.fill_rect(panel, theme.card),
        AgentSettingsSurfaceKind::Medium | AgentSettingsSurfaceKind::Expanded => {
            cx.backend.fill_round_rect(panel, 20.0, theme.card);
            cx.backend.stroke_round_rect(panel, 20.0, theme.border, 1.0);
        }
        AgentSettingsSurfaceKind::Desktop => {
            cx.backend.fill_round_rect(panel, 18.0, theme.card);
            cx.backend.stroke_round_rect(panel, 18.0, theme.border, 1.0);
        }
    }
    if layout.kind != AgentSettingsSurfaceKind::Desktop {
        paint_touch_title(cx, theme, ui, layout);
    }
    if layout.kind == AgentSettingsSurfaceKind::Expanded {
        let x = panel.origin.x + layout.header.size.x;
        cx.backend.stroke_line(
            Point2D::new(x, panel.origin.y),
            Point2D::new(x, panel.origin.y + panel.size.y),
            theme.border,
            1.0,
        );
    }
    super::tabs::paint_tab_strip(cx, theme, settings, ui, layout, mode);
    let content_rect = layout.content;
    cx.backend.save();
    cx.backend.clip_rect(layout.content_paint_clip());
    cx.backend.translate(Point2D::new(0.0, -scroll));
    match mode.active_tab(settings, ui) {
        AgentSettingsTab::Agents => {
            paint_agents_tab(cx, theme, settings, ui, content_rect, now_ms, mode)
        }
        AgentSettingsTab::Mcp => {
            agent_settings_mcp::paint_mcp_tab(cx, theme, settings, ui, content_rect, now_ms)
        }
        AgentSettingsTab::Images => {
            paint_secondary_hero(cx, theme, ui, content_rect, "settings.images");
            agent_settings_images::paint_images_tab(
                cx,
                theme,
                settings,
                ui,
                hero_body_rect_for_ui(content_rect, ui.touch_chrome()),
                now_ms,
            )
        }
        AgentSettingsTab::Fonts => {
            paint_secondary_hero(cx, theme, ui, content_rect, "settings.fonts");
            agent_settings_fonts::paint_fonts_tab(
                cx,
                theme,
                ui,
                hero_body_rect_for_ui(content_rect, ui.touch_chrome()),
            )
        }
        AgentSettingsTab::System => {
            agent_settings_system::paint_system_tab(cx, theme, settings, ui, content_rect)
        }
        AgentSettingsTab::Account => {
            paint_secondary_hero(cx, theme, ui, content_rect, "settings.account");
            agent_settings_account::paint_account_tab(
                cx,
                theme,
                ui,
                hero_body_rect_for_ui(content_rect, ui.touch_chrome()),
            )
        }
    }
    cx.backend.restore();
    paint_close(cx, theme, settings, ui, layout);
}

fn paint_touch_title(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    layout: AgentSettingsLayout,
) {
    let text = TextLayout::single_run(
        t_settings(ui, "settings.title"),
        crate::widgets::agent_settings_rows::SETTINGS_FONT_FAMILY,
        20.0,
        theme.foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &text,
        Point2D::new(layout.header.origin.x + 20.0, layout.header.origin.y + 36.0),
    );
}

/// Tab intro for the tabs the panel paints one on behalf of. `prefix`
/// names the i18n family (`settings.images` resolves to
/// `settings.images.heroTitle` plus `.heroSubtitle`), so the key pair
/// can't drift from the tab it belongs to.
fn paint_secondary_hero(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    prefix: &str,
) {
    let (title, subtitle) = match prefix {
        "settings.images" => ("settings.images.heroTitle", "settings.images.heroSubtitle"),
        "settings.fonts" => ("settings.fonts.heroTitle", "settings.fonts.heroSubtitle"),
        _ => (
            "settings.account.heroTitle",
            "settings.account.heroSubtitle",
        ),
    };
    crate::widgets::agent_settings_rows::paint_tab_intro_for_ui(
        cx,
        theme,
        content,
        t_settings(ui, title),
        Some(t_settings(ui, subtitle)),
        ui.touch_chrome(),
    );
}

fn paint_close(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    layout: AgentSettingsLayout,
) {
    let pressed = ui.button_pressed(ButtonPressTarget::AgentSettings(AgentSettingsButton::Close));
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        layout.close_target,
        settings.hover_agent_settings_close,
        pressed,
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        layout.close_icon.origin,
        layout.close_icon.size.x,
        theme.foreground,
        2.0,
    );
}

fn paint_agents_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
    mode: AgentSettingsPanelMode,
) {
    super::hero::paint_agents_hero(cx, theme, ui, content, !ui.touch_chrome());
    let mut y = agents_body_top(content);
    y = agent_settings_builtin::paint_builtin_section(cx, theme, settings, ui, content, y, now_ms);
    if !mode.shows_external_agents(ui) {
        return;
    }
    y += SECTION_GAP;
    y = agent_settings_acp::paint_acp_section(cx, theme, settings, ui, content, y, now_ms);
    y += SECTION_GAP;

    y = paint_section_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.title"),
        "",
        content.origin.x,
        y,
        content.size.x,
    );
    for provider in AgentProvider::DISPLAY {
        let card = agent_card_rect_at(content.origin.x, y, content.size.x);
        // `index` is the ALL storage index (connect state); the card
        // list itself walks DISPLAY order.
        let index = AgentSettings::provider_index(provider);
        paint_agent_card(cx, theme, settings, ui, provider, card, index);
        y += CARD_HEIGHT + CARD_GAP;
        if matches!(provider, AgentProvider::ClaudeCode)
            && settings.provider_verified_connected_at(index)
        {
            let hint = TextLayout::single_run(
                t_settings(ui, "settings.agents.claudeHint"),
                "system-ui",
                12.0,
                (theme.muted_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&hint, Point2D::new(content.origin.x, y + 15.0));
            y += CLAUDE_HINT_H;
        }
    }
}

fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    paint_section_header_inset(cx, theme, title, action, x, y, w, 0.0)
}

#[allow(clippy::too_many_arguments)]
fn paint_section_header_inset(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    x: f32,
    y: f32,
    w: f32,
    right_inset: f32,
) -> f32 {
    crate::widgets::agent_settings_rows::paint_section_title(cx, theme, x, y, title);
    if !action.is_empty() {
        let action_w = text_metrics::measure_chrome(cx.backend, action, 12.0);
        let act = TextLayout::single_run(
            action,
            "system-ui",
            12.0,
            (theme.primary).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&act, Point2D::new(x + w - right_inset - action_w, y + 18.0));
    }
    y + PROVIDER_SECTION_HEADER_H
}

pub fn drag_for_hit(
    _hit: AgentSettingsHit,
) -> Option<op_editor_core::agent_settings::AgentSettingsDrag> {
    None
}
