//! Widget payload conversion for the scene builder.
//!
//! Code motion out of the `layout_scene.rs` spine (800-line ceiling): the
//! payload -> [`SceneWidget`] copy, plus the one paint-time lookup a widget
//! needs from the variable table (the page foreground for adjacent labels).

use jian_scene::layout_scene::{SceneWidget, SceneWidgetOption};
use op_editor_core::scene_vars::VariableTable;

/// Convert a payload `WidgetPayload` into the paint-only
/// [`SceneWidget`]. Plain field copy — option rows map 1:1 — plus the
/// document's page-text colour for checkbox / radio labels.
pub(super) fn widget_payload_to_scene(
    w: &crate::payload::WidgetPayload,
    var_table: &VariableTable,
) -> SceneWidget {
    SceneWidget {
        kind: w.kind.clone(),
        checked: w.checked,
        toggle_progress: None,
        value_num: w.value_num,
        value_str: w.value_str.clone(),
        placeholder: w.placeholder.clone(),
        leading_icon: w.leading_icon.clone(),
        trailing_icon: w.trailing_icon.clone(),
        label: w.label.clone(),
        // A label beside the indicator is page text; resolving the design
        // system's foreground keeps it independent of checked / accent paint.
        label_foreground: var_table
            .resolve_color("--foreground")
            .or_else(|| var_table.resolve_color("foreground")),
        min: w.min,
        max: w.max,
        step: w.step,
        indeterminate: w.indeterminate,
        corner_radius_authored: w.corner_radius_authored,
        options: w
            .options
            .iter()
            .map(|o| SceneWidgetOption {
                value: o.value.clone(),
                label: o.label.clone(),
            })
            .collect(),
    }
}
