//! Paint and hit-test surface for the collaboration popover.
//!
//! The panel consumes only the sanitized models from `collab_ui`; network and
//! ticket data never enter this widget. The single deliberate exception is the
//! guest's owner-confirmation screen, whose entire purpose is to show the
//! verified account subject and device id so a human can decide whether to
//! join — see `collab_panel_owner_confirm`.

use crate::theme::Theme;
use crate::widgets::collab_ui::{
    CollabAdmissionRequestModel, CollabPanelActionModel, CollabPanelModel, CollabPanelScreen,
};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};
use op_editor_core::{CollabPanelHover, CollabUiAction, EditorUiState};

#[path = "collab_panel_paint.rs"]
mod paint;
use paint::{paint_button, paint_participant, paint_text};
#[path = "collab_panel_interaction.rs"]
mod interaction;
#[path = "collab_panel_owner_confirm.rs"]
mod owner_confirm;
use owner_confirm::{CONFIRM_OWNER_HEAD_HEIGHT, CONFIRM_OWNER_ROW_HEIGHT};

pub const COLLAB_PANEL_WIDTH: f32 = 340.0;
const HEADER_HEIGHT: f32 = 44.0;
const PAD: f32 = 14.0;
const ROW_HEIGHT: f32 = 34.0;
const INPUT_HEIGHT: f32 = 32.0;
const ACTION_HEIGHT: f32 = 32.0;
const ACTION_GAP: f32 = 8.0;
const NOTICE_HEIGHT: f32 = 42.0;
const CLEAR_BUTTON_SIZE: f32 = 22.0;
const CONNECTION_PATH_HEIGHT: f32 = 28.0;
const INVITE_HEIGHT: f32 = 48.0;
const SHARE_ENDPOINT_HEIGHT: f32 = 38.0;
const ADMISSION_HEIGHT: f32 = 62.0;
const ADMISSION_ACTION_HEIGHT: f32 = 28.0;
const REGION_OPTION_HEIGHT: f32 = 28.0;
const REGION_SECTION_HEIGHT: f32 = 62.0;
const MAX_VISIBLE_PARTICIPANTS: usize = 8;
const MAX_VISIBLE_ENDPOINTS: usize = 6;

#[derive(Clone, PartialEq, Eq)]
pub enum CollabPanelHit {
    Close,
    FocusJoinAddress,
    ClearJoinAddress,
    OpenSignIn,
    CopyInvite(String),
    CopyShareEndpoint(String),
    Action(CollabUiAction),
    Inside,
}

impl std::fmt::Debug for CollabPanelHit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Close => formatter.write_str("Close"),
            Self::FocusJoinAddress => formatter.write_str("FocusJoinAddress"),
            Self::ClearJoinAddress => formatter.write_str("ClearJoinAddress"),
            Self::OpenSignIn => formatter.write_str("OpenSignIn"),
            Self::CopyInvite(_) => formatter.write_str("CopyInvite([REDACTED])"),
            Self::CopyShareEndpoint(_) => formatter.write_str("CopyShareEndpoint([REDACTED])"),
            Self::Action(CollabUiAction::JoinAddress { .. }) => {
                formatter.write_str("Action(JoinAddress([REDACTED]))")
            }
            Self::Action(CollabUiAction::JoinDiscovered { .. }) => {
                formatter.write_str("Action(JoinDiscovered([REDACTED]))")
            }
            Self::Action(action) => formatter.debug_tuple("Action").field(action).finish(),
            Self::Inside => formatter.write_str("Inside"),
        }
    }
}

pub struct CollabPanel<'a> {
    id: WidgetId,
    ui: &'a EditorUiState,
    model: CollabPanelModel,
    theme: Theme,
    /// Frame clock for the join field's caret blink. Hit-test-only callers
    /// construct with 0 — geometry never depends on it.
    now_ms: u64,
}

impl<'a> CollabPanel<'a> {
    pub fn for_editor_ui(ui: &'a EditorUiState) -> Option<Self> {
        Self::for_editor_ui_at(ui, 0)
    }

