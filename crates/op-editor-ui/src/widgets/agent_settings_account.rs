//! Account tab of the settings modal.
//!
//! Signed out: a login-guidance card reusing the same primary "Sign in
//! with browser" affordance as [`crate::widgets::login_modal`]. Signed
//! in: display name / username / avatar-initial + a Sign Out row. Mirrors
//! the System tab's card-based layout (`agent_settings_system.rs`).

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::button::tokens_from_theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::settings_form::ellipsize;
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_widgets::components::button::{Button, ButtonVariant};
use jian_widgets::components::card::Card;
use op_editor_core::editor_ui_state::{EditorUiState, Locale};
use op_editor_core::AccountState;

const TITLE_H: f32 = 48.0;
const CARD_H: f32 = 88.0;
const AVATAR: f32 = 44.0;
const ACTION_BTN_W: f32 = 112.0;
const ACTION_BTN_H: f32 = 34.0;
const CARD_INSET: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTabHit {
    /// Signed out: opens the sign-in modal.
    SignIn,
    /// Signed in: clears the account back to `Anonymous`.
    SignOut,
    None,
}

pub(super) fn content_height() -> f32 {
    12.0 + TITLE_H + CARD_H + 24.0
}

fn card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 12.0 + TITLE_H),
        size: Point2D::new(content.size.x, CARD_H),
    }
}

fn action_btn_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - CARD_INSET - ACTION_BTN_W,
            card.origin.y + (CARD_H - ACTION_BTN_H) / 2.0,
        ),
        size: Point2D::new(ACTION_BTN_W, ACTION_BTN_H),
    }
}

fn action_btn_target(card: Rect, touch: bool) -> Rect {
    let visual = action_btn_rect(card);
    if !touch {
        return visual;
    }
    Rect {
        origin: Point2D::new(visual.origin.x, card.origin.y + (CARD_H - 44.0) / 2.0),
        size: Point2D::new(visual.size.x, 44.0),
    }
}

pub fn hit_test(content: Rect, ui: &EditorUiState, scrolled: Point2D) -> AccountTabHit {
    if !action_btn_target(card_rect(content), ui.touch_chrome()).contains(scrolled) {
        return AccountTabHit::None;
    }
    if ui.account.is_signed_in() {
        AccountTabHit::SignOut
    } else {
        AccountTabHit::SignIn
    }
}

pub(super) fn paint_account_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
) {
    let title = TextLayout::single_run(
        t_settings(ui, "settings.account.title"),
        "system-ui",
        19.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(650);
    let title_icon = Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 4.0),
        size: Point2D::new(32.0, 32.0),
    };
    cx.backend
        .fill_oval(title_icon, theme.primary.with_alpha(0.10));
    cx.backend
        .stroke_oval(title_icon, theme.primary.with_alpha(0.22), 1.0);
    draw_icon(
        cx.backend,
        Icon::User,
        Point2D::new(title_icon.origin.x + 8.0, title_icon.origin.y + 8.0),
        16.0,
        theme.primary,
        1.7,
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x + 44.0, content.origin.y + 27.0),
    );

    let card = card_rect(content);
    account_card_style(theme).paint(cx.backend, card, &tokens_from_theme(theme));

    match &ui.account {
        AccountState::Anonymous => paint_signed_out(cx, theme, ui, card),
        AccountState::SignedIn {
            display_name,
            username,
        } => paint_signed_in(cx, theme, ui, card, display_name, username),
    }
}

