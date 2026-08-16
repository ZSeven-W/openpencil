//! Sign-in modal — opened from the TopBar avatar button while signed
//! out, or from the settings modal's Account tab. The surface keeps
//! authentication focused: OpenPencil identity, one browser action,
//! and a concise security/status note.
//!
//! Touch builds without an auth runtime show a disabled "coming soon"
//! action instead of pretending to sign in. `AccountState::
//! dev_fake_signed_in` is the dev/demo fast path (gated host-side by
//! `OPENPENCIL_DEV_FAKE_LOGIN=1`), never reachable from this widget on
//! its own.

use crate::theme::Theme;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::text_metrics;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::editor_ui_state::Locale;
use op_editor_core::{EditorState, LoginModalButton};

#[path = "login_modal_disabled_action.rs"]
mod disabled_action;

pub const MODAL_WIDTH: f32 = 408.0;
pub const MODAL_HEIGHT: f32 = 306.0;
const PANEL_RADIUS: f32 = 18.0;
const SIGN_IN_BTN_W: f32 = 336.0;
const SIGN_IN_BTN_H: f32 = 48.0;
// Keep the CTA center at `panel.bottom - 56`, matching the established
// host-side press fixture while the content above it is redesigned.
const SIGN_IN_BTN_Y: f32 = MODAL_HEIGHT - 80.0;
const CLOSE_SIZE: f32 = 30.0;
const BRAND_BADGE_SIZE: f32 = 52.0;
const BRAND_LOGO_IMAGE_ID: u64 = 0x4f50_5a37_4c4f_474f;
const BRAND_LOGO_PNG: &[u8] = include_bytes!("../../../op-host-desktop/assets/icon.png");
const STATUS_HEIGHT: f32 = 44.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginModalHit {
    Close,
    SignIn,
    Outside,
    Inside,
}

pub struct LoginModal {
    pub id: WidgetId,
    pub theme: Theme,
    locale: Locale,
    /// Set after the primary button is clicked in a stub build —
    /// replaces the security note with the honest availability status.
    stub_hint_shown: bool,
    /// In-flight device-login progress — takes precedence over the
    /// security note and the stub hint.
    flow_status: Option<op_editor_core::LoginFlowStatus>,
    touch: bool,
    sign_in_enabled: bool,
    hover: Option<LoginModalButton>,
    pressed: Option<LoginModalButton>,
}

impl LoginModal {
    pub fn for_editor(state: &EditorState) -> Self {
        Self {
            id: WidgetId::new(5500),
            theme: theme_for(&state.editor_ui),
            locale: state.editor_ui.effective_locale(),
            stub_hint_shown: state.editor_ui.login_modal_stub_hint_shown,
            flow_status: state.editor_ui.login_modal_status,
            touch: state.editor_ui.touch_chrome(),
            sign_in_enabled: !state.editor_ui.touch_chrome()
                || state.editor_ui.account_ui_available,
            hover: state.editor_ui.login_modal_hover,
            pressed: match state.editor_ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::LoginModal(button)) => Some(button),
                _ => None,
            },
        }
    }

    pub fn rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        if !self.touch {
            let x = ((viewport_w - MODAL_WIDTH) / 2.0).max(16.0);
            let y = ((viewport_h - MODAL_HEIGHT) / 2.0).max(crate::widgets::TOP_BAR_HEIGHT + 16.0);
            return Rect::xywh(x, y, MODAL_WIDTH, MODAL_HEIGHT);
        }
        // The same modal is used by 320/390pt phones. Keep a 16pt horizontal
        // gutter and centre vertically inside the safe-area-local viewport;
        // requiring a desktop TopBar gutter here pushed the 306pt card out of
        // a 320pt landscape phone.
        let width = MODAL_WIDTH.min((viewport_w - 32.0).max(0.0));
        let height = MODAL_HEIGHT.min((viewport_h - 16.0).max(0.0));
        let x = ((viewport_w - width) / 2.0).max(0.0);
        let y = ((viewport_h - height) / 2.0).max(0.0);
        Rect::xywh(x, y, width, height)
    }

    pub fn hit_test(&self, panel: Rect, point: Point2D) -> LoginModalHit {
        if !panel.contains(point) {
            return LoginModalHit::Outside;
        }
        if close_rect(panel, self.touch).contains(point) {
            return LoginModalHit::Close;
        }
        if self.sign_in_enabled && sign_in_rect(panel).contains(point) {
            return LoginModalHit::SignIn;
        }
        LoginModalHit::Inside
    }
}

