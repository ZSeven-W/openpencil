//! MCP tab of the settings modal.
//!
//! Layout follows the modal's shared language (`agent_settings_rows`):
//! a compact tab intro, then borderless full-width rows separated by
//! hairlines.
//! The server lives on one row, each CLI integration on its own row with
//! a switch, and the endpoint JSON sits under a "custom configuration"
//! section header carrying the copy action. None of the write behaviour
//! changed with the reshuffle — a toggled row still writes the same MCP
//! endpoint into the same CLI config file.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::paint_settings_input_view;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_metrics::CONTENT_TAIL_PAD;
use crate::widgets::agent_settings_rows::{
    fit_text, measure_settings_text, paint_footnote, paint_row_hairline, paint_row_label,
    paint_row_label_above_status, paint_row_status_line, paint_section_title, paint_tab_intro,
    row_control_rect, row_height, row_rect, tab_intro_height, RowLines, FOOTNOTE_H, ROW_HEIGHT,
    SECTION_GAP, SECTION_HEADER_H, SETTINGS_FONT_FAMILY,
};
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AgentSettings, McpCli, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

const CLIENT_CONFIG_COPY_FEEDBACK_MS: u64 = 2_000;
const BTN_W: f32 = 72.0;
const BTN_H: f32 = 28.0;
const PORT_FIELD_W: f32 = 64.0;
const PORT_FIELD_H: f32 = 28.0;
const COPY_BTN_W: f32 = 132.0;
const COPY_BTN_H: f32 = 28.0;
const COPY_BTN_ICON: f32 = 13.0;
/// Description line plus the monospace endpoint line under the custom
/// configuration header.
const CONFIG_BODY_H: f32 = 46.0;
/// Width the server row reserves on its right for port label + field +
/// the Start/Stop button.
const SERVER_CONTROLS_W: f32 = 208.0;
const SECTION_ICON: f32 = 14.0;

/// The MCP tab opens with the shared intro: title plus one muted line.
const INTRO_HAS_DESC: bool = true;

fn body_top(content: Rect) -> f32 {
    content.origin.y + tab_intro_height(INTRO_HAS_DESC)
}

/// The server row carries a status line under its label, so it takes the
/// two-line box; the CLI toggles below are label-only.
const SERVER_ROW_LINES: RowLines = RowLines::Two;

pub(super) fn server_row_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, body_top(content)),
        size: Point2D::new(content.size.x, row_height(SERVER_ROW_LINES)),
    }
}

pub(super) fn integrations_top(content: Rect) -> f32 {
    body_top(content) + row_height(SERVER_ROW_LINES) + SECTION_GAP
}

pub(super) fn cli_row_rect(content: Rect, index: usize) -> Rect {
    row_rect(content, integrations_top(content) + SECTION_HEADER_H, index)
}

fn integrations_block_h() -> f32 {
    SECTION_HEADER_H + McpCli::DISPLAY.len() as f32 * ROW_HEIGHT + FOOTNOTE_H
}

/// Row boxes for this tab, paired with their line count — walked by the
/// traversal layout test.
#[cfg(test)]
pub(super) fn row_boxes(content: Rect) -> Vec<(Rect, RowLines)> {
    let mut boxes = vec![(server_row_rect(content), SERVER_ROW_LINES)];
    if CLI_INTEGRATIONS_AVAILABLE {
        boxes.extend((0..McpCli::DISPLAY.len()).map(|i| (cli_row_rect(content, i), RowLines::One)));
    }
    boxes
}

/// The endpoint JSON only exists once a server is listening (or the embed
/// host manages one), so its section appears with it.
fn has_client_config(settings: &AgentSettings) -> bool {
    settings.mcp_host_managed() || settings.mcp_server.running
}

/// Top of the custom-configuration section. Its *visibility* depends on
/// whether a server is listening (`has_client_config`), but its position
/// does not, so callers gate and place independently.
pub(super) fn custom_config_top(content: Rect) -> f32 {
    let mut y = body_top(content) + row_height(SERVER_ROW_LINES);
    if CLI_INTEGRATIONS_AVAILABLE {
        y += SECTION_GAP + integrations_block_h();
    }
    y + SECTION_GAP
}

