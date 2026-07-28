//! Paint and hit-test surface for the collaboration popover.
//!
//! The panel consumes only the sanitized models from `collab_ui`; network,
//! ticket, stable-subject, and device data never enter this widget.

use crate::theme::Theme;
use crate::widgets::collab_ui::{
    role_label, CollabAdmissionRequestModel, CollabAvatarModel, CollabPanelActionModel,
    CollabPanelModel, CollabPanelScreen,
};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{CollabUiAction, EditorUiState};

pub const COLLAB_PANEL_WIDTH: f32 = 340.0;
const HEADER_HEIGHT: f32 = 44.0;
const PAD: f32 = 14.0;
const ROW_HEIGHT: f32 = 34.0;
const INPUT_HEIGHT: f32 = 32.0;
const ACTION_HEIGHT: f32 = 32.0;
const ACTION_GAP: f32 = 8.0;
const NOTICE_HEIGHT: f32 = 42.0;
const SHARE_ENDPOINT_HEIGHT: f32 = 38.0;
const ADMISSION_HEIGHT: f32 = 62.0;
const ADMISSION_ACTION_HEIGHT: f32 = 28.0;
const MAX_VISIBLE_PARTICIPANTS: usize = 8;
const MAX_VISIBLE_ENDPOINTS: usize = 6;

#[derive(Clone, PartialEq, Eq)]
pub enum CollabPanelHit {
    Close,
    FocusJoinAddress,
    OpenSignIn,
    CopyShareEndpoint(String),
    Action(CollabUiAction),
    Inside,
}

impl std::fmt::Debug for CollabPanelHit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Close => formatter.write_str("Close"),
            Self::FocusJoinAddress => formatter.write_str("FocusJoinAddress"),
            Self::OpenSignIn => formatter.write_str("OpenSignIn"),
            Self::CopyShareEndpoint(_) => formatter.write_str("CopyShareEndpoint([REDACTED])"),
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
}

impl<'a> CollabPanel<'a> {
    pub fn for_editor_ui(ui: &'a EditorUiState) -> Option<Self> {
        if !ui.collab.panel.open {
            return None;
        }
        Some(Self {
            id: WidgetId::new(5650),
            ui,
            model: CollabPanelModel::for_editor_ui(ui),
            theme: theme_for(ui),
        })
    }

    pub fn rect_at(&self, anchor: Rect, viewport: Rect) -> Rect {
        let width = COLLAB_PANEL_WIDTH.min((viewport.size.x - 16.0).max(180.0));
        let right = anchor.origin.x + anchor.size.x;
        let x = (right - width)
            .max(viewport.origin.x + 8.0)
            .min(viewport.origin.x + viewport.size.x - width - 8.0);
        let preferred_y = anchor.origin.y + anchor.size.y + 6.0;
        let height = self
            .panel_height()
            .min((viewport.origin.y + viewport.size.y - preferred_y - 8.0).max(120.0));
        Rect::xywh(x, preferred_y, width, height)
    }

