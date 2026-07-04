//! Leaf formatter helpers for [`super::PreviewSession`]'s overlay +
//! diagnostics paths. Split out of `preview/mod.rs` to keep it under
//! the repo's 800-line-per-file cap. All pure, no `self`.

use jian_core::widget_state::WidgetState;
use jian_ops_schema::error::LoadWarning;
use op_editor_ui::layout_scene::SceneWidget;

/// Overlay one widget's live runtime value onto its scene widget. Only
/// value fields change — geometry / options / labels stay as the design
/// scene resolved them. The static design value is the fallback (no
/// runtime state exists until the user interacts).
pub(super) fn apply_widget_state(widget: &mut SceneWidget, state: &WidgetState) {
    match state {
        // text_input / text_area / number_input. `Some("")` falls back
        // to the placeholder in `text_field_display_text`, so an empty
        // edited field shows its placeholder again.
        WidgetState::TextInput(st) => {
            widget.value_str = Some(st.text().to_owned());
        }
        // switch / checkbox.
        WidgetState::Toggle { on } => {
            widget.checked = Some(*on);
        }
        WidgetState::Slider { value, .. } => {
            widget.value_num = Some(*value as f32);
        }
        WidgetState::Select { value, .. } => {
            widget.value_str = value.clone();
        }
        WidgetState::Radio { value, .. } => {
            widget.value_str = value.clone();
        }
        WidgetState::Tabs { active, .. } => {
            widget.value_str = active.clone();
        }
    }
}

/// Human-readable form of an expression result for scene text: strings
/// verbatim, null → empty, everything else via JSON rendering (ints
/// print without a trailing `.0`).
pub(super) fn display_string(value: &jian_core::value::RuntimeValue) -> String {
    match &value.0 {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Format a load warning for the editor's diagnostics surface. Only
/// surfaces the actionable ones (legacy promotions, future versions,
/// skipped logic) — generic unknown-field noise is dropped.
pub(super) fn format_warning(w: &LoadWarning) -> Option<String> {
    match w {
        LoadWarning::LegacyRolePromoted {
            path,
            from_role,
            to,
        } => Some(format!(
            "LegacyRolePromoted: '{path}' role '{from_role}' → {to}"
        )),
        LoadWarning::FutureFormatVersion {
            found,
            supported_max,
        } => Some(format!(
            "FutureFormatVersion: {found} (supported ≤ {supported_max})"
        )),
        LoadWarning::LogicModulesSkipped { reason } => {
            Some(format!("LogicModulesSkipped: {reason}"))
        }
        LoadWarning::InvalidExpression { path, reason, .. } => {
            Some(format!("InvalidExpression: '{path}': {reason}"))
        }
        LoadWarning::UnknownField { .. } => None,
    }
}