fn close_rect(panel: Rect, touch: bool) -> Rect {
    let size = if touch { 44.0 } else { CLOSE_SIZE };
    let inset = if touch { 8.0 } else { 16.0 };
    Rect::xywh(
        panel.origin.x + panel.size.x - inset - size,
        panel.origin.y + inset,
        size,
        size,
    )
}

fn sign_in_rect(panel: Rect) -> Rect {
    let width = SIGN_IN_BTN_W.min((panel.size.x - 32.0).max(0.0));
    Rect::xywh(
        panel.origin.x + (panel.size.x - width) / 2.0,
        panel.origin.y + panel.size.y - (MODAL_HEIGHT - SIGN_IN_BTN_Y),
        width,
        SIGN_IN_BTN_H.min(panel.size.y),
    )
}

fn status_rect(panel: Rect) -> Rect {
    let top = panel.origin.y + 166.0;
    let available_height = (sign_in_rect(panel).origin.y - 8.0 - top).max(0.0);
    Rect::xywh(
        panel.origin.x + 36.0,
        top,
        panel.size.x - 72.0,
        STATUS_HEIGHT.min(available_height),
    )
}

fn t(locale: Locale, key: &'static str) -> &'static str {
    op_i18n::translate(locale, key)
}

fn subtitle(locale: Locale) -> &'static str {
    t(locale, "account.signInSubtitle")
}

fn security_note(locale: Locale) -> &'static str {
    // The web editor already runs in a browser — its verification page
    // opens as a popup window, not "the system browser".
    if cfg!(target_arch = "wasm32") {
        t(locale, "account.securityNotePopup")
    } else {
        t(locale, "account.securityNoteBrowser")
    }
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    let t = amount.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn paint_centered_text(
    backend: &mut dyn RenderBackend,
    text: &str,
    center_x: f32,
    baseline_y: f32,
    font_size: f32,
    weight: u16,
    color: Color,
) {
    let width = backend.measure_text_weighted(text, font_size, weight);
    let layout =
        TextLayout::single_run(text, "system-ui", font_size, color.to_jian(), Point2D::ZERO)
            .with_font_weight(weight);
    backend.draw_text(&layout, Point2D::new(center_x - width / 2.0, baseline_y));
}

fn paint_brand_badge(backend: &mut dyn RenderBackend, theme: &Theme, panel: Rect) {
    let badge = Rect::xywh(
        panel.origin.x + (panel.size.x - BRAND_BADGE_SIZE) / 2.0,
        panel.origin.y + 24.0,
        BRAND_BADGE_SIZE,
        BRAND_BADGE_SIZE,
    );
    // The official application artwork already carries the white rounded
    // tile and cyan Z/pencil mark. Keep its natural transparent padding;
    // wrapping it in another coloured badge changes the brand silhouette.
    let tile_inset = 5.0;
    let shadow = Rect::xywh(
        badge.origin.x + tile_inset,
        badge.origin.y + tile_inset + 2.0,
        badge.size.x - tile_inset * 2.0,
        badge.size.y - tile_inset * 2.0,
    );
    backend.fill_drop_shadow(
        shadow,
        11.0,
        9.0,
        Color::BLACK.with_alpha(if theme.background.r < 0.5 { 0.30 } else { 0.14 }),
    );

    if !has_cached_image_bytes(BRAND_LOGO_IMAGE_ID) {
        store_remote_image_bytes(BRAND_LOGO_IMAGE_ID, BRAND_LOGO_PNG.to_vec());
    }
    let max_edge_px = required_raster_edge(badge, backend.dpi_scale());
    let sharp_enough = backend.image_decoded(BRAND_LOGO_IMAGE_ID, BRAND_LOGO_PNG, max_edge_px);
    if !sharp_enough {
        note_pending_decode(BRAND_LOGO_IMAGE_ID, max_edge_px);
    }
    if sharp_enough || backend.image_resident(BRAND_LOGO_IMAGE_ID) {
        backend.draw_image(badge, BRAND_LOGO_IMAGE_ID, BRAND_LOGO_PNG);
    } else {
        // One-frame decode fallback: preserve the official white tile without
        // flashing the old generic pencil glyph.
        backend.fill_round_rect(shadow, 11.0, Color::WHITE);
    }
}