    pub fn panel_height(&self) -> f32 {
        let notice = if self.model.notice.is_some() {
            NOTICE_HEIGHT + 8.0
        } else {
            0.0
        };
        let body = match &self.model.screen {
            CollabPanelScreen::Unavailable | CollabPanelScreen::SignInRequired => 82.0,
            CollabPanelScreen::Home => 66.0,
            CollabPanelScreen::Progress { .. } => 70.0,
            CollabPanelScreen::Join { discovered, .. } => {
                64.0 + discovered.len().min(MAX_VISIBLE_ENDPOINTS) as f32 * ROW_HEIGHT
            }
            CollabPanelScreen::Session {
                share_endpoint,
                participants,
                admission_request,
                ..
            } => {
                58.0 + if share_endpoint.is_some() {
                    SHARE_ENDPOINT_HEIGHT
                } else {
                    0.0
                } + if admission_request.is_some() {
                    ADMISSION_HEIGHT
                } else {
                    0.0
                } + participants.len().min(MAX_VISIBLE_PARTICIPANTS) as f32 * ROW_HEIGHT
            }
        };
        HEADER_HEIGHT + notice + body + self.actions_height() + PAD
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<CollabPanelHit> {
        if !panel.contains(point) {
            return None;
        }
        if self.close_rect(panel).contains(point) {
            return Some(CollabPanelHit::Close);
        }
        let body_top = self.body_top(panel);
        match &self.model.screen {
            CollabPanelScreen::SignInRequired => {
                if self.sign_in_rect(panel, body_top).contains(point) {
                    return Some(CollabPanelHit::OpenSignIn);
                }
            }
            CollabPanelScreen::Join { discovered, .. } => {
                if self.address_rect(panel, body_top + 22.0).contains(point) {
                    return Some(CollabPanelHit::FocusJoinAddress);
                }
                let first_y = body_top + 64.0;
                for (index, endpoint) in discovered.iter().take(MAX_VISIBLE_ENDPOINTS).enumerate() {
                    let row = Rect::xywh(
                        panel.origin.x + PAD,
                        first_y + index as f32 * ROW_HEIGHT,
                        panel.size.x - PAD * 2.0,
                        ROW_HEIGHT,
                    );
                    if row.contains(point) {
                        return Some(if endpoint.compatible {
                            CollabPanelHit::Action(CollabUiAction::JoinDiscovered {
                                discovery_id: endpoint.discovery_id.clone(),
                            })
                        } else {
                            CollabPanelHit::Inside
                        });
                    }
                }
            }
            CollabPanelScreen::Session {
                share_endpoint,
                admission_request,
                ..
            } => {
                if let Some(endpoint) = share_endpoint {
                    if self
                        .share_endpoint_copy_rect(panel)
                        .is_some_and(|rect| rect.contains(point))
                    {
                        return Some(CollabPanelHit::CopyShareEndpoint(
                            endpoint.as_str().to_string(),
                        ));
                    }
                }
                if let Some(request) = admission_request {
                    for (rect, action) in self.admission_action_rects(panel, body_top, request) {
                        if rect.contains(point) {
                            return Some(CollabPanelHit::Action(action.action));
                        }
                    }
                }
            }
            CollabPanelScreen::Unavailable
            | CollabPanelScreen::Home
            | CollabPanelScreen::Progress { .. } => {}
        }
        for (rect, action) in self.action_rects(panel) {
            if rect.contains(point) {
                return Some(CollabPanelHit::Action(action.action));
            }
        }
        Some(CollabPanelHit::Inside)
    }

    fn body_top(&self, panel: Rect) -> f32 {
        panel.origin.y
            + HEADER_HEIGHT
            + if self.model.notice.is_some() {
                NOTICE_HEIGHT + 8.0
            } else {
                0.0
            }
    }

    fn actions_height(&self) -> f32 {
        if self.model.actions.is_empty() {
            0.0
        } else {
            ACTION_HEIGHT + PAD
        }
    }

    fn session_share_height(&self) -> f32 {
        match &self.model.screen {
            CollabPanelScreen::Session {
                share_endpoint: Some(_),
                ..
            } => SHARE_ENDPOINT_HEIGHT,
            _ => 0.0,
        }
    }

    /// Copy target for an owner-only manual share address. Returning `None`
    /// keeps guests and pre-auth screens from manufacturing clipboard data.
    pub fn share_endpoint_copy_rect(&self, panel: Rect) -> Option<Rect> {
        matches!(
            &self.model.screen,
            CollabPanelScreen::Session {
                share_endpoint: Some(_),
                ..
            }
        )
        .then(|| {
            Rect::xywh(
                panel.origin.x + panel.size.x - PAD - 24.0,
                self.body_top(panel) + 53.0,
                24.0,
                24.0,
            )
        })
    }

    fn close_rect(&self, panel: Rect) -> Rect {
        Rect::xywh(
            panel.origin.x + panel.size.x - 38.0,
            panel.origin.y + 8.0,
            30.0,
            28.0,
        )
    }

    fn address_rect(&self, panel: Rect, y: f32) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            y,
            panel.size.x - PAD * 2.0,
            INPUT_HEIGHT,
        )
    }

    fn sign_in_rect(&self, panel: Rect, body_top: f32) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            body_top + 40.0,
            panel.size.x - PAD * 2.0,
            ACTION_HEIGHT,
        )
    }

    fn action_rects(&self, panel: Rect) -> Vec<(Rect, CollabPanelActionModel)> {
        if self.model.actions.is_empty() {
            return Vec::new();
        }
        let count = self.model.actions.len() as f32;
        let available = panel.size.x - PAD * 2.0 - ACTION_GAP * (count - 1.0);
        let width = (available / count).max(68.0);
        let y = panel.origin.y + panel.size.y - PAD - ACTION_HEIGHT;
        self.model
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                (
                    Rect::xywh(
                        panel.origin.x + PAD + index as f32 * (width + ACTION_GAP),
                        y,
                        width,
                        ACTION_HEIGHT,
                    ),
                    action.clone(),
                )
            })
            .collect()
    }

    fn admission_action_rects(
        &self,
        panel: Rect,
        body_top: f32,
        request: &CollabAdmissionRequestModel,
    ) -> Vec<(Rect, CollabPanelActionModel)> {
        let count = request.actions.len() as f32;
        let available = panel.size.x - PAD * 2.0 - ACTION_GAP * (count - 1.0);
        let width = available / count;
        let y = body_top + 81.0 + self.session_share_height();
        request
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                (
                    Rect::xywh(
                        panel.origin.x + PAD + index as f32 * (width + ACTION_GAP),
                        y,
                        width,
                        ADMISSION_ACTION_HEIGHT,
                    ),
                    action.clone(),
                )
            })
            .collect()
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
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(
                self.close_rect(rect).origin.x + 7.0,
                self.close_rect(rect).origin.y + 6.0,
            ),
            16.0,
            self.theme.muted_foreground,
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
            paint_text(
                cx,
                notice,
                11.0,
                self.theme.foreground,
                Point2D::new(notice_rect.origin.x + 9.0, notice_rect.origin.y + 25.0),
                400,
            );
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
                    op_i18n::translate(self.ui.locale, "account.signInWithBrowser"),
                    true,
                    true,
                );
            }
            CollabPanelScreen::Home => {
                self.paint_message(cx, rect, body_top, "collab.join.noSessions");
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
            CollabPanelScreen::Join {
                address,
                discovered,
            } => {
                paint_text(
                    cx,
                    op_i18n::translate(self.ui.locale, "collab.join.address"),
                    11.0,
                    self.theme.muted_foreground,
                    Point2D::new(rect.origin.x + PAD, body_top + 16.0),
                    400,
                );
                let input = self.address_rect(rect, body_top + 22.0);
                cx.backend.fill_round_rect(input, 6.0, self.theme.input);
                cx.backend.stroke_round_rect(
                    input,
                    6.0,
                    if self.ui.collab.panel.join_address_focused {
                        self.theme.ring
                    } else {
                        self.theme.border
                    },
                    1.0,
                );
                let shown = if address.is_empty() {
                    op_i18n::translate(self.ui.locale, "collab.join.addressPlaceholder")
                } else {
                    address
                };
                paint_text(
                    cx,
                    shown,
                    12.0,
                    if address.is_empty() {
                        self.theme.muted_foreground
                    } else {
                        self.theme.foreground
                    },
                    Point2D::new(input.origin.x + 9.0, input.origin.y + 21.0),
                    400,
                );
                let first_y = body_top + 64.0;
                for (index, endpoint) in discovered.iter().take(MAX_VISIBLE_ENDPOINTS).enumerate() {
                    let y = first_y + index as f32 * ROW_HEIGHT;
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
                            op_i18n::translate(self.ui.locale, "collab.join.incompatible"),
                            9.0,
                            self.theme.muted_foreground,
                            Point2D::new(rect.origin.x + rect.size.x - 130.0, y + 21.0),
                            400,
                        );
                    }
                }
                if discovered.is_empty() {
                    paint_text(
                        cx,
                        op_i18n::translate(self.ui.locale, "collab.join.discovering"),
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
                        op_i18n::translate(self.ui.locale, "collab.session.pending"),
                        10.0,
                        self.theme.primary,
                        Point2D::new(rect.origin.x + 90.0, body_top + 40.0),
                        400,
                    );
                }
                let share_offset = if let Some(endpoint) = share_endpoint {
                    paint_text(
                        cx,
                        op_i18n::translate(self.ui.locale, "collab.session.shareAddress"),
                        9.0,
                        self.theme.muted_foreground,
                        Point2D::new(rect.origin.x + PAD, body_top + 59.0),
                        500,
                    );
                    let shown = crate::util::ellipsize_to_width(
                        endpoint.as_str(),
                        rect.size.x - PAD * 2.0 - 30.0,
                        |text| cx.backend.measure_text(text, 11.0),
                    );
                    paint_text(
                        cx,
                        &shown,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD, body_top + 78.0),
                        400,
                    );
                    if let Some(copy) = self.share_endpoint_copy_rect(rect) {
                        draw_icon(
                            cx.backend,
                            Icon::Copy,
                            Point2D::new(copy.origin.x + 4.0, copy.origin.y + 4.0),
                            16.0,
                            self.theme.muted_foreground,
                            1.4,
                        );
                    }
                    SHARE_ENDPOINT_HEIGHT
                } else {
                    0.0
                };
                let admission_offset = if let Some(request) = admission_request {
                    paint_text(
                        cx,
                        &request.label,
                        11.0,
                        self.theme.foreground,
                        Point2D::new(rect.origin.x + PAD, body_top + 72.0 + share_offset),
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
                            + share_offset
                            + admission_offset
                            + index as f32 * ROW_HEIGHT,
                        rect.size.x - PAD * 2.0,
                    );
                }
            }
        }

        let enabled = self.ui.collab.pending_action.is_none();
        for (button, action) in self.action_rects(rect) {
            paint_button(
                cx,
                &self.theme,
                button,
                &action.label,
                action.primary,
                enabled,
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(self.model.title.clone());
        node
    }
}