    /// Build with a frame clock so the join field's caret blinks.
    pub fn for_editor_ui_at(ui: &'a EditorUiState, now_ms: u64) -> Option<Self> {
        if !ui.collab.panel.open {
            return None;
        }
        Some(Self {
            id: WidgetId::new(5650),
            ui,
            model: CollabPanelModel::for_editor_ui(ui),
            theme: theme_for(ui),
            now_ms,
        })
    }
}

impl Widget for CollabPanel<'_> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, COLLAB_PANEL_WIDTH, self.panel_height()),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_round_rect(rect, 10.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 10.0, self.theme.border, 1.0);
        paint_text(
            cx,
            &self.model.title,
            14.0,
            self.theme.foreground,
            Point2D::new(rect.origin.x + PAD, rect.origin.y + 27.0),
            600,
        );
        let close_hovered = self.ui.collab.panel.hover == Some(CollabPanelHover::Close);
        if close_hovered {
            cx.backend
                .fill_round_rect(self.close_rect(rect), 6.0, self.theme.button_hover);
        }
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                self.close_rect(rect).origin.x + (self.close_rect(rect).size.x - 16.0) / 2.0,
                self.close_rect(rect).origin.y + (self.close_rect(rect).size.y - 16.0) / 2.0,
            ),
            16.0,
            if close_hovered {
                self.theme.foreground
            } else {
                self.theme.muted_foreground
            },
            1.4,
        );
        cx.backend.fill_rect(
            Rect::xywh(
                rect.origin.x,
                rect.origin.y + HEADER_HEIGHT - 1.0,
                rect.size.x,
                1.0,
            ),
            self.theme.border,
        );

        // Keep the bottom action row reachable on a short viewport without
        // allowing notices, inputs, or session rows to paint through it.
        cx.backend.save();
        cx.backend.clip_rect(self.body_clip_rect(rect));

        let mut body_top = rect.origin.y + HEADER_HEIGHT;
        if let Some(notice) = self.model.notice.as_deref() {
            let notice_rect = Rect::xywh(
                rect.origin.x + PAD,
                body_top + 6.0,
                rect.size.x - PAD * 2.0,
                NOTICE_HEIGHT,
            );
            cx.backend
                .fill_round_rect(notice_rect, 7.0, self.theme.primary.with_alpha(0.10));
            self.paint_notice_text(cx, notice, notice_rect);
            body_top += NOTICE_HEIGHT + 8.0;
        }

        match &self.model.screen {
            CollabPanelScreen::Unavailable => {
                self.paint_message(cx, rect, body_top, "collab.topbar.unavailable");
            }
            CollabPanelScreen::SignInRequired => {
                self.paint_message(cx, rect, body_top, "collab.join.signInRequired");
                paint_button(
                    cx,
                    &self.theme,
                    self.sign_in_rect(rect, body_top),
                    op_i18n::translate(self.ui.effective_locale(), "account.signInWithBrowser"),
                    true,
                    true,
                    self.ui.collab.panel.hover == Some(CollabPanelHover::OpenSignIn),
                );
            }
            CollabPanelScreen::Home => {
                self.paint_message(cx, rect, body_top, "collab.home.hint");
            }
            CollabPanelScreen::Create => {
                self.paint_message(cx, rect, body_top, "collab.create.choose");
                paint_text(
                    cx,
                    op_i18n::translate(self.ui.effective_locale(), "collab.session.region"),
                    11.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 56.0),
                    500,
                );
                let selected = self.ui.collab.panel.relay_region;
                for (button, region) in self.region_option_rects(rect, body_top) {
                    paint_button(
                        cx,
                        &self.theme,
                        button,
                        op_i18n::translate(self.ui.effective_locale(), region.i18n_key()),
                        region == selected,
                        true,
                        self.ui.collab.panel.hover
                            == Some(op_editor_core::CollabPanelHover::Region(region)),
                    );
                }
            }
            CollabPanelScreen::Progress { message } => {
                draw_icon(
                    cx.backend,
                    Icon::RefreshCw,
                    Point2D::new(rect.origin.x + PAD, body_top + 18.0),
                    16.0,
                    self.theme.primary,
                    1.5,
                );
                paint_text(
                    cx,
                    message,
                    12.0,
                    self.theme.foreground,
                    Point2D::new(rect.origin.x + PAD + 26.0, body_top + 31.0),
                    400,
                );
            }
            CollabPanelScreen::ConfirmOwner(confirm) => {
                self.paint_owner_confirmation(cx, rect, body_top, confirm);
            }
            CollabPanelScreen::Join {
                address: _,
                discovered,
            } => {
                paint_text(
                    cx,
                    op_i18n::translate(self.ui.effective_locale(), "collab.join.code"),
                    11.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 16.0),
                    400,
                );
                let input = self.address_rect(rect, body_top + 22.0);
                cx.backend.fill_round_rect(input, 6.0, self.theme.input);
                let input_hovered = matches!(
                    self.ui.collab.panel.hover,
                    Some(CollabPanelHover::JoinAddress | CollabPanelHover::ClearJoinAddress)
                );
                if input_hovered && !self.ui.collab.panel.join_address_focused {
                    cx.backend
                        .fill_round_rect(input, 6.0, self.theme.button_hover);
                }
                cx.backend.stroke_round_rect(
                    input,
                    6.0,
                    if self.ui.collab.panel.join_address_focused {
                        self.theme.ring
                    } else if input_hovered {
                        self.theme.muted_foreground.with_alpha(0.55)
                    } else {
                        self.theme.border
                    },
                    1.0,
                );
                let clear = self.clear_join_rect(rect, body_top + 22.0);
                let text_inset = if clear.is_some() {
                    CLEAR_BUTTON_SIZE + 5.0
                } else {
                    0.0
                };
                // Value, placeholder, selection highlight, and blinking caret
                // all render through the unified text-input view.
                let view_rect = Rect::xywh(
                    input.origin.x,
                    input.origin.y,
                    input.size.x - text_inset,
                    input.size.y,
                );
                crate::widgets::property_panel_text_input::paint_text_input_view(
                    cx,
                    &self.theme,
                    &self.ui.collab.panel.join_input,
                    view_rect,
                    12.0,
                    9.0,
                    input.origin.y + 21.0,
                    self.now_ms,
                    op_i18n::translate(self.ui.effective_locale(), "collab.join.codePlaceholder"),
                    self.ui.collab.panel.join_address_focused,
                );
                if let Some(clear) = clear {
                    if self.ui.collab.panel.hover == Some(CollabPanelHover::ClearJoinAddress) {
                        cx.backend
                            .fill_round_rect(clear, 6.0, self.theme.button_hover);
                    }
                    let icon_size = 12.0;
                    draw_icon(
                        cx.backend,
                        Icon::Close,
                        Point2D::new(
                            clear.origin.x + (clear.size.x - icon_size) / 2.0,
                            clear.origin.y + (clear.size.y - icon_size) / 2.0,
                        ),
                        icon_size,
                        self.theme.muted_foreground,
                        1.5,
                    );
                }
                paint_text(
                    cx,
                    op_i18n::translate(self.ui.effective_locale(), "collab.join.publicHint"),
                    10.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 72.0),
                    400,
                );
                paint_text(
                    cx,
                    op_i18n::translate(self.ui.effective_locale(), "collab.join.nearby"),
                    10.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 97.0),
                    500,
                );
                let first_y = body_top + 106.0;
                for (index, endpoint) in discovered.iter().take(MAX_VISIBLE_ENDPOINTS).enumerate() {
                    let y = first_y + index as f32 * ROW_HEIGHT;
                    if endpoint.compatible
                        && self.ui.collab.panel.hover == Some(CollabPanelHover::Discovered(index))
                    {
                        cx.backend.fill_round_rect(
                            self.discovered_rect(rect, first_y, index),
                            6.0,
                            self.theme.button_hover,
                        );
                    }
                    draw_icon(
                        cx.backend,
                        Icon::Users,
                        Point2D::new(rect.origin.x + PAD, y + 9.0),
                        15.0,
                        if endpoint.compatible {
                            self.theme.primary
                        } else {
                            self.theme.muted_foreground
                        },
                        1.4,
                    );
                    paint_text(
                        cx,
                        &endpoint.endpoint,
                        12.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD + 24.0, y + 22.0),
                        400,
                    );
                    if !endpoint.compatible {
                        paint_text(
                            cx,
                            op_i18n::translate(
                                self.ui.effective_locale(),
                                "collab.join.incompatible",
                            ),
                            9.0,
                            self.theme.muted_foreground,
                            Point2D::new(rect.origin.x + rect.size.x - 130.0, y + 21.0),
                            400,
                        );
                    }
                }
                if discovered.is_empty() {
                    let key = if self.ui.collab.phase
                        == op_editor_core::CollabConnectionPhase::Discovering
                    {
                        "collab.join.discovering"
                    } else {
                        "collab.join.noSessions"
                    };
                    paint_text(
                        cx,
                        op_i18n::translate(self.ui.effective_locale(), key),
                        11.0,
                        self.theme.muted_foreground,
                        Point2D::new(rect.origin.x + PAD, first_y + 22.0),
                        400,
                    );
                }
            }
            CollabPanelScreen::Session {
                session_name,
                role_label: session_role,
                invite,
                connection,
                share_endpoint,
                participants,
                pending,
                admission_request,
            } => {
                paint_text(
                    cx,
                    session_name,
                    13.0,
                    self.theme.foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 21.0),
                    600,
                );
                paint_text(
                    cx,
                    session_role,
                    11.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 40.0),
                    400,
                );
                if *pending {
                    paint_text(
                        cx,
                        op_i18n::translate(self.ui.effective_locale(), "collab.session.pending"),
                        10.0,
                        self.theme.primary,
                        Point2D::new(rect.origin.x + 90.0, body_top + 40.0),
                        400,
                    );
                }
                let connection_offset = if let Some(connection) = connection {
                    draw_icon(
                        cx.backend,
                        Icon::Globe,
                        Point2D::new(rect.origin.x + PAD, body_top + 64.0),
                        14.0,
                        self.theme.primary,
                        1.4,
                    );
                    let label =
                        crate::widgets::collab_ui::connection_path_label(self.ui, *connection);
                    paint_text(
                        cx,
                        &label,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD + 21.0, body_top + 76.0),
                        500,
                    );
                    CONNECTION_PATH_HEIGHT
                } else {
                    0.0
                };
                let invite_offset = if let Some(invite) = invite {
                    let top = body_top + 58.0 + connection_offset;
                    let card = Rect::xywh(
                        rect.origin.x + PAD,
                        top + 2.0,
                        rect.size.x - PAD * 2.0,
                        INVITE_HEIGHT - 4.0,
                    );
                    cx.backend
                        .fill_round_rect(card, 7.0, self.theme.primary.with_alpha(0.08));
                    paint_text(
                        cx,
                        op_i18n::translate(self.ui.effective_locale(), "collab.session.invite"),
                        9.0,
                        self.theme.muted_foreground,
                        Point2D::new(rect.origin.x + PAD + 9.0, top + 14.0),
                        500,
                    );
                    let shown = crate::util::ellipsize_to_width(
                        invite.as_str(),
                        rect.size.x - PAD * 2.0 - 48.0,
                        |text| text_metrics::measure_chrome_weighted(cx.backend, text, 11.0, 500),
                    );
                    paint_text(
                        cx,
                        &shown,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD + 9.0, top + 34.0),
                        500,
                    );
                    if let Some(copy) = self.invite_copy_rect(rect) {
                        let hovered =
                            self.ui.collab.panel.hover == Some(CollabPanelHover::CopyInvite);
                        if hovered {
                            cx.backend
                                .fill_round_rect(copy, 6.0, self.theme.button_hover);
                        }
                        draw_icon(
                            cx.backend,
                            Icon::Copy,
                            Point2D::new(copy.origin.x + 4.0, copy.origin.y + 4.0),
                            16.0,
                            if hovered {
                                self.theme.foreground
                            } else {
                                self.theme.primary
                            },
                            1.4,
                        );
                    }
                    INVITE_HEIGHT
                } else {
                    0.0
                };
                let share_offset = if let Some(endpoint) = share_endpoint {
                    let top = body_top + 58.0 + connection_offset + invite_offset;
                    let local_label = format!(
                        "{} · {}",
                        op_i18n::translate(self.ui.effective_locale(), "collab.connection.lan"),
                        op_i18n::translate(
                            self.ui.effective_locale(),
                            "collab.session.shareAddress"
                        )
                    );
                    paint_text(
                        cx,
                        &local_label,
                        9.0,
                        self.theme.muted_foreground,
                        Point2D::new(rect.origin.x + PAD, top + 11.0),
                        500,
                    );
                    let shown = crate::util::ellipsize_to_width(
                        endpoint.as_str(),
                        rect.size.x - PAD * 2.0 - 30.0,
                        |text| text_metrics::measure_chrome(cx.backend, text, 11.0),
                    );
                    paint_text(
                        cx,
                        &shown,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD, top + 29.0),
                        400,
                    );
                    if let Some(copy) = self.share_endpoint_copy_rect(rect) {
                        let hovered =
                            self.ui.collab.panel.hover == Some(CollabPanelHover::CopyShareEndpoint);
                        if hovered {
                            cx.backend
                                .fill_round_rect(copy, 6.0, self.theme.button_hover);
                        }
                        draw_icon(
                            cx.backend,
                            Icon::Copy,
                            Point2D::new(copy.origin.x + 4.0, copy.origin.y + 4.0),
                            16.0,
                            if hovered {
                                self.theme.foreground
                            } else {
                                self.theme.muted_foreground
                            },
                            1.4,
                        );
                    }
                    SHARE_ENDPOINT_HEIGHT
                } else {
                    0.0
                };
                let admission_offset = if let Some(request) = admission_request {
                    let public_offset = connection_offset + invite_offset;
                    paint_text(
                        cx,
                        &request.label,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(
                            rect.origin.x + PAD,
                            body_top + 72.0 + public_offset + share_offset,
                        ),
                        500,
                    );
                    let enabled = self.ui.collab.pending_action.is_none();
                    for (button, action) in self.admission_action_rects(rect, body_top, request) {
                        paint_button(
                            cx,
                            &self.theme,
                            button,
                            &action.label,
                            action.primary,
                            enabled,
                            self.action_is_hovered(&action.action),
                        );
                    }
                    ADMISSION_HEIGHT
                } else {
                    0.0
                };
                for (index, participant) in participants
                    .iter()
                    .take(MAX_VISIBLE_PARTICIPANTS)
                    .enumerate()
                {
                    paint_participant(
                        cx,
                        &self.theme,
                        self.ui,
                        participant,
                        rect.origin.x + PAD,
                        body_top
                            + 58.0
                            + connection_offset
                            + invite_offset
                            + share_offset
                            + admission_offset
                            + index as f32 * ROW_HEIGHT,
                        rect.size.x - PAD * 2.0,
                    );
                }
            }
        }

        cx.backend.restore();

        let enabled = self.ui.collab.pending_action.is_none();
        for (button, action) in self.action_rects(rect) {
            paint_button(
                cx,
                &self.theme,
                button,
                &action.label,
                action.primary,
                enabled,
                self.action_is_hovered(&action.action),
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(self.model.title.clone());
        node
    }
}

#[cfg(test)]
#[path = "collab_panel_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "collab_panel_region_tests.rs"]
mod region_tests;

#[cfg(test)]
#[path = "collab_panel_touch_tests.rs"]
mod touch_tests;
