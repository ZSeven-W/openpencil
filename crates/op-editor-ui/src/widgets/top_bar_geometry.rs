//! `TopBar` right-cluster + file-menu/git-button geometry helpers — split
//! out of `top_bar.rs` to keep that file under the repo's 800-line cap.
//! Paint (`top_bar_paint.rs`) and hit-test (`top_bar.rs::hit_test`) both
//! route through these so button rects can never drift between the two.

use super::top_bar_title::TopBarTitleLayout;
use crate::widgets::top_bar::*;
use crate::{Point2D, Rect};

impl TopBar {
    /// Returns the on-screen rect of the Globe-plus-chevron locale
    /// button. Used by the host to anchor the LocalePicker dropdown
    /// directly underneath when `Document.ui.locale_picker.open ==
    /// true`. The button itself is wider than a normal icon button
    /// so the chevron-down has room to render.
    /// Anchor rect for the file-menu dropdown overlay (folder +
    /// chevron compound). Host anchors the dropdown directly under
    /// this rect when `Document.ui.file_menu_open == true`.
    pub fn file_menu_rect(top_bar_rect: Rect, fullscreen: bool) -> Rect {
        // Mirror the paint layout: panel button │ divider │ file-menu.
        // The divider span (gap + width + gap) pushes the file-menu
        // right of the sidebar toggle — keep this anchor in sync so
        // the dropdown opens under the folder button, not left of it.
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        let file_menu_x = top_bar_rect.origin.x
            + PAD
            + Self::left_inset_for(fullscreen)
            + ICON_BUTTON
            + divider_span;
        Rect {
            origin: Point2D::new(file_menu_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    pub fn file_menu_rect_for(&self, top_bar_rect: Rect) -> Rect {
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        let file_menu_x =
            top_bar_rect.origin.x + PAD + self.left_inset() + ICON_BUTTON + divider_span;
        Rect {
            origin: Point2D::new(file_menu_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// Import button, right of the file menu. Canonical anchor shared by
    /// hit-test, paint, and the import dropdown so they cannot drift.
    pub fn import_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let divider_span = DIVIDER_GAP + DIVIDER_W + DIVIDER_GAP;
        let file_menu = self.file_menu_rect_for(top_bar_rect);
        Rect {
            origin: Point2D::new(
                file_menu.origin.x + FILE_MENU_BUTTON_WIDTH + divider_span,
                file_menu.origin.y,
            ),
            size: Point2D::new(FILE_MENU_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// Whether the Preview (Play) button paints / hit-tests. Gated only by
    /// the host capability (`PREVIEW_BUTTON_AVAILABLE`, desktop-only) —
    /// preview interaction graduated out of the experimental-features gate
    /// (widget-config and other experimental items stay gated separately;
    /// see `EditorUiState::agent_settings.experimental_features_enabled`).
    /// The right-cluster layout collapses when this is false, so paint,
    /// hit-test, and the globe-anchored locale picker all key off this one
    /// predicate.
    pub fn preview_button_visible(&self) -> bool {
        PREVIEW_BUTTON_AVAILABLE
    }

    /// File-scoped chrome (open menu, Figma import, centered file name):
    /// hidden inside a VS Code embed — the workbench owns file identity.
    pub(super) fn file_controls_visible(&self) -> bool {
        self.embed != op_editor_core::EmbedHost::VsCode
    }

    /// The Maximize toggle is meaningless inside an embed iframe.
    pub(super) fn fullscreen_button_visible(&self) -> bool {
        self.embed != op_editor_core::EmbedHost::VsCode
    }

    pub fn globe_rect(&self, top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        // Right-cluster layout (right → left): Maximize (hidden in a
        // VS Code embed) | Play (native only) | Sun | Globe. Icon buttons
        // are normal ICON_BUTTON wide; Globe is the wider
        // GLOBE_BUTTON_WIDTH so the chevron fits.
        let icon_count =
            1.0 + if self.fullscreen_button_visible() {
                1.0
            } else {
                0.0
            } + if self.preview_button_visible() {
                1.0
            } else {
                0.0
            };
        let globe_x = right - PAD - ICON_BUTTON * icon_count - GLOBE_BUTTON_WIDTH;
        Rect {
            origin: Point2D::new(globe_x, top_bar_rect.origin.y + 8.0),
            size: Point2D::new(GLOBE_BUTTON_WIDTH, ICON_BUTTON),
        }
    }

    /// User-avatar button — anchored directly left of the Globe button,
    /// between the locale/theme cluster and the agent-status chip's
    /// divider (TS parity spot: "between the agents chip and the
    /// globe/theme icons"). Derived from [`Self::globe_rect`] so the two
    /// can never drift apart.
    pub fn account_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let globe = self.globe_rect(top_bar_rect);
        Rect {
            origin: Point2D::new(globe.origin.x - ICON_BUTTON, globe.origin.y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        }
    }

    fn account_or_globe_anchor_x(&self, top_bar_rect: Rect) -> f32 {
        if self.account_button_visible {
            self.account_button_rect(top_bar_rect).origin.x
        } else {
            self.globe_rect(top_bar_rect).origin.x
        }
    }

    pub(super) fn collaboration_avatar_span(&self) -> f32 {
        let count = self.collab.avatars.len();
        if count == 0 {
            return 0.0;
        }
        COLLAB_AVATAR_CHIP
            + (count.saturating_sub(1) as f32) * (COLLAB_AVATAR_CHIP - COLLAB_AVATAR_OVERLAP)
    }

    pub(super) fn collaboration_chip_rect(&self, top_bar_rect: Rect, text_w: f32) -> Rect {
        if !self.collab.visible {
            return Rect::xywh(
                self.account_or_globe_anchor_x(top_bar_rect),
                top_bar_rect.origin.y,
                0.0,
                0.0,
            );
        }
        let avatars = self.collaboration_avatar_span();
        let leading = if avatars > 0.0 { avatars } else { ICON_SIZE };
        let avatar_gap = 6.0;
        let overflow_w = if self.collab.participant_overflow > 0 {
            // Compact `+N` suffix; exact glyph width is deliberately bounded
            // because the participant list itself is capped in core state.
            7.0 * format!("+{}", self.collab.participant_overflow)
                .chars()
                .count() as f32
                + 4.0
        } else {
            0.0
        };
        // 10 px after the leading cluster accounts for its trailing gap
        // plus the six-pixel phase dot before the label.
        let chip_w = 9.0 + leading + overflow_w + avatar_gap + 10.0 + text_w + 10.0;
        let anchor = self.account_or_globe_anchor_x(top_bar_rect);
        Rect {
            origin: Point2D::new(
                anchor - chip_w - DIVIDER_GAP,
                top_bar_rect.origin.y + (top_bar_rect.size.y - 26.0) / 2.0,
            ),
            size: Point2D::new(chip_w, 26.0),
        }
    }

    /// Geometry used by hit-test and popup anchoring before a paint backend
    /// can measure the localized label.
    pub fn collaboration_chip_rect_estimated(&self, top_bar_rect: Rect) -> Rect {
        self.collaboration_chip_rect(top_bar_rect, estimated_text_width(&self.collab.label, 11.0))
    }

    /// Left edge the agent chip's divider hangs off. When collaboration is
    /// available, its chip sits between the agent launcher and account;
    /// otherwise the old account/globe anchor remains unchanged.
    pub(super) fn chip_right_anchor_x(&self, top_bar_rect: Rect) -> f32 {
        if self.collab.visible {
            self.collaboration_chip_rect_estimated(top_bar_rect)
                .origin
                .x
        } else {
            self.account_or_globe_anchor_x(top_bar_rect)
        }
    }

    /// Agent-chip bounds for an already measured status-label width.
    ///
    /// Paint, hit-test, and the center-title slot all share this geometry so
    /// a long file name cannot cross underneath the chip.
    pub(super) fn agent_chip_rect(&self, top_bar_rect: Rect, text_w: f32) -> Rect {
        let dot_w = if self.chip_status_text().is_some() {
            6.0 + 6.0
        } else {
            0.0
        };
        let chip_w = 8.0 + self.agent_icons_span() + dot_w + text_w + 12.0;
        Rect {
            origin: Point2D::new(
                self.chip_right_anchor_x(top_bar_rect) - chip_w - (DIVIDER_GAP * 2.0 + DIVIDER_W),
                top_bar_rect.origin.y + (top_bar_rect.size.y - 26.0) / 2.0,
            ),
            size: Point2D::new(chip_w, 26.0),
        }
    }

    /// Play / Stop toggle button — second from the right (just left of
    /// Maximize), or rightmost when Maximize is hidden in a VS Code embed.
    /// Shared by paint + hit-test so they can't drift.
    pub(super) fn preview_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        let icon_y = top_bar_rect.origin.y + 8.0;
        let fullscreen_slot = if self.fullscreen_button_visible() {
            1.0
        } else {
            0.0
        };
        Rect {
            origin: Point2D::new(right - PAD - ICON_BUTTON * (fullscreen_slot + 1.0), icon_y),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        }
    }

    /// Theme toggle button. Its x-position shifts right in the web/wasm build
    /// where the Preview button is hidden, and in a VS Code embed where the
    /// Maximize button is hidden.
    pub(super) fn theme_button_rect(&self, top_bar_rect: Rect) -> Rect {
        let right = top_bar_rect.origin.x + top_bar_rect.size.x;
        let right_icons = if self.fullscreen_button_visible() {
            1.0
        } else {
            0.0
        } + if self.preview_button_visible() {
            1.0
        } else {
            0.0
        };
        Rect {
            origin: Point2D::new(
                right - PAD - ICON_BUTTON * (right_icons + 1.0),
                top_bar_rect.origin.y + 8.0,
            ),
            size: Point2D::new(ICON_BUTTON, ICON_BUTTON),
        }
    }

    /// Git-panel toggle button — sits just right of the centred file
    /// name. Width holds the branch glyph plus an optional branch
    /// label. Shared by paint + hit-test so they can't drift.
    pub(super) fn git_button_rect(&self, top_bar_rect: Rect) -> Rect {
        self.title_layout_estimated(top_bar_rect)
            .git_rect
            .unwrap_or(Rect {
                origin: Point2D::new(top_bar_rect.origin.x, top_bar_rect.origin.y),
                size: Point2D::new(0.0, 0.0),
            })
    }

    pub(super) fn git_icon_left(git_button: Rect) -> f32 {
        git_button.origin.x + GIT_BUTTON_PAD_X
    }

    /// Center-x of the Git-panel toggle button when it is shown
    /// (desktop only — see `GIT_BUTTON_AVAILABLE`). The floating Git
    /// panel anchors its caret here so it reads as a popover hanging
    /// off the button (TS parity); `None` when the button is hidden.
    pub fn git_button_center_x(&self, top_bar_rect: Rect) -> Option<f32> {
        if !GIT_BUTTON_AVAILABLE || !self.file_controls_visible() {
            return None;
        }
        let r = self.git_button_rect(top_bar_rect);
        (r.size.x > 0.0).then_some(r.origin.x + r.size.x / 2.0)
    }

    /// Deterministic title geometry shared by paint, hit-test, and popup
    /// anchoring. The estimate is intentionally conservative; paint clips to
    /// the returned slot as a final guard against platform font differences.
    pub(super) fn title_layout_estimated(&self, top_bar_rect: Rect) -> TopBarTitleLayout {
        self.title_layout(top_bar_rect, estimated_text_width)
    }
}

fn estimated_text_width(text: &str, font_size: f32) -> f32 {
    text.chars()
        .map(|c| {
            if c.is_ascii() {
                font_size * 0.68
            } else {
                font_size
            }
        })
        .sum()
}
