//! MCP tab of the settings modal.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::{
    caret_x_for_text, paint_caret, paint_settings_selection, settings_caret_for_focus,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AgentSettings, McpCli, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;

const TITLE_H: f32 = 36.0;
const SERVER_CARD_H: f32 = 52.0;
const CLIENT_CONFIG_H: f32 = 58.0;
const CLIENT_CONFIG_GAP: f32 = 8.0;
const CLIENT_CONFIG_COPY_FEEDBACK_MS: u64 = 2_000;
const SECTION_GAP: f32 = 28.0;
const SECTION_TITLE_H: f32 = 28.0;
const SUBTITLE_H: f32 = 20.0;
const ROW_GAP_BEFORE_GRID: f32 = 12.0;
const CELL_H: f32 = 52.0;
const CELL_VGAP: f32 = 12.0;
const CELL_HGAP: f32 = 16.0;
const BTN_W: f32 = 72.0;
const BTN_H: f32 = 28.0;
const PORT_FIELD_W: f32 = 64.0;
const PORT_FIELD_H: f32 = 28.0;
const CLIENT_COPY_BTN: f32 = 20.0;
const CLIENT_COPY_ICON: f32 = 10.0;
const COPY_FEEDBACK_GREEN: Color = Color {
    r: 0.13,
    g: 0.77,
    b: 0.37,
    a: 1.0,
};

fn server_card_top(content: Rect) -> f32 {
    content.origin.y + TITLE_H
}

fn client_config_block_h(settings: &AgentSettings) -> f32 {
    if settings.mcp_server.running {
        CLIENT_CONFIG_GAP + CLIENT_CONFIG_H
    } else {
        0.0
    }
}

fn grid_top(content: Rect, settings: &AgentSettings) -> f32 {
    server_card_top(content)
        + SERVER_CARD_H
        + client_config_block_h(settings)
        + SECTION_GAP
        + SECTION_TITLE_H
        + SUBTITLE_H * 2.0
        + ROW_GAP_BEFORE_GRID
}

pub(super) fn content_height(settings: &AgentSettings) -> f32 {
    TITLE_H
        + SERVER_CARD_H
        + client_config_block_h(settings)
        + SECTION_GAP
        + SECTION_TITLE_H
        + SUBTITLE_H * 2.0
        + ROW_GAP_BEFORE_GRID
        + 3.0 * CELL_H
        + 2.0 * CELL_VGAP
        + 24.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpHit {
    ToggleServer,
    ToggleCli(McpCli),
    CopyClientConfig,
    FocusPort,
    None,
}

fn server_card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, server_card_top(content)),
        size: Point2D::new(content.size.x, SERVER_CARD_H),
    }
}

fn server_button_rect(content: Rect) -> Rect {
    let card = server_card_rect(content);
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - BTN_W,
            card.origin.y + (SERVER_CARD_H - BTN_H) / 2.0,
        ),
        size: Point2D::new(BTN_W, BTN_H),
    }
}

fn port_field_rect(content: Rect) -> Rect {
    let card = server_card_rect(content);
    let btn = server_button_rect(content);
    let mid_y = card.origin.y + SERVER_CARD_H / 2.0;
    let port_field_x = btn.origin.x - 8.0 - PORT_FIELD_W;
    Rect {
        origin: Point2D::new(port_field_x, mid_y - PORT_FIELD_H / 2.0),
        size: Point2D::new(PORT_FIELD_W, PORT_FIELD_H),
    }
}

fn cli_cell_rect(content: Rect, settings: &AgentSettings, idx: usize) -> Rect {
    let col = (idx % 2) as f32;
    let row = (idx / 2) as f32;
    let cell_w = (content.size.x - CELL_HGAP) / 2.0;
    Rect {
        origin: Point2D::new(
            content.origin.x + col * (cell_w + CELL_HGAP),
            grid_top(content, settings) + row * (CELL_H + CELL_VGAP),
        ),
        size: Point2D::new(cell_w, CELL_H),
    }
}

fn client_config_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            content.origin.x,
            server_card_top(content) + SERVER_CARD_H + CLIENT_CONFIG_GAP,
        ),
        size: Point2D::new(content.size.x, CLIENT_CONFIG_H),
    }
}

fn client_config_copy_button_rect(content: Rect) -> Rect {
    let rect = client_config_rect(content);
    Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - 12.0 - CLIENT_COPY_BTN,
            rect.origin.y + 8.0,
        ),
        size: Point2D::new(CLIENT_COPY_BTN, CLIENT_COPY_BTN),
    }
}

