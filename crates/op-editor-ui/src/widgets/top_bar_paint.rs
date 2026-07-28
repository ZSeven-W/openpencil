//! `TopBar::paint_chrome` — the full top-bar composition pass.
//!
//! Split out of `top_bar.rs` to keep that file under the repo's
//! 800-line cap. `impl Widget for TopBar::paint` delegates straight
//! here. The geometry / consts / button-rect helpers + the small
//! free-fn painters live in `top_bar.rs` (re-exported `pub(super)`).

use crate::theme::Theme;
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
        if self.show_traffic_controls && !cfg!(target_os = "macos") {
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
        // File-scoped chrome (file-menu compound, Figma import, centered
        // file name + edited label + git-branch button) — hidden inside a
        // VS Code embed, where the workbench owns file identity. Includes
        // the two dividers that frame this group so hiding the buttons
        // doesn't leave orphan divider lines.
        if self.file_controls_visible() {
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
            // Import button (Figma / HTML menu).
            let figma_x = divider2_x + DIVIDER_W + DIVIDER_GAP;
            paint_import_button(
                cx,
                &self.theme,
                figma_x,
                center_y,
                self.is_hovered(TopBarButton::OpenImportMenu),
                self.is_pressed(TopBarButton::OpenImportMenu),
            );

            // ── Bounded centered file title ─────────────────────
            // File name, dirty marker, and Git button share one slot between
            // the import control and agent chip. Paint uses the same family
            // metrics as the text runs, so the title ends exactly at the
            // reserved Git gap instead of inheriting a conservative estimate.
            // Clipping remains a hard guard against platform-font differences.
            let title = self.title_layout(rect, |text, size| {
                cx.backend.measure_text_family(text, size, "system-ui")
            });
            if title.slot.size.x > 0.0 {
                cx.backend.save();
                cx.backend.clip_rect(title.slot);

                if !title.file_name.is_empty() {
                    let name = TextLayout::single_run(
                        &title.file_name,
                        "system-ui",
                        13.0,
                        (self.theme.foreground).to_jian(),
                        Point2D::new(0.0, 0.0),
                    );
                    cx.backend
                        .draw_text(&name, Point2D::new(title.file_x, center_y + 5.0));
                }

                if let Some(edited_x) = title.edited_x {
                    let edited = TextLayout::single_run(
                        self.label_edited,
                        "system-ui",
                        11.0,
                        (self.theme.muted_foreground).to_jian(),
                        Point2D::new(0.0, 0.0),
                    );
                    cx.backend
                        .draw_text(&edited, Point2D::new(edited_x, center_y + 4.0));
                }

                // Git-panel button just right of the file name (TS
                // GitButton). It may collapse in an exceptionally narrow
                // window so the dirty marker remains visible.
                if let Some(git_rect) = title.git_rect {
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

                cx.backend.restore();
            }
        }

        // ── Right cluster ──────────────────────────────────────
        // Right → left: Maximize (hidden in a VS Code embed) | Play
        // (native only) | Sun | Globe+Chevron. Globe is a wider compound
        // button (signals the dropdown affordance).
        let mut rx = rect.origin.x + rect.size.x - PAD;

        // Fullscreen — hidden inside a VS Code embed (the container's own
        // window chrome owns fullscreen). When hidden, Play/Sun/Globe
        // shift right to fill the vacated slot instead of leaving a gap.
        if self.fullscreen_button_visible() {
            rx -= ICON_BUTTON;
            paint_icon_button(
                cx,
                &self.theme,
                rx,
                center_y,
                Icon::Maximize,
                self.is_hovered(TopBarButton::ToggleFullscreen),
                self.is_pressed(TopBarButton::ToggleFullscreen),
            );
        }

        if self.preview_button_visible() {
            rx -= ICON_BUTTON;
            // Preview (Play) toggle — Square glyph while active (click →
            // stop), Play glyph while idle (click → enter preview).
            let preview_icon = if self.preview_active {
                Icon::Square
            } else {
                Icon::Play
            };
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

        // Theme toggle — Sun in dark mode (click → light); Moon in
        // light mode (click → dark).
        rx -= ICON_BUTTON;
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

        // i18n globe — plain icon button like its Sun/Maximize siblings
        // (the trailing chevron was dropped: it added noise without
        // information, the dropdown affordance is the click itself).
        paint_icon_button(
            cx,
            &self.theme,
            rx + (GLOBE_BUTTON_WIDTH - ICON_BUTTON) / 2.0,
            center_y,
            Icon::Globe,
            self.is_hovered(TopBarButton::ToggleLocale),
            self.is_pressed(TopBarButton::ToggleLocale),
        );
        // `rx` now points at the LEFT edge of the globe button.

        // User-avatar button — sits between the Globe and the agent
        // chip (TS layout spot: "between the agents chip and the
        // globe/theme icons"). Paints only when the host enabled the
        // runtime account gate (desktop with an auth backend).
        if self.account_button_visible {
            rx -= ICON_BUTTON;
            paint_account_button(
                cx,
                &self.theme,
                &self.account,
                rx,
                center_y,
                self.is_hovered(TopBarButton::OpenAccount),
                self.is_pressed(TopBarButton::OpenAccount),
            );
        }
        if self.collab.visible {
            let collab_rect = self.collaboration_chip_rect_estimated(rect);
            paint_collaboration_chip(
                cx,
                &self.theme,
                &self.collab,
                collab_rect,
                self.is_hovered(TopBarButton::OpenCollaboration),
                self.is_pressed(TopBarButton::OpenCollaboration),
            );
            rx = collab_rect.origin.x;
        }
        // `rx` now points at the LEFT edge of the chip's anchor —
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
        let icons_span = self.agent_icons_span();
        let text_w = cx.backend.measure_text(chip_text, 11.0);
        let chip_rect = self.agent_chip_rect(rect, text_w);
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
        let mut text_x = chip_rect.origin.x + 8.0 + icons_span;
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

        // Divider between the agent chip and the avatar button — groups
        // the status chip apart from the account/locale/theme/fullscreen
        // controls.
        paint_divider(cx, &self.theme, rx - DIVIDER_GAP - DIVIDER_W, center_y);
    }
}

fn paint_collaboration_chip(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    model: &crate::widgets::collab_ui::CollabTopBarModel,
    rect: Rect,
    hovered: bool,
    pressed: bool,
) {
    let foreground = crate::widgets::button::paint_ghost_button_feedback(
        cx.backend, theme, rect, hovered, pressed,
    );
    let tone = match model.tone {
        crate::widgets::collab_ui::CollabTopBarTone::Neutral => theme.muted_foreground,
        crate::widgets::collab_ui::CollabTopBarTone::Progress => theme.primary,
        crate::widgets::collab_ui::CollabTopBarTone::Connected => Color {
            r: 0.063,
            g: 0.725,
            b: 0.506,
            a: 1.0,
        },
        crate::widgets::collab_ui::CollabTopBarTone::Warning => Color {
            r: 0.961,
            g: 0.62,
            b: 0.043,
            a: 1.0,
        },
        crate::widgets::collab_ui::CollabTopBarTone::ReadOnly => theme.muted_foreground,
        crate::widgets::collab_ui::CollabTopBarTone::Ended => Color {
            r: 0.937,
            g: 0.267,
            b: 0.267,
            a: 1.0,
        },
    };
    let center_y = rect.origin.y + rect.size.y / 2.0;
    let mut x = rect.origin.x + 9.0;
    if model.avatars.is_empty() {
        draw_icon(
            cx.backend,
            Icon::Users,
            Point2D::new(x, center_y - ICON_SIZE / 2.0),
            ICON_SIZE,
            tone,
            1.4,
        );
        x += ICON_SIZE + 6.0;
    } else {
        for avatar in &model.avatars {
            let chip = Rect::xywh(
                x,
                center_y - COLLAB_AVATAR_CHIP / 2.0,
                COLLAB_AVATAR_CHIP,
                COLLAB_AVATAR_CHIP,
            );
            crate::widgets::collab_avatar_paint::paint_collab_avatar(
                cx,
                avatar,
                chip,
                8.0,
                chip.origin.y + 12.0,
            );
            cx.backend
                .stroke_round_rect(chip, COLLAB_AVATAR_CHIP / 2.0, theme.card, 1.0);
            x += COLLAB_AVATAR_CHIP - COLLAB_AVATAR_OVERLAP;
        }
        x += COLLAB_AVATAR_OVERLAP;
        if model.participant_overflow > 0 {
            let overflow = format!("+{}", model.participant_overflow);
            let overflow_layout = TextLayout::single_run(
                &overflow,
                "system-ui",
                9.0,
                theme.muted_foreground.to_jian(),
                Point2D::ZERO,
            );
            cx.backend
                .draw_text(&overflow_layout, Point2D::new(x, center_y + 3.0));
            x += cx.backend.measure_text(&overflow, 9.0) + 4.0;
        }
        x += 6.0;
    }

    cx.backend
        .fill_round_rect(Rect::xywh(x, center_y - 3.0, 6.0, 6.0), 3.0, tone);
    x += 10.0;
    cx.backend.save();
    cx.backend.clip_rect(rect);
    let label = TextLayout::single_run(
        &model.label,
        "system-ui",
        11.0,
        foreground.to_jian(),
        Point2D::ZERO,
    );
    cx.backend
        .draw_text(&label, Point2D::new(x, center_y + 4.0));
    cx.backend.restore();
}

/// User-avatar button: a generic outline glyph when signed out, or a
/// filled initial-letter circle when signed in. Shares the same
/// hover/press ghost background as its icon-button siblings (Sun /
/// Globe / Maximize) so it reads consistently in the chrome row.
pub(super) fn paint_account_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    account: &op_editor_core::AccountState,
    x: f32,
    center_y: f32,
    hovered: bool,
    pressed: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    let color = paint_hover_bg(cx, theme, button_rect, hovered, pressed);
    match account {
        op_editor_core::AccountState::Anonymous => {
            draw_icon(
                cx.backend,
                Icon::User,
                Point2D::new(
                    x + (ICON_BUTTON - ICON_SIZE) / 2.0,
                    glyph_top(center_y, ICON_SIZE),
                ),
                ICON_SIZE,
                color,
                1.4,
            );
        }
        op_editor_core::AccountState::SignedIn { .. } => {
            const AVATAR: f32 = 20.0;
            let avatar_rect = Rect {
                origin: Point2D::new(x + (ICON_BUTTON - AVATAR) / 2.0, center_y - AVATAR / 2.0),
                size: Point2D::new(AVATAR, AVATAR),
            };
            cx.backend.fill_oval(avatar_rect, theme.primary);
            let letter = account.initial().to_string();
            let letter_w = cx.backend.measure_text(&letter, 11.0);
            let label = TextLayout::single_run(
                &letter,
                "system-ui",
                11.0,
                (theme.primary_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &label,
                Point2D::new(
                    avatar_rect.origin.x + (AVATAR - letter_w) / 2.0,
                    center_y + 4.0,
                ),
            );
        }
    }
}

/// An icon button with hover/pressed background + foreground glyph.
/// (Lives here, next to its only caller `paint_chrome`, to keep
/// `top_bar.rs` under the 800-line cap.)
pub(super) fn paint_icon_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    x: f32,
    center_y: f32,
    icon: Icon,
    hovered: bool,
    pressed: bool,
) {
    let button_rect = Rect {
        origin: Point2D::new(x, center_y - ICON_BUTTON / 2.0),
        size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
    };
    jian_widgets::components::icon_button::IconButton {
        icon_paths: icon.paths(),
        hovered,
        pressed,
        active: false,
        enabled: true,
        icon_size: ICON_SIZE,
        stroke_width: 1.4,
    }
    .paint(
        cx.backend,
        button_rect,
        &crate::widgets::button::tokens_from_theme(theme),
    );
}
