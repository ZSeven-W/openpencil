//! Support half of the shared PropertyPanel dispatch (split out of
//! `property_panel_dispatch.rs` for the 800-line cap): the Code-panel
//! action state machine, the effect-parameter focus seed, and the
//! font / colour-variable resolution helpers. Re-exported from
//! `property_panel_dispatch` so hosts see a single module path.

use op_editor_core::EditorState;

use crate::widgets::property_panel_action::CodegenAction;
use crate::widgets::PropertyPanel;

/// Seed the effect-parameter draft after the host committed any pending
/// property / variable-row edit. Re-reads the live value off the panel
/// snapshot, because that commit may have moved it.
pub fn focus_effect_param(
    state: &mut EditorState,
    effect: usize,
    field: op_editor_core::EffectField,
    value: f32,
    now_ms: u64,
) {
    let value = effect_param_snapshot_value(state, effect, field).unwrap_or(value);
    let initial = if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    };
    let ui = &mut state.ui;
    ui.property_input.set_text(initial.clone());
    ui.property_input.touch(now_ms);
    ui.property_input_draft = initial;
    ui.property_caret_pos = ui.property_input.caret();
    ui.property_caret_anchor_ms = now_ms;
    ui.property_draft_select_all = false;
    state.editor_ui.effect_param_focus =
        Some(op_editor_core::editor_ui_state::EffectParamFocus { effect, field });
}

/// Platform work a Code-panel action still needs after
/// [`apply_codegen_action`] ran its state half.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenFollowUp {
    None,
    /// The framework changed — drop any live code-selection drag.
    FrameworkChanged,
    /// Push the generated code onto the platform clipboard.
    Copy(String),
    /// Write the generated code out (native: pending flag drained by
    /// the desktop export pass; web: browser download).
    Download,
    /// Build + save the live AI structure bundle.
    ExportBundle,
}

/// Code-panel action state half. `SelectFramework` / `Generate` /
/// `Regenerate` / `Cancel` / framework scrolling are pure
/// `editor_state.codegen`; the IO arms come back as a follow-up.
pub fn apply_codegen_action(
    state: &mut EditorState,
    action: &CodegenAction,
    now_ms: u64,
) -> CodegenFollowUp {
    use op_editor_core::codegen::CodegenPhase;
    let property_panel_width = state.editor_ui.property_panel_width;
    let density = PropertyPanel::for_selection(state);
    let logical_property_panel_width = density.as_ref().map_or(property_panel_width, |panel| {
        panel.logical_length(property_panel_width)
    });
    let cg = &mut state.codegen;
    match action {
        CodegenAction::SelectFramework(fw) => {
            if cg.select_framework(*fw) {
                return CodegenFollowUp::FrameworkChanged;
            }
            CodegenFollowUp::None
        }
        CodegenAction::Generate => {
            cg.pending_generate = true;
            cg.phase = CodegenPhase::Generating;
            cg.error = None;
            cg.code_scroll.offset = 0.0;
            cg.code_selection = None;
            CodegenFollowUp::None
        }
        CodegenAction::Regenerate => {
            cg.pending_regenerate = true;
            cg.phase = CodegenPhase::Generating;
            cg.error = None;
            cg.code_scroll.offset = 0.0;
            cg.code_selection = None;
            CodegenFollowUp::None
        }
        CodegenAction::Cancel => {
            cg.pending_generate = false;
            cg.pending_regenerate = false;
            // Raise the cancel intent for the host runner — it aborts
            // the in-flight worker / XHR so the run actually stops
            // instead of streaming on and resurrecting the panel
            // (TS: abort()).
            cg.pending_cancel = true;
            cg.phase = if cg.code.is_empty() {
                CodegenPhase::Idle
            } else {
                CodegenPhase::Complete
            };
            CodegenFollowUp::None
        }
        CodegenAction::Copy => {
            cg.copied_at = Some(now_ms);
            CodegenFollowUp::Copy(cg.code.clone())
        }
        CodegenAction::Download => CodegenFollowUp::Download,
        CodegenAction::ExportBundle => CodegenFollowUp::ExportBundle,
        CodegenAction::ScrollFrameworksLeft | CodegenAction::ScrollFrameworksRight => {
            let logical_max = crate::widgets::property_panel_code::framework_row_overflow(
                logical_property_panel_width,
            );
            let max = density
                .as_ref()
                .map_or(logical_max, |panel| panel.physical_length(logical_max));
            let step = density
                .as_ref()
                .map_or(100.0, |panel| panel.physical_length(100.0));
            let delta = if matches!(action, CodegenAction::ScrollFrameworksLeft) {
                -step
            } else {
                step
            };
            cg.framework_scroll.scroll_by(delta, max, 0.0);
            CodegenFollowUp::None
        }
    }
}

/// Resolve a `SetFontFamilyIndex` / `RemoveImportedFont` index against
/// the SAME entries list the picker painted / hit-tested (imported +
/// bundled + system lists filtered by the live search).
pub(crate) fn font_picker_family_at(state: &EditorState, index: usize) -> Option<String> {
    let ui = &state.editor_ui;
    crate::widgets::property_panel_typography::font_picker_entries(
        &ui.imported_font_families,
        &ui.bundled_font_families,
        &ui.system_font_families,
        &ui.font_picker_search,
    )
    .get(index)
    .map(|entry| entry.family.to_string())
}

fn effect_param_snapshot_value(
    state: &EditorState,
    effect: usize,
    field: op_editor_core::EffectField,
) -> Option<f32> {
    PropertyPanel::for_selection(state)?
        .snapshot
        .effects
        .get(effect)
        .map(|summary| summary.param_value(field))
}

/// The name of colour variable `index` in document order.
pub fn color_variable_name_at(state: &EditorState, index: usize) -> Option<String> {
    state
        .doc
        .variables
        .as_ref()?
        .iter()
        .filter(|(_, def)| matches!(def.kind, jian_ops_schema::variable::VariableKind::Color))
        .nth(index)
        .map(|(name, _)| name.clone())
}

/// Translate the panel's `ColorTarget` into the `ui_draft` one the
/// colour-picker state machine uses.
pub fn color_target(t: op_editor_core::ColorTarget) -> op_editor_core::ui_draft::ColorTarget {
    match t {
        op_editor_core::ColorTarget::Fill => op_editor_core::ui_draft::ColorTarget::Fill,
        op_editor_core::ColorTarget::Stroke => op_editor_core::ui_draft::ColorTarget::Stroke,
        op_editor_core::ColorTarget::GradientStop(i) => {
            op_editor_core::ui_draft::ColorTarget::GradientStop(i)
        }
        op_editor_core::ColorTarget::EffectColor(i) => {
            op_editor_core::ui_draft::ColorTarget::EffectColor(i)
        }
    }
}