fn paint_primary_action(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    locale: Locale,
    panel: Rect,
    enabled: bool,
    hovered: bool,
    pressed: bool,
) {
    let button = sign_in_rect(panel);
    if !enabled {
        disabled_action::paint(backend, theme, button, t(locale, "settings.account.signIn"));
        return;
    }
    let state_mix = if pressed {
        0.13
    } else if hovered {
        0.08
    } else {
        0.0
    };
    let state_color = if pressed { Color::BLACK } else { Color::WHITE };
    let first = mix(
        mix(theme.primary, Color::WHITE, 0.06),
        state_color,
        state_mix,
    );
    let second = mix(
        mix(theme.primary, Color::rgb_u8(79, 70, 229), 0.28),
        state_color,
        state_mix,
    );

    if !pressed {
        let shadow = Rect::xywh(
            button.origin.x,
            button.origin.y + 3.0,
            button.size.x,
            button.size.y,
        );
        backend.fill_drop_shadow(
            shadow,
            11.0,
            if hovered { 12.0 } else { 8.0 },
            theme.primary.with_alpha(if hovered { 0.26 } else { 0.18 }),
        );
    }
    backend.fill_round_rect_linear_gradient(button, 11.0, &[(0.0, first), (1.0, second)], 0.0, 1.0);
    backend.stroke_round_rect(button, 11.0, theme.primary_foreground.with_alpha(0.18), 1.0);

    let content_offset_y = if pressed { 1.0 } else { 0.0 };
    // Web: the plain "Sign in" label — "with browser" is redundant when
    // the editor itself runs in one.
    let label = if cfg!(target_arch = "wasm32") {
        t(locale, "settings.account.signIn")
    } else {
        t(locale, "account.signInWithBrowser")
    };
    let label_size = 13.5;
    let label_weight = 600;
    let label_width = backend.measure_text_weighted(label, label_size, label_weight);
    let icon_size = 17.0;
    let group_width = icon_size + 9.0 + label_width;
    let group_x = button.origin.x + (button.size.x - group_width) / 2.0;
    let icon_y = button.origin.y + (button.size.y - icon_size) / 2.0 + content_offset_y;
    draw_icon(
        backend,
        Icon::Globe,
        Point2D::new(group_x, icon_y),
        icon_size,
        theme.primary_foreground,
        1.7,
    );
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        label_size,
        theme.primary_foreground.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(label_weight);
    backend.draw_text(
        &label_layout,
        Point2D::new(
            group_x + icon_size + 9.0,
            button.origin.y + button.size.y / 2.0 + 4.5 + content_offset_y,
        ),
    );

    let arrow_size = 15.0;
    draw_icon(
        backend,
        Icon::ArrowRight,
        Point2D::new(
            button.origin.x + button.size.x - 28.0,
            button.origin.y + (button.size.y - arrow_size) / 2.0 + content_offset_y,
        ),
        arrow_size,
        theme.primary_foreground.with_alpha(0.82),
        1.6,
    );
}

