//! One provider row on the Agents tab — brand avatar, name, the
//! probe-derived status line, a right-aligned status pill, and the
//! Connect / Disconnect button. Split out of `agent_settings_panel.rs`
//! for the 800-line cap.
//!
//! The status line mirrors the TS card
//! (`agent-settings-providers-tab.tsx:242-269`); the pill is the
//! at-a-glance summary of the same probe state.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_panel::{
    AVATAR_ICON, AVATAR_SIZE, CARD_HEIGHT, NAME_FONT, STATUS_PILL_FONT, SUB_FONT,
};
use crate::widgets::agent_settings_panel_geometry::{
    connect_btn_rect_at, disconnect_btn_rect_at, status_pill_slot_rect_at,
};
use crate::widgets::agent_settings_rows::{
    fit_text, measure_settings_text, paint_row_hairline, ROW_LABEL_BASELINE,
    ROW_SECOND_LINE_BASELINE, SETTINGS_FONT_FAMILY,
};
use crate::widgets::brand_icons::{
    paint_brand_logo, paint_deepseek_harness_logo, paint_opencode_logo, BrandLogo,
};
use crate::widgets::button::tokens_from_theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_widgets::components::button::{Button, ButtonVariant};
use op_editor_core::agent_settings::{AgentProvider, AgentSettings};
use op_editor_core::editor_ui_state::EditorUiState;

