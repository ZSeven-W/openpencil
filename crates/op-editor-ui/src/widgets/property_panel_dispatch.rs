//! Shared PropertyPanel action dispatch.
//!
//! `apply_property_action` used to exist as two ~500-line near-verbatim
//! copies (`op-host-native/src/widget_host/property_dispatch.rs` and its
//! `op-host-web` twin). Every arm that only writes `EditorState` lives
//! here now; the hosts keep the platform glue (layout-scene size
//! resolution, file dialogs, clipboard, font enumeration, crop-edit
//! entry) and drive it through the returned outcome / follow-up.
//!
//! Ordering contract the hosts must preserve, in this order:
//!   1. commit the image-fill tile-scale draft,
//!   2. snapshot for history when [`updates_document`],
//!   3. [`apply_instance_lifecycle_action`] (acts on the REAL Ref node,
//!      so it runs BEFORE the instance-write redirect),
//!   4. resolve the Fill/Hug pixel fallback off the real layout scene,
//!   5. `begin_instance_write_for_anchor()`,
//!   6. [`apply_property_action`] + the host's own `HostOwned` match,
//!   7. `finish_instance_write` / history push / `mark_dirty`.

use op_editor_core::{EditorState, NodeId};

use crate::widgets::property_panel_action::CompositingTarget;
use crate::widgets::property_panel_layout_ops as layout_ops;
use crate::widgets::PropertyPanelAction;

// The Code-panel action half, the effect-param focus seed and the
// font / colour resolution helpers live in a sibling module (800-line
// cap); re-exported so both hosts keep one `property_panel_dispatch::`
// entry point.
use crate::widgets::property_panel_dispatch_support::font_picker_family_at;
pub use crate::widgets::property_panel_dispatch_support::{
    apply_codegen_action, color_target, color_variable_name_at, focus_effect_param, CodegenFollowUp,
};

/// Platform work the shared dispatcher cannot do itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyActionFollowUp {
    /// Enter image-crop editing for the selected node.
    EnterImageCropEdit,
    /// Leave image-crop editing.
    ExitImageCropEdit,
    /// Enumerate installed font families before the picker paints.
    /// Native resolves this synchronously (the TS app requests Local
    /// Font Access inside the click gesture for the same reason); the
    /// web host drains its own permission flow after the press and
    /// ignores it.
    EnsureSystemFontsLoaded,
}

/// What [`apply_property_action`] did with an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyActionOutcome {
    /// Fully applied against `EditorState`.
    Handled,
    /// Applied, but the host must still run this platform follow-up.
    FollowUp(PropertyActionFollowUp),
    /// Platform-owned — the caller's own match handles it.
    HostOwned,
}

/// Result of [`apply_instance_lifecycle_action`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceLifecycleOutcome {
    /// Not an instance-lifecycle action — continue the normal dispatch.
    Ignored,
    /// Handled: the host refits the viewport when `page_switched`,
    /// applies `select` (in that order — `zoom_to_fit` reads the canvas
    /// region, which the selection can widen), marks dirty and returns.
    Handled {
        page_switched: bool,
        select: Option<NodeId>,
    },
}

/// Host-owned state the shared arms need to touch. Built inline at the
/// call site so the borrows stay disjoint from `&mut EditorState`.
pub struct PropertyActionContext<'a> {
    pub now_ms: u64,
    /// Fill/Hug pixel size read off the real layout scene BEFORE the
    /// instance-write redirect swapped in a merged display node.
    pub resolved_sizing_fallback: Option<f64>,
    pub image_adjustment_drag: &'a mut Option<op_editor_core::ImageAdjustmentField>,
    pub effect_radius_drag: &'a mut Option<usize>,
}

/// Does this action write the document through a lightweight core
/// mutator that doesn't capture history itself? Those need the host's
/// snapshot-and-compare choke point so an instance redirect finishes
/// before equality is tested and a same-value selection never creates
/// an empty undo entry.
pub fn updates_document(action: &PropertyPanelAction) -> bool {
    matches!(
        action,
        PropertyPanelAction::SetNodeBlendMode(_)
            | PropertyPanelAction::SetNodeMaskType(_)
            | PropertyPanelAction::SetFillBlendMode { .. }
            | PropertyPanelAction::SetImageFillMode(_)
            | PropertyPanelAction::ClearPageBackground
    )
}