fn paint_signed_out(cx: &mut PaintCx<'_>, theme: &Theme, ui: &EditorUiState, card: Rect) {
    let avatar_rect = avatar_rect(card);
    paint_avatar_tile(cx, theme, avatar_rect);

    let text_x = avatar_rect.origin.x + AVATAR + 12.0;
    let text_w = (action_btn_rect(card).origin.x - text_x - 12.0).max(0.0);
    let label_text = ellipsize(
        cx,
        t_settings(ui, "settings.account.notSignedIn"),
        text_w,
        14.0,
    );
    let label = TextLayout::single_run(
        &label_text,
        "system-ui",
        14.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(600);
    cx.backend.draw_text(
        &label,
        Point2D::new(text_x, card.origin.y + card.size.y / 2.0 - 4.0),
    );
    let hint_text = ellipsize(cx, signed_out_hint(ui.effective_locale()), text_w, 11.0);
    let hint = TextLayout::single_run(
        &hint_text,
        "system-ui",
        11.0,
        theme.muted_foreground.to_jian(),
        Point2D::ZERO,
    );
    cx.backend.draw_text(
        &hint,
        Point2D::new(text_x, card.origin.y + card.size.y / 2.0 + 17.0),
    );
    paint_primary_action(
        cx,
        theme,
        action_btn_rect(card),
        t_settings(ui, "settings.account.signIn"),
    );
}

fn paint_signed_in(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    display_name: &str,
    username: &str,
) {
    let avatar_rect = avatar_rect(card);
    cx.backend.fill_oval(avatar_rect, theme.primary);
    let initial = ui.account.initial().to_string();
    let initial_w = text_metrics::measure_chrome(cx.backend, &initial, 15.0);
    let initial_label = TextLayout::single_run(
        &initial,
        "system-ui",
        15.0,
        (theme.primary_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &initial_label,
        Point2D::new(
            avatar_rect.origin.x + (AVATAR - initial_w) / 2.0,
            avatar_rect.origin.y + AVATAR / 2.0 + 5.0,
        ),
    );
    let _ = crate::widgets::account_avatar_paint::paint_account_avatar_image(
        cx,
        &ui.account,
        avatar_rect,
    );
    cx.backend
        .stroke_oval(avatar_rect, theme.border.with_alpha(0.72), 1.0);

    let text_x = avatar_rect.origin.x + AVATAR + 12.0;
    let text_w = (action_btn_rect(card).origin.x - text_x - 12.0).max(0.0);
    let display_name = ellipsize(cx, display_name, text_w, 14.0);
    let name_label = TextLayout::single_run(
        &display_name,
        "system-ui",
        14.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(600);
    cx.backend.draw_text(
        &name_label,
        Point2D::new(text_x, card.origin.y + CARD_H / 2.0 - 2.0),
    );
    let username_display = format!("@{}", username);
    let username_display = ellipsize(cx, &username_display, text_w, 11.0);
    let username_label = TextLayout::single_run(
        &username_display,
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &username_label,
        Point2D::new(text_x, card.origin.y + CARD_H / 2.0 + 16.0),
    );

    paint_action_button(
        cx,
        theme,
        action_btn_rect(card),
        t_settings(ui, "settings.account.signOut"),
    );
}

fn avatar_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + CARD_INSET,
            card.origin.y + (CARD_H - AVATAR) / 2.0,
        ),
        size: Point2D::new(AVATAR, AVATAR),
    }
}

fn paint_avatar_tile(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect) {
    cx.backend.fill_oval(rect, theme.primary.with_alpha(0.09));
    cx.backend
        .stroke_oval(rect, theme.primary.with_alpha(0.20), 1.0);
    draw_icon(
        cx.backend,
        Icon::User,
        Point2D::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
        24.0,
        theme.primary,
        1.8,
    );
}

fn signed_out_hint(locale: Locale) -> &'static str {
    op_i18n::translate(locale, "account.signedOutHint")
}