impl CollabPanel<'_> {
    fn paint_message(&self, cx: &mut PaintCx<'_>, rect: Rect, body_top: f32, key: &'static str) {
        paint_text(
            cx,
            op_i18n::translate(self.ui.locale, key),
            12.0,
            self.theme.muted_foreground,
            Point2D::new(rect.origin.x + PAD, body_top + 29.0),
            400,
        );
    }
}

fn paint_participant(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    participant: &CollabAvatarModel,
    x: f32,
    y: f32,
    width: f32,
) {
    let avatar = Rect::xywh(x, y + 6.0, 22.0, 22.0);
    crate::widgets::collab_avatar_paint::paint_collab_avatar(
        cx,
        participant,
        avatar,
        9.0,
        y + 20.0,
    );
    paint_text(
        cx,
        &participant.display_name,
        12.0,
        theme.foreground,
        Point2D::new(x + 31.0, y + 21.0),
        if participant.is_self { 600 } else { 400 },
    );
    let role = role_label(ui, participant.role);
    let role_w = cx.backend.measure_text(role, 10.0);
    paint_text(
        cx,
        role,
        10.0,
        theme.muted_foreground,
        Point2D::new(x + width - role_w, y + 20.0),
        400,
    );
}

fn paint_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    label: &str,
    primary: bool,
    enabled: bool,
) {
    let background = if primary {
        theme.primary
    } else {
        theme.secondary
    };
    cx.backend.fill_round_rect(
        rect,
        6.0,
        background.with_alpha(if enabled { 1.0 } else { 0.5 }),
    );
    let color = if primary {
        theme.primary_foreground
    } else {
        theme.secondary_foreground
    };
    let width = cx.backend.measure_text(label, 11.0);
    paint_text(
        cx,
        label,
        11.0,
        color.with_alpha(if enabled { 1.0 } else { 0.6 }),
        Point2D::new(
            rect.origin.x + (rect.size.x - width) / 2.0,
            rect.origin.y + 21.0,
        ),
        500,
    );
}

fn paint_text(
    cx: &mut PaintCx<'_>,
    text: &str,
    size: f32,
    color: Color,
    origin: Point2D,
    weight: u16,
) {
    let layout = TextLayout::single_run(text, "system-ui", size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

#[cfg(test)]
#[path = "collab_panel_tests.rs"]
mod tests;