pub(super) fn content_height(settings: &AgentSettings) -> f32 {
    let mut h = tab_intro_height(INTRO_HAS_DESC) + row_height(SERVER_ROW_LINES);
    if CLI_INTEGRATIONS_AVAILABLE {
        h += SECTION_GAP + integrations_block_h();
    }
    if has_client_config(settings) {
        h += SECTION_GAP + SECTION_HEADER_H + CONFIG_BODY_H;
    }
    h + CONTENT_TAIL_PAD
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpHit {
    ToggleServer,
    ToggleCli(McpCli),
    CopyClientConfig,
    FocusPort,
    None,
}

pub(super) fn server_button_rect(content: Rect) -> Rect {
    row_control_rect(server_row_rect(content), BTN_W, BTN_H)
}

pub(super) fn port_field_rect(content: Rect) -> Rect {
    let btn = server_button_rect(content);
    let row = server_row_rect(content);
    Rect {
        origin: Point2D::new(
            btn.origin.x - 8.0 - PORT_FIELD_W,
            // Centre in the row's OWN box. Centring on the one-line stride
            // left the field riding high in a two-line row.
            row.origin.y + (row.size.y - PORT_FIELD_H) / 2.0,
        ),
        size: Point2D::new(PORT_FIELD_W, PORT_FIELD_H),
    }
}

pub(super) fn client_config_copy_button_rect(content: Rect) -> Rect {
    let header = Rect {
        origin: Point2D::new(content.origin.x, custom_config_top(content)),
        size: Point2D::new(content.size.x, SECTION_HEADER_H),
    };
    row_control_rect(header, COPY_BTN_W, COPY_BTN_H)
}

/// Host capability: the CLI-integration toggles write MCP endpoints
/// into external CLI config files (`~/.claude.json` etc.) via the
/// desktop MCP runtime (`mcp_integrations.rs`). The web host has no
/// consumer — hide the list there instead of painting toggles that
/// silently do nothing (same pattern as `GIT_BUTTON_AVAILABLE`).
pub(super) const CLI_INTEGRATIONS_AVAILABLE: bool = !cfg!(target_arch = "wasm32");

pub fn hit_test(content: Rect, settings: &AgentSettings, scrolled: Point2D) -> McpHit {
    // Host-managed (embed): the extension owns the lifecycle — no
    // start/stop, no port editing; the config section always copies.
    let host_managed = settings.mcp_host_managed();
    if !host_managed && (server_button_rect(content)).contains(scrolled) {
        return McpHit::ToggleServer;
    }
    if has_client_config(settings) && (client_config_copy_button_rect(content)).contains(scrolled) {
        return McpHit::CopyClientConfig;
    }
    if !host_managed
        && !settings.mcp_server.running
        && (port_field_rect(content)).contains(scrolled)
    {
        return McpHit::FocusPort;
    }
    if CLI_INTEGRATIONS_AVAILABLE {
        // Row order is `McpCli::DISPLAY`; paint below walks the same array
        // with the same rect math, so a click always lands on the CLI that
        // was painted there.
        for (i, cli) in McpCli::DISPLAY.iter().enumerate() {
            if (cli_row_rect(content, i)).contains(scrolled) {
                return McpHit::ToggleCli(*cli);
            }
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
    paint_tab_intro(
        cx,
        theme,
        content,
        t_settings(ui, "settings.mcp.heroTitle"),
        Some(t_settings(ui, "settings.mcp.heroSubtitle")),
    );
    paint_server_row(cx, theme, settings, ui, content, now_ms);

    if CLI_INTEGRATIONS_AVAILABLE {
        let top = integrations_top(content);
        paint_section_header(
            cx,
            theme,
            content,
            top,
            Icon::Terminal,
            t_settings(ui, "settings.mcp.terminalIntegrations"),
        );
        let last = McpCli::DISPLAY.len() - 1;
        for (i, cli) in McpCli::DISPLAY.iter().enumerate() {
            let row = cli_row_rect(content, i);
            paint_row_label(cx, theme, row, cli.label(), None, SETTINGS_SWITCH_W + 16.0);
            paint_settings_switch(
                cx,
                theme,
                row_control_rect(row, SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
                // Toggle state is stored positionally in append-only
                // `McpCli::ALL` order; the displayed row is `DISPLAY` order.
                settings.mcp_cli_enabled[cli.index()],
            );
            if i != last {
                paint_row_hairline(cx, theme, row);
            }
        }
        paint_footnote(
            cx,
            theme,
            content,
            cli_row_rect(content, last).origin.y + ROW_HEIGHT,
            t_settings(ui, "settings.mcp.terminalFootnote"),
        );
    }

    paint_custom_config(cx, theme, settings, ui, content, now_ms);
}

/// Section header: glyph + title on the left, optional action on the
/// right. Shared shape between the integrations list and the custom
/// configuration block.
fn paint_section_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    content: Rect,
    y: f32,
    icon: Icon,
    title: &str,
) {
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            content.origin.x,
            y + (SECTION_HEADER_H - SECTION_ICON) / 2.0,
        ),
        SECTION_ICON,
        theme.muted_foreground,
        1.6,
    );
    paint_section_title(cx, theme, content.origin.x + SECTION_ICON + 8.0, y, title);
}

fn paint_server_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    let row = server_row_rect(content);
    // Host-managed (embed): the extension's proxy is alive by the time any
    // editor mounts — the row reads always-running at the host's port.
    let host_managed = settings.mcp_host_managed();
    let running = host_managed || settings.mcp_server.running;
    let status_text = if running {
        t_settings(ui, "settings.mcp.running")
    } else {
        t_settings(ui, "settings.mcp.stopped")
    };
    paint_row_label_above_status(
        cx,
        theme,
        row,
        t_settings(ui, "settings.mcp.server"),
        SERVER_CONTROLS_W,
    );
    // The status line carries a leading dot, matching the auto-update row
    // on the System tab, so "is this thing on" reads the same way in both
    // places. Drawn here rather than through `paint_row_label`'s
    // description slot, which has no room for the dot.
    paint_row_status_line(
        cx,
        row,
        status_text,
        if running {
            theme.status_success
        } else {
            theme.muted_foreground
        },
    );
    paint_row_hairline(cx, theme, row);

    let btn = server_button_rect(content);
    let port_label_text = t_settings(ui, "settings.mcp.port");
    let port_label_text = fit_text(cx, port_label_text, 80.0, 11.0);
    let port_label_w = measure_settings_text(cx, &port_label_text, 11.0);
    let port_field = port_field_rect(content);
    let mid_y = port_field.origin.y + port_field.size.y / 2.0;
    let port_label = TextLayout::single_run(
        &port_label_text,
        SETTINGS_FONT_FAMILY,
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &port_label,
        Point2D::new(port_field.origin.x - 8.0 - port_label_w, mid_y + 4.0),
    );
    let port_editable = !running && !host_managed;
    let focused = port_editable && matches!(settings.focus, Some(SettingsFocus::McpPort));
    let port_str = if focused {
        ui.settings_input.text().to_owned()
    } else if host_managed {
        settings
            .embed_mcp_port_text()
            .unwrap_or_default()
            .to_owned()
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
    let port_w = measure_settings_text(cx, &port_str, 12.0);
    let port_layout = TextLayout::single_run(
        &port_str,
        SETTINGS_FONT_FAMILY,
        12.0,
        (if port_editable {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let port_x = port_field.origin.x + (PORT_FIELD_W - port_w) / 2.0;
    let port_y = port_field.origin.y + PORT_FIELD_H / 2.0 + 5.0;
    if focused {
        paint_settings_input_view(
            cx,
            theme,
            ui,
            port_field,
            12.0,
            port_x - port_field.origin.x,
            port_y,
            now_ms,
            "",
        );
    } else {
        cx.backend
            .draw_text(&port_layout, Point2D::new(port_x, port_y));
    }

    // Host-managed: no start/stop affordance — the extension owns the
    // lifecycle (hit_test skips ToggleServer under the same gate).
    if host_managed {
        return;
    }
    let btn_bg = if running { theme.muted } else { theme.primary };
    let btn_fg = if running {
        theme.foreground
    } else {
        theme.primary_foreground
    };
    cx.backend.fill_round_rect(btn, 6.0, btn_bg);
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        btn,
        settings.hover_mcp_server_button,
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpServer,
        )),
    );
    if running {
        cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
    }
    let btn_label = if running {
        t_settings(ui, "settings.mcp.stop")
    } else {
        t_settings(ui, "settings.mcp.start")
    };
    let btn_label = fit_text(cx, btn_label, BTN_W - 8.0, 12.0);
    let btn_label_w = measure_settings_text(cx, &btn_label, 12.0);
    let lay = TextLayout::single_run(
        &btn_label,
        SETTINGS_FONT_FAMILY,
        12.0,
        (btn_fg).to_jian(),
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

fn paint_custom_config(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    now_ms: u64,
) {
    if !has_client_config(settings) {
        return;
    }
    let top = custom_config_top(content);
    paint_section_header(
        cx,
        theme,
        content,
        top,
        Icon::Settings,
        t_settings(ui, "settings.mcp.customConfigTitle"),
    );

    let copy = client_config_copy_button_rect(content);
    let copied = mcp_client_config_copy_feedback_active(settings, now_ms);
    cx.backend.fill_round_rect(copy, 6.0, theme.muted);
    cx.backend.stroke_round_rect(copy, 6.0, theme.border, 1.0);
    crate::widgets::button::paint_ghost_button_feedback(
        cx.backend,
        theme,
        copy,
        settings.hover_mcp_client_config_copy,
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::McpClientConfigCopy,
        )),
    );
    let (icon, icon_color) = if copied {
        (Icon::Check, theme.status_success)
    } else {
        (Icon::Copy, theme.foreground)
    };
    let copy_label = fit_text(
        cx,
        t_settings(ui, "settings.mcp.copyConfig"),
        COPY_BTN_W - COPY_BTN_ICON - 8.0 - 16.0,
        12.0,
    );
    let copy_label_w = measure_settings_text(cx, &copy_label, 12.0);
    let group_w = COPY_BTN_ICON + 8.0 + copy_label_w;
    let icon_x = copy.origin.x + (COPY_BTN_W - group_w) / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(icon_x, copy.origin.y + (COPY_BTN_H - COPY_BTN_ICON) / 2.0),
        COPY_BTN_ICON,
        icon_color,
        1.5,
    );
    let copy_layout = TextLayout::single_run(
        &copy_label,
        SETTINGS_FONT_FAMILY,
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &copy_layout,
        Point2D::new(
            icon_x + COPY_BTN_ICON + 8.0,
            copy.origin.y + COPY_BTN_H / 2.0 + 4.0,
        ),
    );

    let body_y = top + SECTION_HEADER_H;
    let desc = fit_text(
        cx,
        t_settings(ui, "settings.mcp.customConfigDesc"),
        content.size.x,
        12.0,
    );
    let desc_layout = TextLayout::single_run(
        &desc,
        SETTINGS_FONT_FAMILY,
        12.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&desc_layout, Point2D::new(content.origin.x, body_y + 14.0));

    let config = settings.mcp_client_config_display_text();
    // Drawn as a monospace run, so it must be fitted against monospace —
    // the family-blind default would under-report it just like the hero.
    let config = crate::util::ellipsize_to_width(&config, content.size.x, |s| {
        cx.backend.measure_text_family(s, 11.0, "monospace")
    });
    let config_lay = TextLayout::single_run(
        &config,
        "monospace",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&config_lay, Point2D::new(content.origin.x, body_y + 36.0));
}

fn mcp_client_config_copy_feedback_active(settings: &AgentSettings, now_ms: u64) -> bool {
    settings
        .mcp_client_config_copied_at_ms
        .map(|copied_at| now_ms.saturating_sub(copied_at) < CLIENT_CONFIG_COPY_FEEDBACK_MS)
        .unwrap_or(false)
}
