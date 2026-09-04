use super::property_panel::PropertyPanel;
use super::property_panel_action::CodegenAction;
use super::property_panel_code;
use super::property_panel_dispatch as dispatch;
use super::property_panel_sections as sections;
use super::property_panel_test_support::visible_for;
use crate::widgets::test_capture_backend::CaptureBackend;
use crate::widgets::{PaintCx, Widget};
use crate::Rect;
use op_editor_core::codegen::{CodegenHover, CodegenPhase};
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::{ButtonPressTarget, EditorState, PropertyTab};

fn rect_eq(a: Rect, b: Rect) -> bool {
    (a.origin.x - b.origin.x).abs() < 0.01
        && (a.origin.y - b.origin.y).abs() < 0.01
        && (a.size.x - b.size.x).abs() < 0.01
        && (a.size.y - b.size.y).abs() < 0.01
}

#[test]
fn property_action_pressed_uses_shared_feedback() {
    let mut state = EditorState::sample();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::PropertyPanel(0));
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    assert_eq!(panel.action_pressed, Some(0));

    let rect = Rect::xywh(0.0, 0.0, 280.0, 900.0);
    let action_rects = sections::action_button_rects_with_fill_picker(
        rect,
        visible_for(&panel),
        &panel.snapshot.effects,
        &panel.snapshot.fills,
        &panel.snapshot.interactions,
        panel.fill_type_picker.open,
        panel.fill_type_picker_index,
        panel.font_picker.open,
        panel.font_weight_picker_open,
        panel.export_scale_picker_open,
        panel.export_format_picker_open,
        panel.padding_mode_popover_open,
    );
    assert!(
        !action_rects.is_empty(),
        "sample panel should expose actions"
    );

    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(_, radius, color)| *radius == 6.0 && *color == expected),
        "pressed property action should paint shared pressed feedback"
    );
}

#[test]
fn codegen_action_pressed_uses_shared_feedback() {
    let mut state = EditorState::new();
    state.editor_ui.property_tab = PropertyTab::Code;
    state.codegen.phase = CodegenPhase::Complete;
    state.codegen.code = "fn main() {\n    println!(\"hi\");\n}\n".into();
    state.editor_ui.pressed_button = Some(ButtonPressTarget::Codegen(CodegenHover::Copy));
    let panel = PropertyPanel::for_selection(&state).expect("Code tab panel is selection-free");
    assert_eq!(panel.codegen_pressed, Some(CodegenHover::Copy));

    let rect = Rect::xywh(0.0, 0.0, 280.0, 700.0);
    let (_, copy_rect) = property_panel_code::code_action_rects_in_panel_with_locale(
        rect,
        &state.codegen,
        panel.locale,
    )
    .into_iter()
    .find(|(action, _)| matches!(action, CodegenAction::Copy))
    .expect("Copy action rect");
    let expected = panel
        .theme
        .button_hover
        .with_alpha(panel.theme.button_hover.a * 1.8);
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(fill, _, color)| rect_eq(*fill, copy_rect) && *color == expected),
        "pressed codegen Copy action should paint shared pressed feedback"
    );
}

#[test]
fn compact_direct_tab_and_codegen_dispatch_cannot_reopen_code() {
    let mut state = EditorState::sample();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    let mut image_drag = None;
    let mut effect_drag = None;

    let outcome = dispatch::apply_property_action(
        &mut state,
        &super::PropertyPanelAction::SetPropertyTab(PropertyTab::Code),
        dispatch::PropertyActionContext {
            now_ms: 0,
            resolved_sizing_fallback: None,
            image_adjustment_drag: &mut image_drag,
            effect_radius_drag: &mut effect_drag,
        },
    );
    assert_eq!(outcome, dispatch::PropertyActionOutcome::Handled);
    assert_eq!(state.editor_ui.property_tab, PropertyTab::Design);

    let follow_up = dispatch::apply_codegen_action(&mut state, &CodegenAction::Generate, 1);
    assert_eq!(follow_up, dispatch::CodegenFollowUp::None);
    assert!(!state.codegen.pending_generate);
}