pub use crate::widgets::property_panel_collab::collab_gate_action;

/// Translate the widget-facade compositing target into the core one.
pub fn compositing_picker_target(
    target: CompositingTarget,
) -> op_editor_core::CompositingPickerTarget {
    match target {
        CompositingTarget::NodeBlend => op_editor_core::CompositingPickerTarget::NodeBlend,
        CompositingTarget::NodeMask => op_editor_core::CompositingPickerTarget::NodeMask,
        CompositingTarget::FillBlend(index) => {
            op_editor_core::CompositingPickerTarget::FillBlend(index)
        }
    }
}

/// Instance / component lifecycle actions. They act on the REAL Ref
/// node, so they dispatch BEFORE the instance-write redirect swaps in
/// the merged display node.
pub fn apply_instance_lifecycle_action(
    state: &mut EditorState,
    action: &PropertyPanelAction,
) -> InstanceLifecycleOutcome {
    use PropertyPanelAction as A;
    match action {
        A::GoToComponent => {
            let mut page_switched = false;
            let mut select = None;
            if let Some(jian_ops_schema::node::PenNode::Ref(reference)) = state.selected_node() {
                let master = NodeId::new(reference.target.clone());
                // Cross-page master: selection resolves against the
                // ACTIVE page only, so switch pages first.
                if let Some(pages) = state.doc.pages.as_ref() {
                    let target_page = pages.iter().position(|page| {
                        op_editor_core::walkers::find_node(&page.children, &master).is_some()
                    });
                    if let Some(index) = target_page {
                        if index != state.ui.active_page_index {
                            let _ = state.set_active_page(index);
                            page_switched = true;
                        }
                    }
                }
                select = Some(master);
            }
            InstanceLifecycleOutcome::Handled {
                page_switched,
                select,
            }
        }
        A::DetachInstance | A::DetachComponent => {
            let id = state.selection.anchor.clone();
            if id.is_real() {
                let _ = state.detach_component(&id);
            }
            InstanceLifecycleOutcome::Handled {
                page_switched: false,
                select: None,
            }
        }
        A::ToggleInstanceComponentPicker => {
            let anchor = state.selection.anchor.as_str().to_string();
            state.editor_ui.toggle_instance_component_picker(&anchor);
            state.editor_ui.close_fill_type_picker();
            state.editor_ui.close_compositing_picker();
            InstanceLifecycleOutcome::Handled {
                page_switched: false,
                select: None,
            }
        }
        A::SetInstanceComponent(component_id) => {
            let instance_id = state.selection.anchor.clone();
            let component_id = NodeId::new(component_id.clone());
            let _ = state.set_instance_component(&instance_id, &component_id);
            state.editor_ui.close_instance_component_picker();
            InstanceLifecycleOutcome::Handled {
                page_switched: false,
                select: None,
            }
        }
        _ => InstanceLifecycleOutcome::Ignored,
    }
}

