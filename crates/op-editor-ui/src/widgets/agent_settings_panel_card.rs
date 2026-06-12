//! One provider card on the Agents tab — brand avatar, name, the
//! probe-derived status line, and the Connect / Disconnect button.
//! Split out of `agent_settings_panel.rs` for the 800-line cap.
//!
//! The status line mirrors the TS card
//! (`agent-settings-providers-tab.tsx:242-269`).

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_panel::{
    AVATAR_ICON, AVATAR_SIZE, CARD_HEIGHT, CONNECT_BTN_W, NAME_FONT, SUB_FONT,
};
use crate::widgets::agent_settings_panel_geometry::{
    connect_btn_rect_at, disconnect_btn_rect_at, to_jian,
};
use crate::widgets::brand_icons::{paint_brand_logo, paint_opencode_logo, BrandLogo};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AgentProvider, AgentSettings};
use op_editor_core::editor_ui_state::EditorUiState;

pub(super) fn paint_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    provider: AgentProvider,
    card: Rect,
    index: usize,
) {
    let outlined = matches!(provider, AgentProvider::ClaudeCode);
    let card_hovered = settings.hover_provider == index;
    if outlined {
        cx.backend.fill_round_rect(card, 10.0, theme.muted);
    } else if card_hovered {
        // Subtle wash on hover so the whole row reads as "I'm
        // pointing at this card" rather than leaving the hover
        // signal entirely on the trailing button.
        cx.backend.fill_round_rect(card, 10.0, theme.accent);
    }
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);
    let avatar = Rect {
        origin: Point2D::new(
            card.origin.x + 12.0,
            card.origin.y + (CARD_HEIGHT - AVATAR_SIZE) / 2.0,
        ),
        size: Point2D::new(AVATAR_SIZE, AVATAR_SIZE),
    };
    cx.backend.fill_round_rect(avatar, 6.0, theme.background);
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
        AgentProvider::GeminiCli => paint_brand_logo(
            cx.backend,
            BrandLogo::Gemini,
            icon_top_left,
            AVATAR_ICON,
            theme.foreground,
        ),
    }
    let text_x = card.origin.x + 12.0 + AVATAR_SIZE + 12.0;
    let name = TextLayout::single_run(
        provider.name(),
        "system-ui",
        NAME_FONT,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&name, Point2D::new(text_x, card.origin.y + 22.0));
    let connected = settings.connected[index];
    let conn = &settings.provider_connection[index];
    let sub_localized = t_settings(ui, provider.subtitle_key());
    let green = Color {
        r: 0.34,
        g: 0.78,
        b: 0.45,
        a: 1.0,
    };
    // amber-500 — the TS notInstalled / warning text color.
    let amber = Color {
        r: 0.96,
        g: 0.62,
        b: 0.04,
        a: 1.0,
    };
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
    let sub_max_w = card.size.x - (12.0 + AVATAR_SIZE + 12.0) - (CONNECT_BTN_W + 24.0);
    let sub_text = truncate_text(cx, &sub_text, SUB_FONT, sub_max_w);
    let sub = TextLayout::single_run(
        &sub_text,
        "system-ui",
        SUB_FONT,
        to_jian(sub_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&sub, Point2D::new(text_x, card.origin.y + 38.0));
    let hovered = settings.hover_provider == index;
    if connected {
        if hovered {
            let btn = disconnect_btn_rect_at(card);
            let red = Color {
                r: 0.93,
                g: 0.30,
                b: 0.30,
                a: 1.0,
            };
            cx.backend.fill_round_rect(btn, 6.0, theme.muted);
            cx.backend.stroke_round_rect(btn, 6.0, red, 1.0);
            let label = t_settings(ui, "settings.agents.disconnect");
            let lw = cx.backend.measure_text(label, 12.0);
            let layout = TextLayout::single_run(
                label,
                "system-ui",
                12.0,
                to_jian(red),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &layout,
                Point2D::new(btn.origin.x + (btn.size.x - lw) / 2.0, btn.origin.y + 18.0),
            );
        }
    } else {
        let btn = connect_btn_rect_at(card);
        cx.backend.fill_round_rect(btn, 5.0, theme.primary);
        // While the probe runs the button reads "…" — the TS card
        // swaps the label for a spinner and disables the button
        // (the press handler ignores Connect while Probing).
        let label = if probing {
            "…"
        } else {
            t_settings(ui, "settings.agents.connect")
        };
        let lw = cx.backend.measure_text(label, 12.0);
        let layout = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            to_jian(theme.primary_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(btn.origin.x + (btn.size.x - lw) / 2.0, btn.origin.y + 18.0),
        );
    }
}

/// Trim `text` with a trailing ellipsis so it fits `max_w` — the
/// provider-card status line carries probe-derived strings of
/// unbounded length (errors, connection info, install commands).
fn truncate_text(cx: &mut PaintCx<'_>, text: &str, font_size: f32, max_w: f32) -> String {
    if cx.backend.measure_text(text, font_size) <= max_w {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        if cx.backend.measure_text(&format!("{out}…"), font_size) > max_w {
            out.pop();
            out.push('…');
            return out;
        }
    }
    out
}