fn account_card_style(theme: &Theme) -> Card {
    Card {
        fill: Some(theme.muted),
        border: Some(theme.border),
        radius: 10.0,
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

fn paint_primary_action(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str) {
    let first = mix(theme.primary, Color::WHITE, 0.06);
    let second = mix(theme.primary, Color::rgb_u8(79, 70, 229), 0.32);
    let shadow = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + 3.0),
        size: rect.size,
    };
    cx.backend
        .fill_drop_shadow(shadow, 11.0, 9.0, theme.primary.with_alpha(0.20));
    cx.backend.fill_round_rect_linear_gradient(
        rect,
        11.0,
        &[(0.0, first), (1.0, second)],
        0.0,
        1.0,
    );
    cx.backend
        .stroke_round_rect(rect, 11.0, theme.primary_foreground.with_alpha(0.16), 1.0);

    let font_size = 12.5;
    let weight = 600;
    let arrow_size = 14.0;
    let gap = 8.0;
    let label_w = cx.backend.measure_text_weighted(label, font_size, weight);
    let group_w = label_w + gap + arrow_size;
    let group_x = rect.origin.x + (rect.size.x - group_w) / 2.0;
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        font_size,
        theme.primary_foreground.to_jian(),
        Point2D::ZERO,
    )
    .with_font_weight(weight);
    cx.backend.draw_text(
        &layout,
        Point2D::new(group_x, rect.origin.y + rect.size.y / 2.0 + 4.0),
    );
    draw_icon(
        cx.backend,
        Icon::ArrowRight,
        Point2D::new(
            group_x + label_w + gap,
            rect.origin.y + (rect.size.y - arrow_size) / 2.0,
        ),
        arrow_size,
        theme.primary_foreground.with_alpha(0.88),
        1.7,
    );
}

fn paint_action_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, label: &str) {
    Button {
        label,
        icon_paths: None,
        variant: ButtonVariant::DestructiveOutline,
        enabled: true,
        hovered: false,
        pressed: false,
        font_size: 12.0,
    }
    .paint(cx.backend, rect, &tokens_from_theme(theme));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_card_geometry_keeps_the_action_inside_the_card() {
        let content = Rect::xywh(220.0, 80.0, 472.0, 640.0);
        let card = card_rect(content);
        let action = action_btn_rect(card);
        let avatar = avatar_rect(card);

        for rect in [action, avatar] {
            assert!(card.contains(rect.origin));
            assert!(card.contains(Point2D::new(
                rect.origin.x + rect.size.x,
                rect.origin.y + rect.size.y,
            )));
        }
        assert!(avatar.origin.x + avatar.size.x < action.origin.x);
    }

    #[test]
    fn account_card_uses_the_settings_surface_in_both_themes() {
        for theme in [Theme::light(), Theme::dark()] {
            let style = account_card_style(&theme);
            assert_eq!(style.fill, Some(theme.muted));
            assert_eq!(style.border, Some(theme.border));
            assert_eq!(style.radius, 10.0);
        }
    }

    #[test]
    fn account_action_center_is_clickable() {
        let content = Rect::xywh(220.0, 80.0, 472.0, 640.0);
        let action = action_btn_rect(card_rect(content));
        let center = Point2D::new(
            action.origin.x + action.size.x / 2.0,
            action.origin.y + action.size.y / 2.0,
        );
        let mut ui = EditorUiState::default();

        assert_eq!(hit_test(content, &ui, center), AccountTabHit::SignIn);
        ui.account = AccountState::SignedIn {
            display_name: "Kayshen".into(),
            username: "kayshen".into(),
        };
        assert_eq!(hit_test(content, &ui, center), AccountTabHit::SignOut);
    }

    #[test]
    fn touch_account_action_has_a_44_point_target() {
        let content = Rect::xywh(16.0, 120.0, 288.0, 400.0);
        let target = action_btn_target(card_rect(content), true);
        let ui = EditorUiState {
            touch: true,
            ..EditorUiState::default()
        };

        assert_eq!(target.size.y, 44.0);
        assert_eq!(
            hit_test(
                content,
                &ui,
                Point2D::new(target.origin.x + 1.0, target.origin.y + 1.0)
            ),
            AccountTabHit::SignIn
        );
    }

    #[test]
    fn signed_out_hint_is_localized_for_chinese() {
        assert_eq!(
            signed_out_hint(Locale::ZhCn),
            "登录后即可同步你的设置与偏好"
        );
        assert_eq!(
            signed_out_hint(Locale::ZhTw),
            "登入後即可同步你的設定與偏好"
        );
    }
}