/// Apply the editor-state half of a PropertyPanel action.
pub fn apply_property_action(
    state: &mut EditorState,
    action: &PropertyPanelAction,
    ctx: PropertyActionContext<'_>,
) -> PropertyActionOutcome {
    use PropertyActionFollowUp as F;
    use PropertyActionOutcome::{FollowUp, Handled, HostOwned};
    use PropertyPanelAction as A;
    match action {
        // Dispatched by `apply_instance_lifecycle_action` before the
        // instance-write scope opened; unreachable here.
        A::GoToComponent
        | A::DetachInstance
        | A::DetachComponent
        | A::ToggleInstanceComponentPicker
        | A::SetInstanceComponent(_) => Handled,
        A::SetPropertyTab(tab) => {
            state.editor_ui.property_tab = *tab;
            Handled
        }
        A::ToggleCompositingPicker(target) => {
            state
                .editor_ui
                .toggle_compositing_picker(compositing_picker_target(*target));
            Handled
        }
        A::SetNodeBlendMode(mode) => {
            let _ = state.set_selected_node_blend_mode(mode.clone());
            state.editor_ui.close_compositing_picker();
            Handled
        }
        A::SetNodeMaskType(mask_type) => {
            let _ = state.set_selected_node_mask_type(*mask_type);
            state.editor_ui.close_compositing_picker();
            Handled
        }
        A::SetFillBlendMode { index, mode } => {
            let _ = state.set_selected_fill_blend_mode(*index, mode.clone());
            state.editor_ui.close_compositing_picker();
            Handled
        }
        A::ClearPageBackground => {
            let _ = state.set_active_page_background_color(None);
            Handled
        }
        A::ToggleCornerExpand => {
            state.editor_ui.toggle_corner_expand();
            Handled
        }
        A::SetFillRule(rule) => {
            let _ = state.set_selected_fill_rule(*rule);
            Handled
        }
        A::SetFlexLayout(mode) => {
            layout_ops::set_selected_layout_mode(state, *mode);
            Handled
        }
        A::ToggleSizeFillWidth => {
            layout_ops::toggle_selected_sizing(
                state,
                true,
                jian_ops_schema::sizing::SizingKeyword::FillContainer,
                ctx.resolved_sizing_fallback,
            );
            Handled
        }
        A::ToggleSizeFillHeight => {
            layout_ops::toggle_selected_sizing(
                state,
                false,
                jian_ops_schema::sizing::SizingKeyword::FillContainer,
                ctx.resolved_sizing_fallback,
            );
            Handled
        }
        A::ToggleSizeHugWidth => {
            layout_ops::toggle_selected_sizing(
                state,
                true,
                jian_ops_schema::sizing::SizingKeyword::FitContent,
                ctx.resolved_sizing_fallback,
            );
            Handled
        }
        A::ToggleSizeHugHeight => {
            layout_ops::toggle_selected_sizing(
                state,
                false,
                jian_ops_schema::sizing::SizingKeyword::FitContent,
                ctx.resolved_sizing_fallback,
            );
            Handled
        }
        A::ToggleSizeClipContent => {
            layout_ops::toggle_selected_clip_content(state);
            Handled
        }
        A::SetLayoutAlign(value) => {
            layout_ops::set_selected_layout_align(state, *value);
            Handled
        }
        A::SetLayoutJustify(value) => {
            layout_ops::set_selected_layout_justify(state, *value);
            Handled
        }
        A::SetLayoutAlignment { justify, align } => {
            layout_ops::set_selected_layout_justify(state, *justify);
            layout_ops::set_selected_layout_align(state, *align);
            Handled
        }
        A::CreateComponent => {
            let id = state.selection.anchor.clone();
            if id.is_real() {
                let _ = state.create_component_from_node_name(&id);
            }
            Handled
        }
        A::ToggleFillTypePicker(index) => {
            let ui = &mut state.editor_ui;
            ui.toggle_fill_type_picker_for(*index);
            ui.image_fill_popover_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetFillType { index, fill_type } => {
            state.set_selected_fill_type_at(*index, *fill_type);
            let ui = &mut state.editor_ui;
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::AddFill => {
            let _ = state.add_selected_fill();
            Handled
        }
        A::MoveFill { from, to } => {
            let _ = state.move_selected_fill(*from, *to);
            Handled
        }
        A::RemoveFill(index) => {
            let _ = state.remove_selected_fill(*index);
            let ui = &mut state.editor_ui;
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::AddGradientStop => {
            let _ = state.add_selected_gradient_stop();
            Handled
        }
        A::RemoveGradientStop(index) => {
            let _ = state.remove_selected_gradient_stop(*index);
            Handled
        }
        A::ToggleImageFillPopover => {
            let ui = &mut state.editor_ui;
            ui.image_fill_popover_open = !ui.image_fill_popover_open;
            ui.close_fill_type_picker();
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            if ui.image_fill_popover_open {
                FollowUp(F::EnterImageCropEdit)
            } else {
                Handled
            }
        }
        A::CloseImageFillPopover => {
            state.editor_ui.image_fill_popover_open = false;
            Handled
        }
        A::SetImageFillMode(mode) => {
            let _ = state.set_selected_image_fill_mode(*mode);
            if *mode == op_editor_core::ImageFillMode::Crop {
                FollowUp(F::EnterImageCropEdit)
            } else {
                FollowUp(F::ExitImageCropEdit)
            }
        }
        A::SetImageAdjustment { field, value } => {
            *ctx.image_adjustment_drag = Some(*field);
            let _ = state.set_selected_image_adjustment(*field, *value);
            Handled
        }
        A::ResetImageAdjustments => {
            *ctx.image_adjustment_drag = None;
            let _ = state.reset_selected_image_adjustments();
            Handled
        }
        A::OpenSelectedIconPicker => {
            // Property-panel icon section → replace-selection picker.
            let ui = &mut state.editor_ui;
            ui.open_icon_picker(true);
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetTextAlign(value) => {
            layout_ops::set_selected_text_align(state, *value);
            Handled
        }
        A::SetTextVerticalAlign(value) => {
            layout_ops::set_selected_text_vertical_align(state, *value);
            Handled
        }
        A::SetTextGrowth(value) => {
            layout_ops::set_selected_text_growth(state, *value);
            Handled
        }
        A::ToggleFontFamilyPicker => {
            let opening = !state.editor_ui.font_picker.open;
            let ui = &mut state.editor_ui;
            ui.toggle_font_picker();
            ui.font_weight_picker_open = false;
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            if opening {
                FollowUp(F::EnsureSystemFontsLoaded)
            } else {
                Handled
            }
        }
        A::SetFontFamilyIndex(index) => {
            if let Some(family) = font_picker_family_at(state, *index) {
                layout_ops::set_selected_text_font_family(state, &family);
            }
            state.editor_ui.close_font_picker();
            Handled
        }
        A::ImportFont => {
            // Raise a pending request; each host drains it (native pops
            // the file dialog + registers with FontStore, web opens the
            // hidden file input + persists to IndexedDB). Keep the
            // picker open so the new family appears once it lands.
            state.editor_ui.pending_font_import = true;
            Handled
        }
        A::RemoveImportedFont(index) => {
            // Resolve the family against the SAME entries list the
            // picker painted / hit-tested, then hand it to the host
            // drain to drop from the registry + store.
            if let Some(family) = font_picker_family_at(state, *index) {
                state.editor_ui.pending_font_remove = Some(family);
            }
            Handled
        }
        A::RelinkImage => {
            state.editor_ui.pending_file_action =
                Some(op_editor_core::editor_ui_state::FileAction::RelinkImage);
            Handled
        }
        A::ToggleFontWeightPicker => {
            let ui = &mut state.editor_ui;
            ui.font_weight_picker_open = !ui.font_weight_picker_open;
            ui.font_weight_picker_hover = None;
            ui.close_font_picker();
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetFontWeight(choice) => {
            layout_ops::set_selected_font_weight(state, choice.value());
            state.editor_ui.font_weight_picker_open = false;
            state.editor_ui.font_weight_picker_hover = None;
            Handled
        }
        A::TogglePaddingModePopover => {
            let ui = &mut state.editor_ui;
            ui.padding_mode_popover_open = !ui.padding_mode_popover_open;
            ui.padding_mode_popover_hover = None;
            ui.stroke_mode_popover_open = false;
            ui.stroke_mode_popover_hover = None;
            ui.font_weight_picker_open = false;
            ui.close_font_picker();
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetPaddingMode(mode) => {
            // Scope the pin to the node it was set for so it can't leak
            // into the next selection.
            let anchor = state.selection.anchor.as_str().to_string();
            state.editor_ui.padding_edit_mode = Some(*mode);
            state.editor_ui.padding_edit_mode_anchor = anchor;
            state.editor_ui.padding_mode_popover_open = false;
            state.editor_ui.padding_mode_popover_hover = None;
            state.commit_history();
            let _ = state.set_selected_padding_mode_shape(*mode);
            Handled
        }
        A::ToggleStrokeModePopover => {
            let ui = &mut state.editor_ui;
            ui.stroke_mode_popover_open = !ui.stroke_mode_popover_open;
            ui.stroke_mode_popover_hover = None;
            ui.padding_mode_popover_open = false;
            ui.padding_mode_popover_hover = None;
            ui.font_weight_picker_open = false;
            ui.close_font_picker();
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetStrokeMode(mode) => {
            let anchor = state.selection.anchor.as_str().to_string();
            state.editor_ui.stroke_edit_mode = Some(*mode);
            state.editor_ui.stroke_edit_mode_anchor = anchor;
            state.editor_ui.stroke_mode_popover_open = false;
            state.editor_ui.stroke_mode_popover_hover = None;
            state.commit_history();
            let _ = state.set_selected_stroke_mode_shape(*mode);
            Handled
        }
        A::OpenColorPicker(target) => {
            // Fallback anchor when called outside the press path.
            state.editor_ui.property_color_variable_picker_open = None;
            let _ = state.open_color_picker(color_target(*target), 0.0);
            Handled
        }
        A::OpenFillColorPicker(index) => {
            // Fallback anchor when called outside the press path.
            state.editor_ui.property_color_variable_picker_open = None;
            let _ = state.open_color_picker_for_fill(
                op_editor_core::ui_draft::ColorTarget::Fill,
                *index,
                0.0,
            );
            Handled
        }
        A::ToggleColorVariablePicker(target) => {
            let target = color_target(*target);
            let ui = &mut state.editor_ui;
            ui.property_color_variable_picker_open =
                if ui.property_color_variable_picker_open == Some(target) {
                    None
                } else {
                    // Each open starts at the top of the list.
                    ui.property_color_variable_picker_scroll.offset = 0.0;
                    Some(target)
                };
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            Handled
        }
        A::BindColorVariable { target, index } => {
            if let Some(name) = color_variable_name_at(state, *index) {
                state.commit_history();
                let _ = state.bind_selected_color_variable(color_target(*target), &name);
            }
            state.editor_ui.property_color_variable_picker_open = None;
            Handled
        }
        A::UnbindColorVariable(target) => {
            state.commit_history();
            let _ = state.unbind_selected_color_variable(color_target(*target));
            state.editor_ui.property_color_variable_picker_open = None;
            Handled
        }
        A::ToggleExportScalePicker => {
            let ui = &mut state.editor_ui;
            ui.export_scale_picker_open = !ui.export_scale_picker_open;
            ui.export_format_picker_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_picker_hover = None;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::ToggleExportFormatPicker => {
            let ui = &mut state.editor_ui;
            ui.export_format_picker_open = !ui.export_format_picker_open;
            ui.export_scale_picker_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_picker_hover = None;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetExportScale(scale) => {
            let ui = &mut state.editor_ui;
            ui.export_scale = *scale;
            ui.export_scale_picker_open = false;
            ui.export_picker_hover = None;
            Handled
        }
        A::SetExportFormat(format) => {
            let ui = &mut state.editor_ui;
            ui.export_format = *format;
            ui.export_format_picker_open = false;
            ui.export_picker_hover = None;
            Handled
        }
        A::ExportImageNow => {
            state.editor_ui.pending_file_action =
                Some(op_editor_core::editor_ui_state::FileAction::ExportImageConfirm);
            Handled
        }
        A::ToggleEffectAddPicker => {
            state.editor_ui.toggle_effect_add_picker();
            Handled
        }
        A::AddEffect(kind) => {
            use crate::widgets::property_panel_snapshot::EffectKind;
            match kind {
                EffectKind::Shadow => {
                    state.add_drop_shadow_to_selected();
                }
                EffectKind::LayerBlur => {
                    state.add_layer_blur_to_selected();
                }
                EffectKind::BackgroundBlur => {
                    state.add_background_blur_to_selected();
                }
            }
            state.editor_ui.close_effect_add_picker();
            Handled
        }
        A::SetEffectVisible(index, visible) => {
            let _ = state.set_selected_effect_visible(*index, *visible);
            Handled
        }
        A::RemoveEffect(index) => {
            let _ = state.remove_selected_effect(*index);
            Handled
        }
        A::AdjustEffectParam {
            effect,
            field,
            new_value,
        } => {
            let id = state.selection.anchor.clone();
            if id.is_real() {
                if ctx.effect_radius_drag.is_none() {
                    state.commit_history();
                }
                *ctx.effect_radius_drag = Some(*effect);
                let _ = state.apply(op_editor_core::EditorCommand::SetEffectParam {
                    node_id: id,
                    index: *effect as u32,
                    field: *field,
                    value: *new_value,
                });
            }
            Handled
        }
        // The seeding half is shared (`focus_effect_param`), but the
        // host has to commit any pending property / variable-row draft
        // first — those commits live host-side.
        A::FocusEffectParam { .. } => HostOwned,
        A::MatchImageAspectRatio => HostOwned,
        A::OpenEffectColorPicker(index) => {
            let _ = state.open_color_picker(
                op_editor_core::ui_draft::ColorTarget::EffectColor(*index),
                0.0,
            );
            Handled
        }
        A::ToggleWidgetChecked(new_value) => {
            state.commit_history();
            let _ = state.set_selected_widget_checked(*new_value);
            Handled
        }
        A::PickFillImage => {
            // Queue the file picker — each host drains this flag once
            // the event handler released the host borrow and writes the
            // chosen image into the selected node's primary fill.
            state.editor_ui.pending_file_action =
                Some(op_editor_core::editor_ui_state::FileAction::PickFillImage);
            Handled
        }
        A::ToggleInteractionMenu => {
            let ui = &mut state.editor_ui;
            ui.toggle_interaction_menu();
            ui.close_fill_type_picker();
            ui.image_fill_popover_open = false;
            ui.close_font_picker();
            ui.font_weight_picker_open = false;
            ui.export_scale_picker_open = false;
            ui.export_format_picker_open = false;
            ui.property_color_variable_picker_open = None;
            Handled
        }
        A::SetInteractionNavigate { path } => {
            let node_id = state.selection.anchor.clone();
            if node_id.is_real() {
                state.commit_history();
                let patch_json =
                    crate::widgets::property_panel_interactions::navigate_patch_json(path.as_str());
                let _ = state.apply(op_editor_core::EditorCommand::PatchNodeData {
                    node_id,
                    patch_json,
                    page_id: None,
                });
            }
            state.editor_ui.close_interaction_menu();
            Handled
        }
        A::SetInteractionPop => {
            let node_id = state.selection.anchor.clone();
            if node_id.is_real() {
                state.commit_history();
                let _ = state.apply(op_editor_core::EditorCommand::PatchNodeData {
                    node_id,
                    patch_json: crate::widgets::property_panel_interactions::POP_PATCH_JSON
                        .to_string(),
                    page_id: None,
                });
            }
            state.editor_ui.close_interaction_menu();
            Handled
        }
        A::RemoveInteraction => {
            let node_id = state.selection.anchor.clone();
            if node_id.is_real() {
                // Clear only `onTap` — if the node's `events` block
                // carries no other handler afterward, drop the whole
                // field (no `"events":{}` shell left behind); else
                // re-serialize the trimmed block so any sibling handler
                // (`onChange`, …) survives untouched.
                use op_editor_core::pen_node_ext::PenNodeExt;
                let mut handlers = state
                    .selected_node()
                    .and_then(|n| n.events().cloned())
                    .unwrap_or_default();
                handlers.on_tap = None;
                let patch_json = if handlers == jian_ops_schema::events::EventHandlers::default() {
                    r#"{"events":null}"#.to_string()
                } else {
                    let value = serde_json::to_value(&handlers).unwrap_or(serde_json::Value::Null);
                    format!(r#"{{"events":{value}}}"#)
                };
                state.commit_history();
                let _ = state.apply(op_editor_core::EditorCommand::PatchNodeData {
                    node_id,
                    patch_json,
                    page_id: None,
                });
            }
            state.editor_ui.close_interaction_menu();
            Handled
        }
        // Image Search / Generate popovers ride host-owned selection
        // drag + input-blur glue.
        A::ToggleImageSearchPopover
        | A::ToggleImageGeneratePopover
        | A::RunImageSearch
        | A::SelectImageSearchResult(_)
        | A::RunImageGenerate
        | A::ApplyGeneratedImage
        | A::RetryImageGenerate
        | A::OpenImageGenSettings => HostOwned,
        // Clipboard / download / bundle export are platform IO.
        A::Codegen(_) => HostOwned,
    }
}
