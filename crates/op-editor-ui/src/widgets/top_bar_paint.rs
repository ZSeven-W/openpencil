//! `TopBar::paint_chrome` — the full top-bar composition pass.
//!
//! Split out of `top_bar.rs` to keep that file under the repo's
//! 800-line cap. `impl Widget for TopBar::paint` delegates straight
//! here. The geometry / consts / button-rect helpers + the small
//! free-fn painters live in `top_bar.rs` (re-exported `pub(super)`).

use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::top_bar::*;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::TopBarButton;

impl TopBar {
    pub(super) fn paint_chrome(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, self.theme.background);
        // Bottom hairline.
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + rect.size.y - 1.0),
                size: Point2D::new(rect.size.x, 1.0),
            },
            self.theme.border,
        );

        let center_y = rect.origin.y + rect.size.y / 2.0;
        // ── Window-control dots ────────────────────────────────
        // macOS keeps the *native* traffic-light buttons (the
        // transparent title bar preserves them), so the custom
        // dots paint only on Windows / Linux — whose
        // `decorations(false)` window ships none. The desktop
        // runner wires custom-dot clicks via `window_control_at`.
        if !cfg!(target_os = "macos") {
            let traffic = [
                Color {
                    r: 1.0,
                    g: 0.373,
                    b: 0.341,
                    a: 1.0,
                }, // #FF5F57
                Color {
                    r: 0.996,
                    g: 0.737,
                    b: 0.18,
                    a: 1.0,
                }, // #FEBC2E
                Color {
                    r: 0.157,
                    g: 0.784,
                    b: 0.251,
                    a: 1.0,
                }, // #28C840
            ];
            let first_dot_cx = rect.origin.x + PAD + TRAFFIC_DOT / 2.0;
            for (i, color) in traffic.into_iter().enumerate() {
                let dot_cx = first_dot_cx + i as f32 * TRAFFIC_STEP;
                cx.backend.fill_oval(
                    Rect {
                        origin: Point2D::new(
                            dot_cx - TRAFFIC_DOT / 2.0,
                            center_y - TRAFFIC_DOT / 2.0,
                        ),
                        size: Point2D::new(TRAFFIC_DOT, TRAFFIC_DOT),
                    },
                    color,
                );
                // Hovering the cluster reveals each dot's glyph
                // (macOS-style): ✕ close, − minimise, + maximise.
                if self.traffic_hover {
                    let glyph = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.55,
                    };
                    let r = 3.0_f32;
                    let lw = 1.3_f32;
                    match i {
                        0 => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y - r),
                                Point2D::new(dot_cx + r, center_y + r),
                                glyph,
                                lw,
                            );
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y + r),
                                Point2D::new(dot_cx + r, center_y - r),
                                glyph,
                                lw,
                            );
                        }
                        1 => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y),
                                Point2D::new(dot_cx + r, center_y),
                                glyph,
                                lw,
                            );
                        }
                        _ => {
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx - r, center_y),
                                Point2D::new(dot_cx + r, center_y),
                                glyph,
                                lw,
                            );
                            cx.backend.stroke_line(
                                Point2D::new(dot_cx, center_y - r),
                                Point2D::new(dot_cx, center_y + r),
                                glyph,
                                lw,
                            );
                        }
                    }
                }
            }
        }
        // ── Left cluster ───────────────────────────────────────
        // sidebar toggle │ file-menu │ Figma — each group split by a
        // TS-style 1×14 divider (4 px gap each side).
        let panel_left_x = rect.origin.x + PAD + self.left_inset();
        paint_icon_button(
            cx,
            &self.theme,
            panel_left_x,
            center_y,
            Icon::PanelLeft,
            self.is_hovered(TopBarButton::ToggleSidebar),
            self.is_pressed(TopBarButton::ToggleSidebar),
        );
        // Divider between the sidebar toggle and the file-menu.
        let divider1_x = panel_left_x + ICON_BUTTON + DIVIDER_GAP;
        paint_divider(cx, &self.theme, divider1_x, center_y);
        // File-menu compound: folder + tight chevron in one button.
        let file_menu_x = divider1_x + DIVIDER_W + DIVIDER_GAP;
        paint_file_menu_button(
            cx,
            &self.theme,
            file_menu_x,
            center_y,
            self.is_hovered(TopBarButton::ToggleFileMenu),
            self.is_pressed(TopBarButton::ToggleFileMenu),
        );
        // Divider before the Figma import affordance.
        let divider2_x = file_menu_x + FILE_MENU_BUTTON_WIDTH + DIVIDER_GAP;
        paint_divider(cx, &self.theme, divider2_x, center_y);
        // Figma import button.
        let figma_x = divider2_x + DIVIDER_W + DIVIDER_GAP;
        paint_figma_button(
            cx,
            &self.theme,
            figma_x,
            center_y,
            self.is_hovered(TopBarButton::OpenFigmaImport),
            self.is_pressed(TopBarButton::OpenFigmaImport),
        );

        // ── Centered file name ─────────────────────────────────
        let name = TextLayout::single_run(
            &self.file_name,
            "system-ui",
            13.0,
            (self.theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        // Approximate text width; skia textlayout would tell us the
        // exact pixel width but for Step 4 we don't pull that in.
        let approx_w = self.file_name.chars().count() as f32 * 9.0;
        cx.backend.draw_text(
            &name,
            Point2D::new(
                rect.origin.x + (rect.size.x - approx_w) / 2.0,
                center_y + 5.0,
            ),
        );

        // Git-panel button just right of the file name (TS GitButton):
        // a branch glyph + optional branch name. Always shown on
        // desktop — a click toggles the git panel (which offers `init`
        // when the doc isn't yet in a repo). Compiled out on web,
        // which has no git backend to paint a panel.
        if GIT_BUTTON_AVAILABLE {
            let git_rect = self.git_button_rect(rect);
            let git_color = paint_hover_bg(
                cx,
                &self.theme,
                git_rect,
                self.is_hovered(TopBarButton::ToggleGitPanel),
                self.is_pressed(TopBarButton::ToggleGitPanel),
            );
            draw_icon(
                cx.backend,
                Icon::GitBranch,
                Point2D::new(
                    Self::git_icon_left(git_rect),
                    glyph_top(center_y, ICON_SIZE),
                ),
                ICON_SIZE,
                git_color,
                1.4,
            );
            if let Some(branch) = self.git_branch.as_deref() {
                let label = TextLayout::single_run(
                    branch,
                    "system-ui",
                    11.0,
                    (git_color).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        Self::git_icon_left(git_rect) + ICON_SIZE + 6.0,
                        center_y + 4.0,
                    ),
                );
            }
        }

        // ── Right cluster ──────────────────────────────────────
        // Right → left: Maximize | Play | Sun | Globe+Chevron. Globe is
        // a wider compound button (signals the dropdown affordance).
        let mut rx = rect.origin.x + rect.size.x - PAD - ICON_BUTTON;

        // Fullscreen.
        paint_icon_button(
            cx,
            &self.theme,
            rx,
            center_y,
            Icon::Maximize,
            self.is_hovered(TopBarButton::ToggleFullscreen),
            self.is_pressed(TopBarButton::ToggleFullscreen),
        );
        rx -= ICON_BUTTON;

        // Preview (Play) toggle — Square glyph while active (click →
        // stop), Play glyph while idle (click → enter preview).
        let preview_icon = if self.preview_active {
            Icon::Square
        } else {
            Icon::Play
        };
        if cfg!(target_arch = "wasm32") {
            // Preview runs the jian runtime, which the web build can't
            // host yet — paint the Play button disabled (and it's not
            // hit-tested) so it reads as unavailable, not a dead toggle.
            paint_icon_button_disabled(cx, &self.theme, rx, center_y, preview_icon);
        } else {
            paint_icon_button(
                cx,
                &self.theme,
                rx,
                center_y,
                preview_icon,
                self.is_hovered(TopBarButton::TogglePreview),
                self.is_pressed(TopBarButton::TogglePreview),
            );
        }
        rx -= ICON_BUTTON;

        // Theme toggle — Sun in dark mode (click → light); Moon in
        // light mode (click → dark).
        let theme_icon = match self.theme_mode {
            op_editor_core::ThemeMode::Dark => Icon::Sun,
            op_editor_core::ThemeMode::Light => Icon::Moon,
        };
        paint_icon_button(
            cx,
            &self.theme,
            rx,
            center_y,
            theme_icon,
            self.is_hovered(TopBarButton::ToggleTheme),
            self.is_pressed(TopBarButton::ToggleTheme),
        );
        rx -= GLOBE_BUTTON_WIDTH;

        // i18n globe + chevron-down (single hit-target).
        let globe_button = Rect {
            origin: Point2D::new(rx, center_y - ICON_BUTTON / 2.0),
            size: Point2D::new(GLOBE_BUTTON_WIDTH, ICON_BUTTON),
        };
        let globe_color = paint_hover_bg(
            cx,
            &self.theme,
            globe_button,
            self.is_hovered(TopBarButton::ToggleLocale),
            self.is_pressed(TopBarButton::ToggleLocale),
        );
        let locale_glyph_x = Self::locale_glyph_left(globe_button);
        // Globe glyph at the left half.
        draw_icon(
            cx.backend,
            Icon::Globe,
            Point2D::new(locale_glyph_x, center_y - ICON_SIZE / 2.0),
            ICON_SIZE,
            globe_color,
            1.4,
        );
        // Chevron-down at the right side, smaller.
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(
                locale_glyph_x + ICON_SIZE + COMPOUND_GLYPH_GAP,
                center_y - CHEVRON_SIZE / 2.0,
            ),
            CHEVRON_SIZE,
            globe_color,
            1.4,
        );
        // `rx` now points at the LEFT edge of the globe button —
        // the chip anchors immediately to its left (small gap).

        // Agent chip — two states:
        //   - empty (agent_count == 0): LayoutGrid icon + an
        //     "Agents and MCP" label. Affordance for "set up agents/MCP".
        //   - active (≥ 1): one brand icon per connected provider +
        //     a green dot + "N agent" text.
        // Anchored just left of the globe icon button (+small gap).
        let status_text = self.chip_status_text();
        let show_dot = status_text.is_some();
        let chip_text: &str = status_text.as_deref().unwrap_or(self.label_agents_and_mcp);
        let dot_w = if show_dot { 6.0 + 6.0 } else { 0.0 };
        let icons_w = self.agent_icons_width();
        let text_w = cx.backend.measure_text(chip_text, 11.0);
        let chip_w = 8.0 + icons_w + 6.0 + dot_w + text_w + 12.0;
        // Leave room for the chip↔globe divider (4 px gap + 1 px + 4 px).
        let chip_rect = Rect {
            origin: Point2D::new(
                rx - chip_w - (DIVIDER_GAP * 2.0 + DIVIDER_W),
                center_y - 13.0,
            ),
            size: Point2D::new(chip_w, 26.0),
        };
        // Hover wash behind the whole chip (TS `hover:bg-accent`).
        let _ = crate::widgets::button::paint_ghost_button_feedback(
            cx.backend,
            &self.theme,
            chip_rect,
            self.is_hovered(TopBarButton::OpenAgentSettings),
            self.is_pressed(TopBarButton::OpenAgentSettings),
        );
        // Leading icons (no border ring — TS empty-state chip has no
        // outline). The empty state shows the single LayoutGrid
        // set-up affordance; the active chip stacks one brand logo
        // per connected provider so the user sees *which* agents
        // are on.
        let icons_y = glyph_top(center_y, ICON_SIZE);
        if self.agent_count == 0 {
            draw_icon(
                cx.backend,
                Icon::LayoutGrid,
                Point2D::new(chip_rect.origin.x + 8.0, icons_y),
                ICON_SIZE,
                self.theme.muted_foreground,
                1.4,
            );
        } else {
            // Stacked brand chips — one rounded `bg-foreground/10` square per
            // connected provider, overlapped with a card-coloured ring so each
            // stays distinct (TS `top-bar.tsx`: `-space-x-1.5 ring-1 ring-card`).
            let chip_top = center_y - AGENT_ICON_CHIP / 2.0;
            let step = AGENT_ICON_CHIP - AGENT_ICON_OVERLAP;
            let logo_inset = (AGENT_ICON_CHIP - AGENT_ICON_LOGO) / 2.0;
            let mut ix = chip_rect.origin.x + 8.0;
            for (i, provider) in op_editor_core::AgentProvider::ALL.iter().enumerate() {
                if !self.connected[i] {
                    continue;
                }
                let chip_box = Rect {
                    origin: Point2D::new(ix, chip_top),
                    size: Point2D::new(AGENT_ICON_CHIP, AGENT_ICON_CHIP),
                };
                cx.backend.fill_round_rect(
                    chip_box,
                    5.0,
                    Color {
                        a: 0.1,
                        ..self.theme.foreground
                    },
                );
                cx.backend
                    .stroke_round_rect(chip_box, 5.0, self.theme.card, 1.0);
                crate::widgets::ai_chat_model_picker::paint_provider_logo(
                    cx,
                    *provider,
                    Point2D::new(ix + logo_inset, center_y - AGENT_ICON_LOGO / 2.0),
                    AGENT_ICON_LOGO,
                    self.theme.foreground,
                );
                ix += step;
            }
        }
        let mut text_x = chip_rect.origin.x + 8.0 + icons_w + 6.0;
        if show_dot {
            // emerald-500 (#10b981) — matches the TS status dot.
            let dot_color = Color {
                r: 0.063,
                g: 0.725,
                b: 0.506,
                a: 1.0,
            };
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(text_x, chip_rect.origin.y + chip_rect.size.y / 2.0 - 3.0),
                    size: Point2D::new(6.0, 6.0),
                },
                3.0,
                dot_color,
            );
            text_x += 6.0 + 6.0;
        }
        let chip_label = TextLayout::single_run(
            chip_text,
            "system-ui",
            11.0,
            (self.theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        // 11 px text centred on the bar's center line (ascent ≈ 8 px).
        cx.backend
            .draw_text(&chip_label, Point2D::new(text_x, center_y + 4.0));

        // Divider between the agent chip and the globe button — groups
        // the status chip apart from the locale/theme/fullscreen controls.
        paint_divider(
            cx,
            &self.theme,
            globe_button.origin.x - DIVIDER_GAP - DIVIDER_W,
            center_y,
        );
    }
}
