//! `PropertyPanel` construction — selection resolution, snapshot
//! extraction wiring, and the `EditorState` → panel-field copy.
//!
//! Split out of `property_panel.rs` to keep both files under the
//! openpencil 800-line cap.

use super::{NodeSnapshot, PropertyPanel};
use crate::layout_scene::{SceneStroke, SceneStrokeAlign};
use crate::widgets::editor_state_ext::theme_for;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::property_panel_snapshot::color_from_hex;
use crate::widgets::WidgetId;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorVariableOption {
    pub name: String,
    pub resolved_hex: Option<String>,
}

fn color_variable_options(state: &EditorState) -> Vec<ColorVariableOption> {
    let Some(vars) = state.doc.variables.as_ref() else {
        return Vec::new();
    };
    vars.iter()
        .filter(|(_, def)| matches!(def.kind, jian_ops_schema::variable::VariableKind::Color))
        .map(|(name, _)| ColorVariableOption {
            name: name.clone(),
            resolved_hex: state.resolve_color_variable_hex(name),
        })
        .collect()
}

fn apply_resolved_variable_colors(
    state: &EditorState,
    node: &jian_ops_schema::node::PenNode,
    snapshot: &mut NodeSnapshot,
    fill_ref: Option<&str>,
    stroke_ref: Option<&str>,
) {
    if let Some(color) = fill_ref
        .and_then(|name| state.resolve_color_variable_hex(name))
        .and_then(|hex| color_from_hex(&hex))
    {
        snapshot.fill = Some(color);
        // The colour-variable subsystem keys off the primary fill, so
        // mirror the binding onto `fills[0]` for the Fill section's
        // first row (it paints the `$name` chip + variable button).
        if let Some(first) = snapshot.fills.first_mut() {
            first.color = color;
            first.variable_ref = fill_ref.map(str::to_string);
        }
    }
    if let Some(color) = stroke_ref
        .and_then(|name| state.resolve_color_variable_hex(name))
        .and_then(|hex| color_from_hex(&hex))
    {
        let width = op_editor_core::fills::node_stroke_width(node).unwrap_or(1.0) as f32;
        snapshot.stroke = Some(SceneStroke {
            color,
            width,
            sides: crate::widgets::property_panel_snapshot::stroke_sides_for_scene(node),
            align: SceneStrokeAlign::Center,
        });
    }
}

impl PropertyPanel {
    /// Conditional builder — returns `Some` only when the editor
    /// has an active selection. Mirrors TS `{hasSelection && ...}`.
    pub fn for_selection(state: &EditorState) -> Option<Self> {
        Self::for_selection_at(state, 0)
    }

    /// Build the panel and replace a single selection's displayed W/H with
    /// its layout-resolved canvas size. This matters for Fill/Hug nodes: the
    /// canonical node stores a sizing keyword, while the inspector must show
    /// the concrete size the user is about to freeze by typing a number.
    pub fn for_selection_with_scene(
        state: &EditorState,
        scene: &crate::layout_scene::LayoutScene,
    ) -> Option<Self> {
        Self::for_selection_at_with_scene(state, scene, 0)
    }

    /// Clocked variant of [`Self::for_selection_with_scene`].
    pub fn for_selection_at_with_scene(
        state: &EditorState,
        scene: &crate::layout_scene::LayoutScene,
        now_ms: u64,
    ) -> Option<Self> {
        let mut panel = Self::for_selection_at(state, now_ms)?;
        if !panel.page_only && state.selection_count() == 1 {
            if let Some(node) = scene
                .active_page()
                .and_then(|page| page.find(state.selection.anchor.as_str()))
            {
                let bounds = node.aggregate_bounds();
                if bounds.size.x.is_finite() && bounds.size.x >= 0.0 {
                    panel.snapshot.width = bounds.size.x.round() as i32;
                }
                if bounds.size.y.is_finite() && bounds.size.y >= 0.0 {
                    panel.snapshot.height = bounds.size.y.round() as i32;
                }
            }
        }
        Some(panel)
    }