/// One provider row.
pub(super) fn paint_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    provider: AgentProvider,
    card: Rect,
    index: usize,
) {
    // The provider list paints in `AgentProvider::DISPLAY` order, so
    // the row knows whether it is the last one by identity, not by
    // its `AgentProvider::ALL` storage index (`index` is the ALL
    // index — connect state lives there).
    let last = Some(provider) == AgentProvider::DISPLAY.last().copied();
    // A hairline-separated list row, matching MCP and System — the modal
    // had been speaking two visual languages, cards here and rows there.
    // Hover still washes the whole row so the target reads the same.
    if settings.hover_provider == index {
        cx.backend.fill_round_rect(card, 8.0, theme.button_hover);
    }
    if !last {
        paint_row_hairline(cx, theme, card);
    }
    let avatar = Rect {
        origin: Point2D::new(
            card.origin.x + 16.0,
            card.origin.y + (CARD_HEIGHT - AVATAR_SIZE) / 2.0,
        ),
        size: Point2D::new(AVATAR_SIZE, AVATAR_SIZE),
    };
    cx.backend.fill_round_rect(avatar, 8.0, theme.background);
    let icon_top_left = Point2D::new(
        avatar.origin.x + (AVATAR_SIZE - AVATAR_ICON) / 2.0,
        avatar.origin.y + (AVATAR_SIZE - AVATAR_ICON) / 2.0,
    );
    match provider {
        AgentProvider::ClaudeCode => paint_brand_logo(
            cx.backend,
            BrandLogo::Claude,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::CodexCli => paint_brand_logo(
            cx.backend,
            BrandLogo::OpenAI,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::OpenCode => {
            paint_opencode_logo(cx.backend, icon_top_left, AVATAR_ICON, theme.foreground)
        }
        AgentProvider::GithubCopilot => paint_brand_logo(
            cx.backend,
            BrandLogo::Copilot,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::Antigravity => paint_brand_logo(
            cx.backend,
            BrandLogo::Antigravity,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::GrokBuild => paint_brand_logo(
            cx.backend,
            BrandLogo::Grok,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
        AgentProvider::DeepSeekHarness => {
            paint_deepseek_harness_logo(cx.backend, icon_top_left, AVATAR_ICON, theme.foreground)
        }
    }
    let text_x = card.origin.x + 16.0 + AVATAR_SIZE + 14.0;
    let name = TextLayout::single_run(
        provider.name(),
        SETTINGS_FONT_FAMILY,
        NAME_FONT,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &name,
        Point2D::new(text_x, card.origin.y + ROW_LABEL_BASELINE),
    );
    let connected = settings.provider_verified_connected_at(index);
    let conn = &settings.provider_connection[index];
    let sub_localized = t_settings(ui, provider.subtitle_key());
    let green = theme.status_success;
    let amber = theme.status_warning;
    // Subtitle mirrors the TS card's status line
    // (agent-settings-providers-tab.tsx:242-269), collapsed onto the
    // single line this card reserves: probe in flight → muted
    // "Connecting…"; not installed → amber guidance with the manual
    // install command; probe error → destructive error text;
    // connected → green "✓ {connectionInfo}" (probe-verified info)
    // or an amber warning when the probe raised one.
    let probing = conn.phase == op_editor_core::agent_settings::ProviderConnectPhase::Probing;
    let (sub_color, sub_text): (Color, String) = if probing {
        (
            theme.muted_foreground,
            t_settings(ui, "settings.agents.connecting").to_string(),
        )
    } else if conn.not_installed {
        let label = t_settings(ui, "settings.agents.notInstalled");
        match conn.install_command.as_deref() {
            Some(cmd) => (amber, format!("{label} — {cmd}")),
            None => (amber, label.to_string()),
        }
    } else if let Some(error) = conn.error.as_deref() {
        (theme.destructive, error.to_string())
    } else if connected {
        match (conn.warning.as_deref(), conn.info.as_deref()) {
            (Some(warning), _) => (amber, warning.to_string()),
            (None, Some(info)) => (green, format!("✓ {info}")),
            (None, None) => (green, format!("✓ {sub_localized}")),
        }
    } else {
        (theme.muted_foreground, sub_localized.to_string())
    };
    // Bound the status line by the WIDEST action button's left edge, not
    // by whichever control this row currently paints — the truncation
    // point then stays put as the row's state changes, and hovering a
    // connected row cannot slide Disconnect over the text.
    let sub_max_w = (disconnect_btn_rect_at(card).origin.x - 12.0 - text_x).max(0.0);
    let sub_text = fit_text(cx, &sub_text, sub_max_w, SUB_FONT);
    let sub = TextLayout::single_run(
        &sub_text,
        SETTINGS_FONT_FAMILY,
        SUB_FONT,
        (sub_color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &sub,
        Point2D::new(text_x, card.origin.y + ROW_SECOND_LINE_BASELINE),
    );
    let (pill_color, pill_label) = if probing {
        (
            theme.muted_foreground,
            t_settings(ui, "settings.agents.statusChecking"),
        )
    } else if connected {
        (green, t_settings(ui, "settings.agents.statusConnected"))
    } else {
        (
            theme.muted_foreground,
            t_settings(ui, "settings.agents.statusNotConnected"),
        )
    };
    paint_status_pill(cx, theme, card, pill_color, pill_label, connected);
    let hovered = settings.hover_provider == index;
    let tokens = tokens_from_theme(theme);
    if connected {
        // Disconnect only surfaces on card hover — a non-primary destructive
        // action, so jian's DestructiveOutline (red border + red label) owns
        // the look + feedback instead of a hand-rolled fill+stroke+text.
        if hovered {
            Button {
                label: t_settings(ui, "settings.agents.disconnect"),
                icon_paths: None,
                variant: ButtonVariant::DestructiveOutline,
                enabled: true,
                hovered: false,
                pressed: false,
                font_size: 12.0,
            }
            .paint(cx.backend, disconnect_btn_rect_at(card), &tokens);
        }
    } else {
        // While the probe runs the button reads "…" and is disabled — the TS
        // card swaps the label for a spinner (the press handler ignores
        // Connect while Probing). jian Button Primary owns the centered label.
        let label = if probing {
            "…"
        } else {
            t_settings(ui, "settings.agents.connect")
        };
        Button {
            label,
            icon_paths: None,
            variant: ButtonVariant::Primary,
            enabled: !probing,
            hovered: false,
            pressed: false,
            font_size: 12.0,
        }
        .paint(cx.backend, connect_btn_rect_at(card), &tokens);
    }
}

/// Right-aligned status readout: a coloured dot plus one word of state.
/// The pill hugs its label inside the row's fixed pill slot, so short
/// labels ("Connected") do not stretch into an empty capsule while the
/// slot itself keeps the hit-test geometry measurement-free.
fn paint_status_pill(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    card: Rect,
    color: Color,
    label: &str,
    connected: bool,
) {
    const DOT: f32 = 7.0;
    const PAD_LEFT: f32 = 10.0;
    const PAD_RIGHT: f32 = 12.0;
    const DOT_GAP: f32 = 7.0;

    let slot = status_pill_slot_rect_at(card);
    let label_budget = (slot.size.x - PAD_LEFT - DOT - DOT_GAP - PAD_RIGHT).max(0.0);
    let label = fit_text(cx, label, label_budget, STATUS_PILL_FONT);
    let label_w = measure_settings_text(cx, &label, STATUS_PILL_FONT);
    let pill_w = (PAD_LEFT + DOT + DOT_GAP + label_w + PAD_RIGHT).min(slot.size.x);
    let pill = Rect {
        origin: Point2D::new(slot.origin.x + slot.size.x - pill_w, slot.origin.y),
        size: Point2D::new(pill_w, slot.size.y),
    };
    let fill = if connected {
        color.with_alpha(0.14)
    } else {
        theme.muted
    };
    cx.backend.fill_round_rect(pill, pill.size.y / 2.0, fill);
    let dot = Rect {
        origin: Point2D::new(
            pill.origin.x + PAD_LEFT,
            pill.origin.y + (pill.size.y - DOT) / 2.0,
        ),
        size: Point2D::new(DOT, DOT),
    };
    cx.backend.fill_round_rect(dot, DOT / 2.0, color);
    let text = TextLayout::single_run(
        &label,
        SETTINGS_FONT_FAMILY,
        STATUS_PILL_FONT,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &text,
        Point2D::new(
            dot.origin.x + DOT + DOT_GAP,
            jian_widgets::centered_text_baseline_y(pill, STATUS_PILL_FONT),
        ),
    );
}