fn paint_status_note(
    backend: &mut dyn RenderBackend,
    theme: &Theme,
    locale: Locale,
    panel: Rect,
    stub_hint_shown: bool,
    flow_status: Option<op_editor_core::LoginFlowStatus>,
) {
    use op_editor_core::{LoginFlowError, LoginFlowStatus};
    let status = status_rect(panel);
    // The web build runs IN a browser: the approval happens in a popup
    // window, so "waiting for your browser" would read as nonsense there.
    let (waiting_key, approval_key) = if cfg!(target_arch = "wasm32") {
        ("account.openingPopup", "account.approveInPopup")
    } else {
        ("account.waitingForBrowser", "account.waitingForApproval")
    };
    let (icon, text, background, border, color) = if let Some(flow) = flow_status {
        match flow {
            LoginFlowStatus::WaitingBrowser => (
                Icon::Globe,
                t(locale, waiting_key),
                theme.row_selected_primary,
                theme.primary.with_alpha(0.32),
                theme.foreground,
            ),
            LoginFlowStatus::WaitingApproval => (
                Icon::Globe,
                t(locale, approval_key),
                theme.row_selected_primary,
                theme.primary.with_alpha(0.32),
                theme.foreground,
            ),
            LoginFlowStatus::Exchanging => (
                Icon::Loader,
                t(locale, "account.signingIn"),
                theme.row_selected_primary,
                theme.primary.with_alpha(0.32),
                theme.foreground,
            ),
            LoginFlowStatus::Failed(error) => (
                Icon::AlertTriangle,
                match error {
                    LoginFlowError::Denied => t(locale, "account.signInDenied"),
                    LoginFlowError::Expired => t(locale, "account.signInExpired"),
                    LoginFlowError::Canceled => t(locale, "account.signInCanceled"),
                    LoginFlowError::Unavailable => t(locale, "account.signInFailed"),
                },
                theme.destructive.with_alpha(0.14),
                theme.destructive.with_alpha(0.4),
                theme.foreground,
            ),
        }
    } else if stub_hint_shown {
        (
            Icon::Info,
            t(locale, "account.signInComingSoon"),
            theme.row_selected_primary,
            theme.primary.with_alpha(0.32),
            theme.foreground,
        )
    } else {
        (
            Icon::Lock,
            security_note(locale),
            theme.muted.with_alpha(0.72),
            theme.border.with_alpha(0.72),
            theme.muted_foreground,
        )
    };
    backend.fill_round_rect(status, 11.0, background);
    backend.stroke_round_rect(status, 11.0, border, 1.0);

    let font_size = 10.5;
    let icon_size = 13.0;
    let gap = 7.0;
    let text = text_metrics::fit_chrome(
        backend,
        text,
        (status.size.x - 18.0 - icon_size - gap).max(0.0),
        font_size,
    );
    let text_width = text_metrics::measure_chrome(backend, &text, font_size);
    let group_width = icon_size + gap + text_width;
    let group_x = status.origin.x + (status.size.x - group_width) / 2.0;
    draw_icon(
        backend,
        icon,
        Point2D::new(group_x, status.origin.y + (status.size.y - icon_size) / 2.0),
        icon_size,
        if stub_hint_shown {
            theme.primary
        } else {
            color
        },
        1.5,
    );
    let layout = TextLayout::single_run(
        &text,
        "system-ui",
        font_size,
        color.to_jian(),
        Point2D::ZERO,
    );
    backend.draw_text(
        &layout,
        Point2D::new(
            group_x + icon_size + gap,
            status.origin.y + status.size.y / 2.0 + 3.5,
        ),
    );
}

impl Widget for LoginModal {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, MODAL_WIDTH, MODAL_HEIGHT),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let panel_shadow = Rect::xywh(rect.origin.x, rect.origin.y + 6.0, rect.size.x, rect.size.y);
        cx.backend.fill_drop_shadow(
            panel_shadow,
            PANEL_RADIUS,
            24.0,
            Color::BLACK.with_alpha(if self.theme.background.r < 0.5 {
                0.46
            } else {
                0.16
            }),
        );
        cx.backend
            .fill_round_rect(rect, PANEL_RADIUS, self.theme.popover);

        let header_glow = Rect::xywh(
            rect.origin.x + 1.0,
            rect.origin.y + 1.0,
            rect.size.x - 2.0,
            116.0,
        );
        cx.backend.fill_round_rect_linear_gradient_per_corner(
            header_glow,
            [PANEL_RADIUS - 1.0, PANEL_RADIUS - 1.0, 0.0, 0.0],
            &[
                (0.0, self.theme.primary.with_alpha(0.0)),
                (0.5, self.theme.primary.with_alpha(0.10)),
                (1.0, self.theme.primary.with_alpha(0.0)),
            ],
            90.0,
            1.0,
        );
        cx.backend.stroke_round_rect(
            rect,
            PANEL_RADIUS,
            mix(self.theme.border, self.theme.primary, 0.10),
            1.0,
        );

        let close = close_rect(rect, self.touch);
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::Close.paths(),
            hovered: self.hover == Some(LoginModalButton::Close),
            pressed: self.pressed == Some(LoginModalButton::Close),
            active: false,
            enabled: true,
            icon_size: 15.0,
            stroke_width: 1.7,
        }
        .paint(
            cx.backend,
            close,
            &crate::widgets::button::tokens_from_theme(&self.theme),
        );

        paint_brand_badge(cx.backend, &self.theme, rect);

        let center_x = rect.origin.x + rect.size.x / 2.0;
        paint_centered_text(
            cx.backend,
            t(self.locale, "account.signInTitle"),
            center_x,
            rect.origin.y + 111.0,
            19.0,
            650,
            self.theme.foreground,
        );
        paint_centered_text(
            cx.backend,
            subtitle(self.locale),
            center_x,
            rect.origin.y + 140.0,
            12.0,
            400,
            self.theme.muted_foreground,
        );

        paint_primary_action(
            cx.backend,
            &self.theme,
            self.locale,
            rect,
            self.sign_in_enabled,
            self.hover == Some(LoginModalButton::SignIn),
            self.pressed == Some(LoginModalButton::SignIn),
        );
        paint_status_note(
            cx.backend,
            &self.theme,
            self.locale,
            rect,
            self.stub_hint_shown,
            self.flow_status,
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Dialog);
        node.set_label(t(self.locale, "account.signInTitle"));
        node.set_description(subtitle(self.locale));
        node
    }
}