    /// Same as [`for_selection`] but threads the host's monotonic
    /// millisecond clock through so the focused-input caret can
    /// blink off the same animation timer as the chat input.
    pub fn for_selection_at(state: &EditorState, now_ms: u64) -> Option<Self> {
        if let Some(panel) = Self::for_selection_nodes(state, now_ms) {
            return Some(panel);
        }
        // The Code tab is selection-independent: the TS code-panel falls
        // back to the active page's children, so the panel must stay
        // alive (and the tab reachable) with an empty / unresolvable
        // selection. The Code body never reads the snapshot, so a
        // neutral placeholder suffices.
        if state.editor_ui.effective_property_tab() == op_editor_core::PropertyTab::Code {
            return Some(Self::build_from_snapshot(
                state,
                NodeSnapshot::empty_for_code_tab(),
                op_editor_core::FillType::Solid,
                now_ms,
                false,
                None,
                None,
            ));
        }
        None
    }

    /// Selection-driven panel builder — `None` when no selected id
    /// resolves to a live node (the pre-Code-tab `for_selection_at`).
    fn for_selection_nodes(state: &EditorState, now_ms: u64) -> Option<Self> {
        if state.selection_count() == 1 {
            let authored_node = state.selected_node();
            let authored_ref_target = match authored_node {
                Some(jian_ops_schema::node::PenNode::Ref(reference)) => {
                    Some(reference.target.clone())
                }
                _ => None,
            };
            // An INSTANCE (`Ref`) selection resolves into its merged
            // display node — component base → descendants[target]
            // overrides → instance props. A virtual child resolves to
            // the effective component child plus descendants[childId].
            // A dangling Ref falls back to the raw node.
            let display = match authored_node {
                Some(node) => op_editor_core::resolve_instance_display_node(&state.doc, node),
                None => op_editor_core::resolve_instance_display_node_for_anchor(
                    &state.doc,
                    &state.selection.anchor,
                ),
            };
            let is_instance = matches!(authored_node, Some(jian_ops_schema::node::PenNode::Ref(_)));
            let display_node = display.or_else(|| authored_node.cloned())?;
            let node = &display_node;
            let fill_type = op_editor_core::first_fill_type(node);
            let variable_name = |raw: Option<&str>| {
                raw.and_then(|value| value.strip_prefix('$'))
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            };
            let fill_ref = variable_name(op_editor_core::first_solid_fill_hex(node));
            let stroke_ref = variable_name(op_editor_core::first_solid_stroke_hex(node));
            // Only a page-root child's `screen` marker is ever
            // meaningful (`wire_screen_navigation`'s contract) — check
            // the AUTHORED selection anchor, not the resolved instance
            // display node, against the active page's top-level ids.
            let is_top_level = state
                .active_children()
                .iter()
                .any(|n| n.id_str() == state.selection.anchor.as_str());
            let mut snapshot = NodeSnapshot::from_node(node, is_top_level);
            if !state.editor_ui.agent_settings.experimental_features_enabled {
                // The Widget section is an experimental surface. Hide it
                // unless opted in — the section's paint AND height both key
                // off `snapshot.widget`, so clearing it here removes the
                // section everywhere with no layout drift.
                snapshot.widget = None;
            }
            snapshot.is_instance = is_instance;
            if !is_instance
                && state
                    .components
                    .find_by_id(&state.selection.anchor)
                    .is_some()
            {
                snapshot.is_reusable = true;
            }
            apply_resolved_variable_colors(
                state,
                node,
                &mut snapshot,
                fill_ref.as_deref(),
                stroke_ref.as_deref(),
            );
            let mut panel = Self::build_from_snapshot(
                state, snapshot, fill_type, now_ms, false, fill_ref, stroke_ref,
            );
            if let Some(target) = authored_ref_target {
                let options = state.components.sorted_options();
                let has_alternative =
                    options.len() > 1 || options.first().is_some_and(|option| option.id != target);
                panel.instance_component_target = Some(target);
                if has_alternative {
                    panel.instance_component_options = options;
                    panel.instance_component_picker_open =
                        state.editor_ui.instance_component_picker_open
                            && state.editor_ui.instance_component_picker_anchor
                                == state.selection.anchor.as_str();
                }
            }
            panel.image_panel_view =
                crate::widgets::property_panel_image_assets::image_panel_view(state, node);
            return Some(panel);
        }
        if state.selection_count() >= 2 {
            let snapshot = NodeSnapshot::from_multi_selection(state)?;
            return Some(Self::build_from_snapshot(
                state,
                snapshot,
                op_editor_core::FillType::Solid,
                now_ms,
                true,
                None,
                None,
            ));
        }
        None
    }

