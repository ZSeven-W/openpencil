//! MCP tab of the settings modal.

use crate::document::{AgentSettings, Document, McpCli};
use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

const TITLE_H: f32 = 36.0;
const SERVER_CARD_H: f32 = 52.0;
const SECTION_GAP: f32 = 28.0;
const SECTION_TITLE_H: f32 = 28.0;
const SUBTITLE_H: f32 = 20.0;
const ROW_GAP_BEFORE_GRID: f32 = 12.0;
const CELL_H: f32 = 52.0;
const CELL_VGAP: f32 = 12.0;
const CELL_HGAP: f32 = 16.0;
const TOGGLE_W: f32 = 36.0;
const TOGGLE_H: f32 = 20.0;
const TOGGLE_KNOB: f32 = 14.0;
const BTN_W: f32 = 72.0;
const BTN_H: f32 = 28.0;
const PORT_FIELD_W: f32 = 64.0;
const PORT_FIELD_H: f32 = 28.0;

fn server_card_top(content: Rect) -> f32 {
    content.origin.y + TITLE_H
}

fn grid_top(content: Rect) -> f32 {
    server_card_top(content) + SERVER_CARD_H + SECTION_GAP + SECTION_TITLE_H + SUBTITLE_H * 2.0 + ROW_GAP_BEFORE_GRID
}

pub(super) fn content_height() -> f32 {
    TITLE_H
        + SERVER_CARD_H
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

fn cli_cell_rect(content: Rect, idx: usize) -> Rect {
    let col = (idx % 2) as f32;
    let row = (idx / 2) as f32;
    let cell_w = (content.size.x - CELL_HGAP) / 2.0;
    Rect {
        origin: Point2D::new(
            content.origin.x + col * (cell_w + CELL_HGAP),
            grid_top(content) + row * (CELL_H + CELL_VGAP),
        ),
        size: Point2D::new(cell_w, CELL_H),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> McpHit {
    if rect_contains(server_button_rect(content), scrolled) {
        return McpHit::ToggleServer;
    }
    for (i, cli) in McpCli::ALL.iter().enumerate() {
        if rect_contains(cli_cell_rect(content, i), scrolled) {
            return McpHit::ToggleCli(*cli);
        }
    }
    McpHit::None
}

pub(super) fn paint_mcp_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    doc: &Document,
    content: Rect,
) {
    let title = TextLayout::single_run(
        t_settings(doc, "settings.mcp.server"),
        "system-ui",
        14.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );
    paint_server_card(cx, theme, settings, doc, content);

    let mut y = server_card_top(content) + SERVER_CARD_H + SECTION_GAP;
    let section_title = TextLayout::single_run(
        t_settings(doc, "settings.mcp.terminalIntegrations"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&section_title, Point2D::new(content.origin.x, y + 16.0));
    y += SECTION_TITLE_H;
    let s1 = TextLayout::single_run(
        t_settings(doc, "settings.mcp.terminalSubtitle1"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&s1, Point2D::new(content.origin.x, y + 13.0));
    y += SUBTITLE_H;
    let s2 = TextLayout::single_run(
        t_settings(doc, "settings.mcp.terminalSubtitle2"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&s2, Point2D::new(content.origin.x, y + 13.0));

    for (i, cli) in McpCli::ALL.iter().enumerate() {
        let cell = cli_cell_rect(content, i);
        paint_cli_cell(cx, theme, *cli, settings.mcp_cli_enabled[i], cell);
    }
}

fn paint_server_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    doc: &Document,
    content: Rect,
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
        Color { r: 0.34, g: 0.78, b: 0.45, a: 1.0 }
    } else {
        theme.muted_foreground
    };
    cx.backend.fill_oval(dot, dot_color);
    let status_text = if running {
        t_settings(doc, "settings.mcp.running")
    } else {
        t_settings(doc, "settings.mcp.stopped")
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
    let port_label_text = t_settings(doc, "settings.mcp.port");
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
    let port_field = Rect {
        origin: Point2D::new(port_field_x, mid_y - PORT_FIELD_H / 2.0),
        size: Point2D::new(PORT_FIELD_W, PORT_FIELD_H),
    };
    cx.backend.stroke_round_rect(port_field, 6.0, theme.border, 1.0);
    let port_str = format!("{}", settings.mcp_server.port);
    let port_w = cx.backend.measure_text(&port_str, 12.0);
    let port_layout = TextLayout::single_run(
        &port_str,
        "system-ui",
        12.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &port_layout,
        Point2D::new(
            port_field.origin.x + (PORT_FIELD_W - port_w) / 2.0,
            port_field.origin.y + PORT_FIELD_H / 2.0 + 5.0,
        ),
    );

    let btn_bg = if running { theme.muted } else { theme.primary };
    let btn_fg = if running { theme.foreground } else { theme.primary_foreground };
    cx.backend.fill_round_rect(btn, 6.0, btn_bg);
    if running {
        cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
    }
    let btn_label = if running {
        t_settings(doc, "settings.mcp.stop")
    } else {
        t_settings(doc, "settings.mcp.start")
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

fn paint_cli_cell(cx: &mut PaintCx<'_>, theme: &Theme, cli: McpCli, enabled: bool, cell: Rect) {
    let bg = if enabled { theme.muted } else { theme.card };
    cx.backend.fill_round_rect(cell, 10.0, bg);
    let border_color = if enabled { theme.primary } else { theme.border };
    let border_w = if enabled { 1.5 } else { 1.0 };
    cx.backend.stroke_round_rect(cell, 10.0, border_color, border_w);

    let label_fg = if enabled { theme.foreground } else { theme.muted_foreground };
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
            cell.origin.x + cell.size.x - 16.0 - TOGGLE_W,
            cell.origin.y + (CELL_H - TOGGLE_H) / 2.0,
        ),
        size: Point2D::new(TOGGLE_W, TOGGLE_H),
    };
    let track_color = if enabled { theme.primary } else { theme.background };
    cx.backend.fill_round_rect(toggle, TOGGLE_H / 2.0, track_color);
    if !enabled {
        cx.backend.stroke_round_rect(toggle, TOGGLE_H / 2.0, theme.border, 1.0);
    }
    let knob_x = if enabled {
        toggle.origin.x + TOGGLE_W - TOGGLE_KNOB - 3.0
    } else {
        toggle.origin.x + 3.0
    };
    let knob = Rect {
        origin: Point2D::new(knob_x, toggle.origin.y + (TOGGLE_H - TOGGLE_KNOB) / 2.0),
        size: Point2D::new(TOGGLE_KNOB, TOGGLE_KNOB),
    };
    cx.backend.fill_oval(knob, theme.foreground);
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
