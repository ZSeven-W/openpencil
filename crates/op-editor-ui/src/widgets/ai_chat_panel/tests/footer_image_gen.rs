//! Footer image-generation toggle: layout, hit/hover, press semantics,
//! tooltip. Split out of `tests/footer_layout.rs` at the 800-line cap.

use super::super::*;
use super::support::*;
use crate::widgets::ai_chat_hit::AIChatHit;

// ── Image-generation toggle ──────────────────────────────────────────────────

fn state_with_image_gen(configured: bool, enabled: bool) -> EditorState {
    let mut s = EditorState::new();
    // The expanded panel only exists where the conversation lives:
    // the rail's Agent tab. Elsewhere the chat is composer-only.
    s.editor_ui.enter_chat_tab();
    if configured {
        let id = s.editor_ui.agent_settings.add_image_gen_profile();
        let _ = id;
        s.editor_ui.agent_settings.image_gen_profiles[0].api_key = "sk-image".into();
    }
    s.editor_ui.agent_settings.image_gen_enabled = enabled;
    s
}

fn image_gen_center(s: &EditorState) -> (AIChatPlaceholder<'_>, Rect, Point2D) {
    let panel = AIChatPlaceholder::from_editor(s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let toolbar_top = input.origin.y + INPUT_AREA_HEIGHT;
    let footer = panel.footer_layout(rect, input, toolbar_top);
    let point = Point2D::new(
        footer.image_gen.origin.x + footer.image_gen.size.x / 2.0,
        footer.image_gen.origin.y + footer.image_gen.size.y / 2.0,
    );
    (panel, rect, point)
}

#[test]
fn image_gen_toggle_is_laid_out_left_of_the_speed_chip() {
    let s = state_with_image_gen(true, false);
    let (_panel, _rect, footer_pt) = image_gen_center(&s);
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input = panel.input_rect(rect);
    let footer = panel.footer_layout(rect, input, input.origin.y + INPUT_AREA_HEIGHT);
    assert!(footer.image_gen.size.x > 0.0, "toggle must be laid out");
    assert!(
        footer.thinking.origin.x + footer.thinking.size.x <= footer.image_gen.origin.x,
        "thinking toggle must end before the image-gen toggle starts"
    );
    assert!(
        footer.image_gen.origin.x + footer.image_gen.size.x <= footer.speed.origin.x,
        "image-gen toggle must end before the ⚡ chip starts"
    );
    assert_eq!(footer.image_gen.size, footer.thinking.size);
    let _ = footer_pt;
}

#[test]
fn image_gen_toggle_hit_and_hover_resolve_their_targets() {
    let s = state_with_image_gen(true, false);
    let (panel, rect, point) = image_gen_center(&s);
    assert_eq!(panel.hit_test(rect, point), Some(AIChatHit::ToggleImageGen));
    assert_eq!(
        panel.footer_hover_at(rect, point),
        Some(op_editor_core::ChatFooterButton::ImageGen)
    );
}

#[test]
fn image_gen_toggle_press_while_unconfigured_opens_settings_on_images_tab() {
    use crate::widgets::chat_click_flow::apply_chat_hit;

    let mut s = state_with_image_gen(false, false);
    let (_panel, _rect, _point) = image_gen_center(&s);
    assert_eq!(
        apply_chat_hit(&mut s, AIChatHit::ToggleImageGen, 0),
        crate::widgets::chat_click_flow::ChatClickStep::Dirty
    );
    assert!(
        !s.editor_ui.agent_settings.image_gen_enabled,
        "an unconfigured press must not flip the flag"
    );
    assert!(s.editor_ui.agent_settings_open, "settings must open");
    assert_eq!(
        s.editor_ui.agent_settings.tab,
        op_editor_core::AgentSettingsTab::Images,
        "the modal must land on the Images tab"
    );
}

#[test]
fn image_gen_toggle_press_while_configured_flips_the_flag() {
    use crate::widgets::chat_click_flow::apply_chat_hit;

    let mut s = state_with_image_gen(true, false);
    apply_chat_hit(&mut s, AIChatHit::ToggleImageGen, 0);
    assert!(s.editor_ui.agent_settings.image_gen_enabled);
    assert!(
        !s.editor_ui.agent_settings_open,
        "a configured press toggles in place, it does not open settings"
    );
    apply_chat_hit(&mut s, AIChatHit::ToggleImageGen, 1);
    assert!(!s.editor_ui.agent_settings.image_gen_enabled);
}

#[test]
fn hovering_the_image_gen_toggle_paints_a_state_appropriate_tooltip() {
    for (configured, key) in [
        (false, "collab.chat.imageGenConfigure"),
        (true, "collab.chat.imageGenName"),
    ] {
        let mut s = state_with_image_gen(configured, false);
        s.editor_ui.locale = op_editor_core::Locale::EnUs;
        s.editor_ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::ImageGen);
        let panel = AIChatPlaceholder::from_editor(&s);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let mut backend = PanelPaintBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);

        let expected = op_i18n::translate(op_editor_core::Locale::EnUs, key);
        assert!(
            backend.texts.iter().any(|(text, ..)| text == expected),
            "hovering (configured={configured}) must paint `{expected}`"
        );
    }
}