    fn build_from_snapshot(
        state: &EditorState,
        snapshot: NodeSnapshot,
        fill_type: op_editor_core::FillType,
        now_ms: u64,
        is_multi: bool,
        fill_variable_ref: Option<String>,
        stroke_variable_ref: Option<String>,
    ) -> Self {
        let ui = &state.editor_ui;
        let code_tab_available = ui.code_property_tab_available();
        let property_tab = ui.effective_property_tab();
        let color_variables = color_variable_options(state);
        let color_variable_count = color_variables.len();
        let flex_layout = snapshot.flex_layout;
        let size_flags = sections::SizeFlags {
            fill_width: snapshot.size_fill_width,
            fill_height: snapshot.size_fill_height,
            hug_width: snapshot.size_hug_width,
            hug_height: snapshot.size_hug_height,
            clip_content: snapshot.size_clip_content,
        };
        // Padding edit mode: the user's gear pin (only while it still
        // applies to the selected node — see `padding_edit_mode_anchor`),
        // else derived from the node's four effective values (TS
        // default-derives each frame). Anchor-scoping stops one node's
        // pinned mode leaking into the next selection.
        let pin_applies = ui.padding_edit_mode_anchor == state.selection.anchor.as_str();
        let padding_edit_mode = ui
            .padding_edit_mode
            .filter(|_| pin_applies)
            .unwrap_or_else(|| {
                let p = snapshot.layout_padding;
                op_editor_core::PaddingEditMode::from_values(p.top, p.right, p.bottom, p.left)
            });
        let stroke_pin_applies = ui.stroke_edit_mode_anchor == state.selection.anchor.as_str();
        let stroke_edit_mode = ui
            .stroke_edit_mode
            .filter(|_| stroke_pin_applies)
            .unwrap_or_else(|| {
                let [top, right, bottom, left] = snapshot.stroke_side_widths();
                op_editor_core::PaddingEditMode::from_values(top, right, bottom, left)
            });
        // The Code tab's idle "N nodes selected" label reads the panel's
        // codegen snapshot. Overwrite the clone with the LIVE generation
        // targets (selection, else the active page's children — mirrors
        // the TS `nodeCount`) so the label tracks what Generate / Export
        // AI Bundle would actually run against this frame.
        let mut codegen = state.codegen.clone();
        codegen.selection_snapshot = live_codegen_target_ids(state);
        let corner_expand_open = ui.corner_expand_open && snapshot.supports_per_corner;
        let mut font_picker = ui.font_picker.clone();
        if ui.font_picker_purpose != Some(op_editor_core::FontPickerPurpose::PropertyText) {
            font_picker.open = false;
        }
        let density_scale = if ui.touch_chrome() {
            super::density::TOUCH_DENSITY_SCALE
        } else {
            1.0
        };
        // Host scroll state remains in physical surface points. Section and
        // Code-tab layout below paint in the panel's density-independent
        // logical coordinate space, so normalize the immutable paint snapshot.
        codegen.framework_scroll.offset /= density_scale;
        codegen.code_scroll.offset /= density_scale;
        font_picker.scroll.offset /= density_scale;
        // Effects / Interactions menus are floating, owning surfaces.  Do not
        // carry a stale body-action hover into the immutable paint snapshot
        // while either menu is open: their downward-opening geometry can
        // overlap the next section's action row.
        let action_hover_blocked =
            ui.effect_add_picker_open || ui.interaction_menu_open || ui.compositing_picker.open;
        Self {
            id: WidgetId::new(2000),
            density_scale,
            color_variable_picker_scroll: ui.property_color_variable_picker_scroll.offset.max(0.0),
            color_variable_picker_hover: ui.property_color_variable_picker_hover,
            snapshot,
            theme: theme_for(ui),
            page_only: false,
            page_name: String::new(),
            page_background: None,
            labels: sections::PropertyLabels::for_editor_ui(ui),
            // Multi-select inputs are inert in v1 — broadcast edits
            // to all selected nodes lands later. Force focus to None
            // so the panel paints all values muted and hit_test
            // returns None (see `hit_test` is_multi short-circuit).
            focus: if is_multi {
                None
            } else {
                state.ui.property_focus
            },
            draft: if is_multi {
                String::new()
            } else {
                state.ui.property_input.text().to_owned()
            },
            input: if is_multi {
                jian_core::text_input::TextInputState::default()
            } else {
                state.ui.property_input.clone()
            },
            caret_pos: if is_multi {
                0
            } else {
                state.ui.property_input.caret()
            },
            select_all: !is_multi && state.ui.property_input.is_select_all(),
            now_ms,
            flex_layout,
            size_flags,
            fill_type,
            fill_type_picker: ui.fill_type_picker.clone(),
            fill_type_picker_index: ui.fill_type_picker_index,
            compositing_picker: ui.compositing_picker.clone(),
            compositing_picker_target: ui.compositing_picker_target,
            instance_component_options: std::sync::Arc::from([]),
            instance_component_target: None,
            instance_component_picker_open: false,
            corner_expand_open,
            effect_add_picker_open: ui.effect_add_picker_open,
            interaction_menu_open: ui.interaction_menu_open,
            interaction_menu_hover: ui.interaction_menu_hover,
            screen_paths: crate::widgets::property_panel_interactions::document_screen_paths(state),
            color_variable_picker_open: ui.property_color_variable_picker_open,
            color_variables,
            fill_variable_ref,
            stroke_variable_ref,
            color_variable_count,
            image_fill_popover_open: ui.image_fill_popover_open,
            font_picker,
            font_picker_search: ui.font_picker_search.clone(),
            system_font_families: ui.system_font_families.clone(),
            bundled_font_families: ui.bundled_font_families.clone(),
            imported_font_families: ui.imported_font_families.clone(),
            font_import_supported: ui.font_import_supported,
            font_picker_import_hover: ui.font_picker_import_hover,
            image_panel: ui.image_panel.clone(),
            image_panel_view: None,
            image_gen_profile: crate::widgets::property_panel_image_assets::image_gen_profile_view(
                state,
            ),
            font_weight_picker_open: ui.font_weight_picker_open,
            font_weight_picker_hover: ui.font_weight_picker_hover,
            font_weight_picker_pressed: match ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::FontWeightPicker(index)) => Some(index),
                _ => None,
            },
            action_hover: if is_multi || action_hover_blocked {
                None
            } else {
                ui.property_action_hover
            },
            action_pressed: if is_multi {
                None
            } else {
                match ui.pressed_button {
                    Some(op_editor_core::ButtonPressTarget::PropertyPanel(i)) => Some(i),
                    _ => None,
                }
            },
            padding_edit_mode,
            padding_mode_popover_open: ui.padding_mode_popover_open,
            padding_mode_popover_hover: ui.padding_mode_popover_hover,
            stroke_edit_mode,
            stroke_mode_popover_open: ui.stroke_mode_popover_open,
            stroke_mode_popover_hover: ui.stroke_mode_popover_hover,
            is_multi,
            tab: property_tab,
            tab_hover: ui.property_tab_hover.filter(|tab| {
                code_tab_available || !matches!(tab, op_editor_core::PropertyTab::Code)
            }),
            code_tab_available,
            export_format: ui.export_format,
            export_scale: ui.export_scale,
            export_scale_picker_open: ui.export_scale_picker_open,
            export_format_picker_open: ui.export_format_picker_open,
            export_picker_hover: ui.export_picker_hover,
            effect_add_menu_hover: ui.effect_add_menu_hover,
            scroll: ui.property_panel_scroll.offset.max(0.0),
            locale: ui.effective_locale(),
            // Inert in the multi-select aggregate view.
            effect_param_focus: if is_multi {
                None
            } else {
                ui.effect_param_focus
            },
            codegen,
            codegen_pressed: match ui.pressed_button.filter(|_| code_tab_available) {
                Some(op_editor_core::ButtonPressTarget::Codegen(hover)) => Some(hover),
                _ => None,
            },
        }
    }
}

/// The node ids a code generation started THIS frame would target:
/// the selection when present, else the active page's children (TS
/// `getTargetNodes` / `nodeCount` in code-panel.tsx). Drives the Code
/// tab's idle node-count label.
fn live_codegen_target_ids(state: &EditorState) -> Vec<String> {
    use op_editor_core::PenNodeExt;
    if !state.selection.set.is_empty() {
        return state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
    }
    state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect()
}