#[cfg(test)]
#[path = "login_modal_touch_tests.rs"]
mod touch_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::editor_ui_state::ThemeMode;
    use op_editor_core::ButtonPressTarget;

    #[derive(Default)]
    struct CaptureBackend {
        fills: Vec<(Rect, f32, Color)>,
        gradients: Vec<(Rect, Vec<(f32, Color)>)>,
        texts: Vec<String>,
        images: Vec<(Rect, u64, usize)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _rect: Rect, _color: Color) {}
        fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, _origin: Point2D) {
            self.texts
                .extend(layout.runs().iter().map(|run| run.content.clone()));
        }
        fn clip_rect(&mut self, _rect: Rect) {}
        fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}
        fn stroke_svg_path(
            &mut self,
            _d: &str,
            _top_left: Point2D,
            _size: f32,
            _color: Color,
            _width: f32,
        ) {
        }
        fn fill_round_rect_linear_gradient(
            &mut self,
            rect: Rect,
            _radius: f32,
            stops: &[(f32, Color)],
            _angle_deg: f32,
            _opacity: f32,
        ) {
            self.gradients.push((rect, stops.to_vec()));
        }
        fn fill_round_rect_linear_gradient_per_corner(
            &mut self,
            rect: Rect,
            _radii: [f32; 4],
            stops: &[(f32, Color)],
            _angle_deg: f32,
            _opacity: f32,
        ) {
            self.gradients.push((rect, stops.to_vec()));
        }
        fn draw_image(&mut self, rect: Rect, image_id: u64, encoded: &[u8]) {
            self.images.push((rect, image_id, encoded.len()));
        }
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _offset: Point2D) {}
        fn resize(&mut self, _width: u32, _height: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn paint_modal(state: &EditorState) -> (LoginModal, Rect, CaptureBackend) {
        let modal = LoginModal::for_editor(state);
        let panel = modal.rect(900.0, 700.0);
        let mut backend = CaptureBackend::default();
        modal.paint(
            &mut PaintCx {
                backend: &mut backend,
            },
            panel,
        );
        (modal, panel, backend)
    }

    fn button_gradient(backend: &CaptureBackend, panel: Rect) -> Vec<(f32, Color)> {
        backend
            .gradients
            .iter()
            .find(|(rect, _)| *rect == sign_in_rect(panel))
            .expect("primary button should use a gradient")
            .1
            .clone()
    }

    #[test]
    fn interactive_rects_are_inside_panel_and_do_not_overlap() {
        let modal = LoginModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        let close = close_rect(panel, modal.touch);
        let sign_in = sign_in_rect(panel);
        let status = status_rect(panel);

        for rect in [close, sign_in, status] {
            assert!(panel.contains(rect.origin));
            assert!(panel.contains(Point2D::new(
                rect.origin.x + rect.size.x,
                rect.origin.y + rect.size.y,
            )));
        }
        assert!(status.origin.y + status.size.y < sign_in.origin.y);
        assert!(close.origin.y + close.size.y < sign_in.origin.y);
    }

    #[test]
    fn clicking_sign_in_and_close_buttons_is_recognised() {
        let modal = LoginModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        let sign_in = sign_in_rect(panel);
        let close = close_rect(panel, modal.touch);

        assert_eq!(
            modal.hit_test(
                panel,
                Point2D::new(
                    sign_in.origin.x + sign_in.size.x / 2.0,
                    sign_in.origin.y + sign_in.size.y / 2.0,
                ),
            ),
            LoginModalHit::SignIn
        );
        assert_eq!(
            modal.hit_test(
                panel,
                Point2D::new(
                    close.origin.x + close.size.x / 2.0,
                    close.origin.y + close.size.y / 2.0,
                ),
            ),
            LoginModalHit::Close
        );
    }

    #[test]
    fn clicking_outside_the_panel_is_outside() {
        let modal = LoginModal::for_editor(&EditorState::new());
        let panel = modal.rect(800.0, 600.0);
        assert_eq!(
            modal.hit_test(panel, Point2D::new(panel.origin.x - 5.0, panel.origin.y)),
            LoginModalHit::Outside
        );
    }

    #[test]
    fn hover_and_pressed_states_change_the_primary_button_treatment() {
        let idle = EditorState::new();
        let (_, panel, idle_paint) = paint_modal(&idle);

        let mut hovered = EditorState::new();
        hovered.editor_ui.login_modal_hover = Some(LoginModalButton::SignIn);
        let (_, _, hover_paint) = paint_modal(&hovered);

        let mut pressed = EditorState::new();
        pressed.editor_ui.pressed_button =
            Some(ButtonPressTarget::LoginModal(LoginModalButton::SignIn));
        let (_, _, pressed_paint) = paint_modal(&pressed);

        assert_ne!(
            button_gradient(&idle_paint, panel),
            button_gradient(&hover_paint, panel)
        );
        assert_ne!(
            button_gradient(&hover_paint, panel),
            button_gradient(&pressed_paint, panel)
        );
    }

    #[test]
    fn status_note_switches_from_security_copy_to_stub_feedback() {
        let mut idle = EditorState::new();
        idle.editor_ui.locale = Locale::EnUs;
        let (_, _, idle_paint) = paint_modal(&idle);
        assert!(idle_paint
            .texts
            .iter()
            .any(|text| text == security_note(Locale::EnUs)));

        let mut stub = EditorState::new();
        stub.editor_ui.locale = Locale::EnUs;
        stub.editor_ui.login_modal_stub_hint_shown = true;
        let (_, _, stub_paint) = paint_modal(&stub);
        assert!(stub_paint
            .texts
            .iter()
            .any(|text| text == t(Locale::EnUs, "account.signInComingSoon")));
        assert!(!stub_paint
            .texts
            .iter()
            .any(|text| text == security_note(Locale::EnUs)));
    }

    #[test]
    fn modal_uses_theme_popover_surface_in_light_and_dark_modes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let mut state = EditorState::new();
            state.editor_ui.theme_mode = mode;
            let (modal, panel, paint) = paint_modal(&state);
            assert!(paint
                .fills
                .iter()
                .any(|(rect, radius, color)| *rect == panel
                    && *radius == PANEL_RADIUS
                    && *color == modal.theme.popover));
        }
    }

    #[test]
    fn brand_identity_uses_official_logo_and_zseven_account_copy() {
        let mut state = EditorState::new();
        state.editor_ui.locale = Locale::ZhCn;
        let (_, _, paint) = paint_modal(&state);

        assert!(paint
            .texts
            .iter()
            .any(|text| text == "使用 Zseven 账户继续"));
        assert!(!paint
            .texts
            .iter()
            .any(|text| text == "使用 OpenPencil 账户继续"));
        assert!(paint.images.iter().any(|(_, image_id, byte_len)| {
            *image_id == BRAND_LOGO_IMAGE_ID && *byte_len == BRAND_LOGO_PNG.len()
        }));
    }
}