pub fn hit_test(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> McpHit {
    if rect_contains(server_button_rect(content), scrolled) {
        return McpHit::ToggleServer;
    }
    if settings.mcp_server.running
        && rect_contains(client_config_copy_button_rect(content), scrolled)
    {
        return McpHit::CopyClientConfig;
    }
    if !settings.mcp_server.running && rect_contains(port_field_rect(content), scrolled) {
        return McpHit::FocusPort;
    }
    for (i, cli) in McpCli::ALL.iter().enumerate() {
        if rect_contains(cli_cell_rect(content, settings, i), scrolled) {
            return McpHit::ToggleCli(*cli);
        }
    }
    McpHit::None
}

pub(super) fn paint_mcp_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    let title = TextLayout::single_run(
        t_settings(ui, "settings.mcp.server"),
        "system-ui",
        14.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );
    paint_server_card(cx, theme, settings, ui, content, now_ms);
    paint_client_config(cx, theme, settings, ui, content, now_ms);

    let mut y =
        server_card_top(content) + SERVER_CARD_H + client_config_block_h(settings) + SECTION_GAP;
    let section_title = TextLayout::single_run(
        t_settings(ui, "settings.mcp.terminalIntegrations"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&section_title, Point2D::new(content.origin.x, y + 16.0));
    y += SECTION_TITLE_H;
    let s1 = TextLayout::single_run(
        t_settings(ui, "settings.mcp.terminalSubtitle1"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&s1, Point2D::new(content.origin.x, y + 13.0));
    y += SUBTITLE_H;
    let s2 = TextLayout::single_run(
        t_settings(ui, "settings.mcp.terminalSubtitle2"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&s2, Point2D::new(content.origin.x, y + 13.0));

    for (i, cli) in McpCli::ALL.iter().enumerate() {
        let cell = cli_cell_rect(content, settings, i);
        paint_cli_cell(cx, theme, *cli, settings.mcp_cli_enabled[i], cell);
    }
}

fn paint_server_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    let card = server_card_rect(content);
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);
    let running = settings.mcp_server.running;
    let mid_y = card.origin.y + SERVER_CARD_H / 2.0;
    let dot = Rect {
        origin: Point2D::new(card.origin.x + 16.0, mid_y - 4.0),
        size: Point2D::new(8.0, 8.0),
    };
    let dot_color = if running {
        Color {
            r: 0.34,
            g: 0.78,
            b: 0.45,
            a: 1.0,
        }
    } else {
        theme.muted_foreground
    };
    cx.backend.fill_oval(dot, dot_color);
    let status_text = if running {
        t_settings(ui, "settings.mcp.running")
    } else {
        t_settings(ui, "settings.mcp.stopped")
    };
    let status = TextLayout::single_run(
        status_text,
        "system-ui",
        12.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&status, Point2D::new(card.origin.x + 32.0, mid_y + 5.0));

    let btn = server_button_rect(content);
    let port_label_text = t_settings(ui, "settings.mcp.port");
    let port_label_w = cx.backend.measure_text(port_label_text, 11.0);
    let port_field_x = btn.origin.x - 8.0 - PORT_FIELD_W;
    let port_label = TextLayout::single_run(
        port_label_text,
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &port_label,
        Point2D::new(port_field_x - 8.0 - port_label_w, mid_y + 4.0),
    );
    let port_field = port_field_rect(content);
    let port_editable = !running;
    let focused = port_editable && matches!(settings.focus, Some(SettingsFocus::McpPort));
    let port_str = if focused {
        ui.settings_input_draft.clone()
    } else {
        format!("{}", settings.mcp_server.port)
    };
    let (border_color, border_w) = if focused {
        (theme.primary, 1.5)
    } else {
        (theme.border, 1.0)
    };
    cx.backend
        .stroke_round_rect(port_field, 6.0, border_color, border_w);
    let port_w = cx.backend.measure_text(&port_str, 12.0);
    let port_layout = TextLayout::single_run(
        &port_str,
        "system-ui",
        12.0,
        to_jian(if port_editable {
            theme.foreground
        } else {
            theme.muted_foreground
        }),
        Point2D::new(0.0, 0.0),
    );
    let port_x = port_field.origin.x + (PORT_FIELD_W - port_w) / 2.0;
    let port_y = port_field.origin.y + PORT_FIELD_H / 2.0 + 5.0;
    if focused && ui.settings_input_select_all && !port_str.is_empty() {
        paint_settings_selection(
            cx,
            theme,
            &port_str,
            port_x,
            port_y,
            12.0,
            port_field.origin.x + port_field.size.x - 8.0,
        );
    }
    cx.backend
        .draw_text(&port_layout, Point2D::new(port_x, port_y));
    if focused {
        let caret = settings_caret_for_focus(ui, SettingsFocus::McpPort);
        let caret_x = caret_x_for_text(
            cx,
            &port_str,
            caret,
            port_x,
            port_field.origin.x + port_field.size.x - 8.0,
            12.0,
        );
        paint_caret(
            cx,
            theme,
            now_ms,
            ui.settings_input_caret_anchor_ms,
            caret_x,
            port_field.origin.y + (PORT_FIELD_H - 15.0) / 2.0,
        );
    }

    let btn_bg = if running { theme.muted } else { theme.primary };
    let btn_fg = if running {
        theme.foreground
    } else {
        theme.primary_foreground
    };
    cx.backend.fill_round_rect(btn, 6.0, btn_bg);
    if settings.hover_mcp_server_button {
        cx.backend.fill_round_rect(btn, 6.0, theme.button_hover);
    }
    if running {
        cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
    }
    let btn_label = if running {
        t_settings(ui, "settings.mcp.stop")
    } else {
        t_settings(ui, "settings.mcp.start")
    };
    let btn_label_w = cx.backend.measure_text(btn_label, 12.0);
    let lay = TextLayout::single_run(
        btn_label,
        "system-ui",
        12.0,
        to_jian(btn_fg),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &lay,
        Point2D::new(
            btn.origin.x + (BTN_W - btn_label_w) / 2.0,
            btn.origin.y + BTN_H / 2.0 + 5.0,
        ),
    );
}

fn paint_client_config(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    if !settings.mcp_server.running {
        return;
    }
    let rect = client_config_rect(content);
    cx.backend.fill_round_rect(rect, 8.0, theme.card);
    cx.backend.stroke_round_rect(rect, 8.0, theme.border, 1.0);
    let title = TextLayout::single_run(
        t_settings(ui, "agents.mcpClientConfig"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(rect.origin.x + 12.0, rect.origin.y + 18.0),
    );
    let copy = client_config_copy_button_rect(content);
    if settings.hover_mcp_client_config_copy {
        cx.backend.fill_round_rect(copy, 6.0, theme.button_hover);
    }
    let copied = mcp_client_config_copy_feedback_active(settings, now_ms);
    let (icon, icon_color) = if copied {
        (Icon::Check, COPY_FEEDBACK_GREEN)
    } else {
        (Icon::Copy, theme.muted_foreground)
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            copy.origin.x + (CLIENT_COPY_BTN - CLIENT_COPY_ICON) / 2.0,
            copy.origin.y + (CLIENT_COPY_BTN - CLIENT_COPY_ICON) / 2.0,
        ),
        CLIENT_COPY_ICON,
        icon_color,
        1.5,
    );
    let config = settings.mcp_server.client_config_display_text();
    let config = ellipsize(
        cx,
        &config,
        rect.size.x - 24.0 - CLIENT_COPY_BTN - 8.0,
        10.0,
    );
    let config_lay = TextLayout::single_run(
        &config,
        "monospace",
        10.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &config_lay,
        Point2D::new(rect.origin.x + 12.0, rect.origin.y + 40.0),
    );
}

fn mcp_client_config_copy_feedback_active(settings: &AgentSettings, now_ms: u64) -> bool {
    settings
        .mcp_client_config_copied_at_ms
        .map(|copied_at| now_ms.saturating_sub(copied_at) < CLIENT_CONFIG_COPY_FEEDBACK_MS)
        .unwrap_or(false)
}

fn paint_cli_cell(cx: &mut PaintCx<'_>, theme: &Theme, cli: McpCli, enabled: bool, cell: Rect) {
    let bg = if enabled { theme.muted } else { theme.card };
    cx.backend.fill_round_rect(cell, 10.0, bg);
    cx.backend.stroke_round_rect(cell, 10.0, theme.border, 1.0);

    let label_fg = if enabled {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let label = TextLayout::single_run(
        cli.label(),
        "system-ui",
        13.0,
        to_jian(label_fg),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(cell.origin.x + 16.0, cell.origin.y + CELL_H / 2.0 + 5.0),
    );

    let toggle = Rect {
        origin: Point2D::new(
            cell.origin.x + cell.size.x - 16.0 - SETTINGS_SWITCH_W,
            cell.origin.y + (CELL_H - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    };
    paint_settings_switch(cx, theme, toggle, enabled);
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
