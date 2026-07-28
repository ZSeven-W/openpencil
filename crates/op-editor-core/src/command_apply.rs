//! [`EditorState::apply`] — apply one [`EditorCommand`] against the
//! editor state.
//!
//! Ported from `openpencil-shell-core::document::apply_mcp_command`.
//! Preserves the two shell-core invariants:
//!
//!   - **Pre-validate-then-mutate.** Every argument (id space, target
//!     existence, geometry, hex, container-children consent) is checked
//!     BEFORE any tree write, so a bad arg never half-mutates the
//!     document. The raw-node helpers in [`crate::command_node`] and
//!     the attribute helpers in [`crate::command_node_attrs`] keep that
//!     discipline internally.
//!   - **`ReplaceNode` destructive-swap guard.** Replacing a node WITH
//!     children requires `drop_children == true`.
//!
//! The result type is `bool` — identical to shell-core's
//! `apply_mcp_command`: `true` when the command changed something (so
//! a host can decide whether to push undo / persist), `false` on an
//! apply-time validation failure. **Exception:** [`EditorState::
//! merge_app_state`] (`MergeAppState`) reports "processed", not
//! "changed" — see its doc comment for why a no-op merge must still
//! return `true`.
//!
//! ### Module layout
//!
//! This file is the spine: [`EditorState::apply`] itself, the single
//! `match` over every [`EditorCommand`] variant. The supporting code
//! lives in sibling submodules (per the 800-line-per-file ceiling):
//!
//! - `helpers` — enum-string parsers, the dirty-marking classifier,
//!   page-index resolution and the active-page insert shims
//! - `app_state` — the `MergeAppState` merge with its ownership rules
//!
use crate::align::AlignAction;
use crate::command::{EditorCommand, VariableScalarPayload};
use crate::id_allocator::{IdAllocError, IdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::tool::Tool;
use crate::viewport::Viewport;
use crate::walkers::find_node;
use jian_ops_schema::conversion::{ConversionEntry, ConversionKind};
use jian_ops_schema::variable::{VariableKind, VariableScalar};

mod app_state;
mod helpers;

pub(crate) use helpers::command_marks_document_dirty;
use helpers::{
    apply_import_svg_on_active_page, apply_insert_node_on_active_page, apply_kit_component_on_page,
    command_page_index, parse_align_action, parse_tool, parse_variable_kind,
};

impl EditorState {
    /// Apply a command using the caller's document-wide id policy.
    ///
    /// Collaboration hosts pass their session-owned namespaced allocator.
    /// Validation failures remain `Ok(false)`; allocator exhaustion is typed
    /// so the host can roll back the surrounding local edit transaction.
    pub fn apply_with_allocator(
        &mut self,
        cmd: EditorCommand,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        let marks_document_dirty = command_marks_document_dirty(&cmd);
        let revision_before = self.revision;
        let changed = match cmd {
            // --- Raw node CRUD -------------------------------------
            EditorCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                target_parent,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = apply_insert_node_on_active_page(
                    self,
                    &kind,
                    &name,
                    x,
                    y,
                    width,
                    height,
                    &fill_hex,
                    &target_parent,
                    allocator,
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed?
            }
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_update_node(&node_id, x, y, width, height, &name, &fill_hex);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::PatchNodeData {
                node_id,
                patch_json,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_patch_node_data(&node_id, &patch_json);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::DeleteNode { node_id, page_id } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_delete_node(&node_id);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                page_id,
                index,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_move_node(&node_id, &target_parent, index);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_copy_node_with_allocator(
                    &node_id,
                    &target_parent,
                    overrides_json.as_deref(),
                    allocator,
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed?
            }
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                drop_children,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_replace_node_with_allocator(
                    &node_id,
                    &kind,
                    &name,
                    x,
                    y,
                    width,
                    height,
                    &fill_hex,
                    drop_children,
                    allocator,
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed?
            }
            EditorCommand::ReplaceSubtree {
                node_id,
                node,
                drop_children,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_replace_subtree_with_allocator(
                    &node_id,
                    *node,
                    drop_children,
                    allocator,
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed?
            }
            EditorCommand::BatchInsert { items, page_id } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_batch_insert_with_allocator(&items, allocator);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed?
            }
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let snap = self.snapshot_for_history();
                let inserted = self.cmd_insert_subtree_with_allocator(nodes, &parent_id, allocator);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                if inserted? {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let snap = self.snapshot_for_history();
                let changed = if self.cmd_insert_authored_subtree(nodes, &parent_id) {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                };
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::RefineDesign {
                root_id,
                canvas_width,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let snap = self.snapshot_for_history();
                let refined =
                    self.cmd_refine_design_with_allocator(&root_id, canvas_width, allocator);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                match refined? {
                    Some(changed) => {
                        if changed {
                            self.history_push_past(snap);
                        }
                        true
                    }
                    None => false,
                }
            }

            // --- Per-node attribute writers ------------------------
            EditorCommand::SetNodeRotation { node_id, degrees } => {
                self.cmd_set_node_rotation(&node_id, degrees)
            }
            EditorCommand::SetNodeText { node_id, text } => self.cmd_set_node_text(&node_id, &text),
            EditorCommand::SetNodeCornerRadius { node_id, radius } => {
                self.cmd_set_node_corner_radius(&node_id, radius)
            }
            EditorCommand::SetNodeFontSize { node_id, font_size } => {
                self.cmd_set_node_font_size(&node_id, font_size)
            }
            EditorCommand::SetNodeFontWeight {
                node_id,
                font_weight,
            } => self.cmd_set_node_font_weight(&node_id, font_weight),
            EditorCommand::SetNodeStrokeHex { node_id, hex } => {
                self.cmd_set_node_stroke_hex(&node_id, &hex)
            }
            EditorCommand::SetNodeStrokeWidth { node_id, width } => {
                self.cmd_set_node_stroke_width(&node_id, width)
            }
            EditorCommand::SetNodeStrokeSideWidth {
                node_id,
                side,
                width,
            } => self.cmd_set_node_stroke_side_width(&node_id, side, width),
            EditorCommand::SetNodeFillHex { node_id, hex } => {
                self.cmd_set_node_fill_hex(&node_id, &hex)
            }
            EditorCommand::SetNodeName { node_id, name } => self.cmd_set_node_name(&node_id, &name),
            EditorCommand::SetNodeFlag {
                node_id,
                flag,
                value,
            } => self.cmd_set_node_flag(&node_id, flag, value),
            EditorCommand::SetNodeFlip {
                node_id,
                flip_x,
                flip_y,
            } => self.cmd_set_node_flip(&node_id, flip_x, flip_y),
            EditorCommand::SetEllipseArc {
                node_id,
                start_angle,
                sweep_angle,
                inner_radius,
            } => self.cmd_set_ellipse_arc(&node_id, start_angle, sweep_angle, inner_radius),
            EditorCommand::AddNodeEffect { node_id, kind } => {
                self.cmd_add_node_effect(&node_id, &kind)
            }
            EditorCommand::RemoveNodeEffect { node_id, index } => {
                self.cmd_remove_node_effect(&node_id, index)
            }
            EditorCommand::SetEffectParam {
                node_id,
                index,
                field,
                value,
            } => self.cmd_set_effect_param(&node_id, index, field, value),
            EditorCommand::SetEffectColor {
                node_id,
                index,
                hex,
            } => self.cmd_set_effect_color(&node_id, index, &hex),

            // --- Variables + themes --------------------------------
            EditorCommand::SetVariableColor { name, hex } => self.set_variable_color(&name, &hex),
            EditorCommand::SetVariableScalar { name, scalar } => match scalar {
                VariableScalarPayload::Number(n) => self.set_variable_number(&name, n),
                VariableScalarPayload::String(s) => self.set_variable_string(&name, s),
                VariableScalarPayload::Boolean(b) => self.set_variable_boolean(&name, b),
            },
            EditorCommand::CreateVariable {
                name,
                kind,
                default_value,
            } => {
                let Some(kind) = parse_variable_kind(&kind) else {
                    return Ok(false);
                };
                // The default value is parsed per kind; a bad value
                // (non-numeric Number, unparseable Boolean) rejects.
                let default = match kind {
                    VariableKind::Color | VariableKind::String => {
                        VariableScalar::Str(default_value)
                    }
                    VariableKind::Number => match default_value.trim().parse::<f64>() {
                        Ok(n) => VariableScalar::Num(n),
                        Err(_) => return Ok(false),
                    },
                    VariableKind::Boolean => match default_value.trim() {
                        "true" => VariableScalar::Bool(true),
                        "false" => VariableScalar::Bool(false),
                        _ => return Ok(false),
                    },
                };
                self.create_variable(&name, kind, default)
            }
            EditorCommand::DeleteVariable { name } => self.delete_variable(&name),
            EditorCommand::RenameVariable { old_name, new_name } => {
                self.rename_variable(&old_name, &new_name)
            }
            EditorCommand::SetVariables { variables, replace } => {
                self.set_variables_bulk(variables, replace)
            }
            EditorCommand::UpsertVariables {
                variables,
                key,
                source_path,
                source_hash,
            } => {
                if variables.is_empty() {
                    return Ok(false);
                }
                self.set_variables_bulk(variables, false);
                crate::conversion::upsert_conversion_entry(
                    &mut self.doc,
                    ConversionEntry {
                        kind: ConversionKind::Token,
                        key,
                        source_path,
                        source_hash,
                        node_id: None,
                        node_ids: None,
                    },
                );
                true
            }
            EditorCommand::SetThemes { themes, replace } => self.set_themes_bulk(themes, replace),
            EditorCommand::MergeThemePreset { variables, themes } => {
                self.set_variables_bulk(variables, false) && self.set_themes_bulk(themes, false)
            }
            EditorCommand::SetDesignMd { spec } => {
                self.doc.design_md = Some(*spec);
                true
            }
            EditorCommand::UpsertComponent {
                key,
                name,
                root,
                source_path,
                source_hash,
            } => crate::conversion::upsert_component_with_allocator(
                self,
                key,
                name,
                *root,
                source_path,
                source_hash,
                allocator,
            )?,
            EditorCommand::UpsertScreen {
                key,
                root,
                source_path,
                source_hash,
            } => crate::conversion::upsert_screen_with_allocator(
                self,
                key,
                *root,
                source_path,
                source_hash,
                allocator,
            )?,
            EditorCommand::SetActiveAxisValue { axis, value } => {
                self.set_active_axis_value(&axis, &value)
            }
            EditorCommand::CycleActiveAxisValue { axis } => self.cycle_active_axis_value(&axis),

            // --- Pages ---------------------------------------------
            EditorCommand::SetActivePage { index } => self.set_active_page(index as usize),
            EditorCommand::AddPage { name, children } => self
                .add_page_with_allocator(name, children, allocator)?
                .is_some(),
            EditorCommand::RenamePage { index, name } => self.rename_page(index as usize, name),
            EditorCommand::DeletePage { index } => self.remove_page(index as usize),
            EditorCommand::DuplicatePage { index, name } => self
                .duplicate_page_with_allocator(index as usize, name, allocator)?
                .is_some(),
            EditorCommand::ReorderPage { from, to } => {
                self.reorder_page(from as usize, to as usize)
            }

            // --- Selection -----------------------------------------
            EditorCommand::ClearSelection => {
                self.clear_selection();
                true
            }
            EditorCommand::SetSelection { node_id } => {
                // Scoped to the active page — parity with shell-core,
                // which rejected off-page ids so later reads stay
                // consistent.
                if !node_id.is_real() || find_node(self.active_children(), &node_id).is_none() {
                    return Ok(false);
                }
                self.set_single_selection(node_id);
                true
            }
            EditorCommand::SetSelectionSet { node_ids } => {
                // Resolve every id against the active page; unknown /
                // off-page ids are dropped silently.
                let resolved: Vec<NodeId> = node_ids
                    .into_iter()
                    .filter(|id| id.is_real() && find_node(self.active_children(), id).is_some())
                    .collect();
                if resolved.is_empty() {
                    self.clear_selection();
                } else {
                    let anchor = resolved.last().cloned().unwrap();
                    if self.selection.anchor != anchor || self.selection.set != resolved {
                        self.editor_ui.image_panel.close_popovers();
                    }
                    self.selection.anchor = anchor;
                    self.selection.set = resolved;
                }
                true
            }
            EditorCommand::ToggleNodeSelection { node_id } => {
                if !node_id.is_real() || find_node(self.active_children(), &node_id).is_none() {
                    return Ok(false);
                }
                self.toggle_selection(node_id);
                true
            }

            // --- Selection-scoped tree ops -------------------------
            EditorCommand::DuplicateSelected { offset_px } => self
                .duplicate_selected_with_allocator(allocator, offset_px as f64)?
                .is_some(),
            EditorCommand::DeleteSelected => {
                if self.selection.set.is_empty() {
                    return Ok(false);
                }
                let snap = self.snapshot_for_history();
                if self.delete_selected() {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::NudgeSelected { dx, dy } => {
                if self.selection.set.is_empty() || (dx == 0 && dy == 0) {
                    return Ok(false);
                }
                let snap = self.snapshot_for_history();
                if self.translate_selected(dx as f64, dy as f64) {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::GroupSelected => {
                let snap = self.snapshot_for_history();
                if self.group_selected_with_allocator(allocator)?.is_some() {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::UngroupSelected => {
                let snap = self.snapshot_for_history();
                if self.ungroup_selected() {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::ReorderSelected { direction } => {
                if !self.selection.anchor.is_real() {
                    return Ok(false);
                }
                let snap = self.snapshot_for_history();
                if self.reorder_selected(direction) {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::AlignSelected { action } => {
                let Some(parsed) = parse_align_action(&action) else {
                    return Ok(false);
                };
                // `align_selected` pushes its own history on real
                // motion.
                self.align_selected(parsed)
            }

            // --- Clipboard -----------------------------------------
            EditorCommand::CopySelected => self.copy_selected(),
            EditorCommand::CutSelected => {
                let snap = self.snapshot_for_history();
                if self.cut_selected() {
                    self.history_push_past(snap);
                    true
                } else {
                    false
                }
            }
            EditorCommand::PasteClipboard { offset_px } => {
                let snap = self.snapshot_for_history();
                let new_ids = self.paste_clipboard_with_allocator(allocator, offset_px as f64)?;
                if new_ids.is_empty() {
                    return Ok(false);
                }
                self.history_push_past(snap);
                true
            }
            EditorCommand::ImportSvg {
                svg,
                x,
                y,
                target_parent,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return Ok(false);
                };
                let original_page_index = self.ui.active_page_index;
                let original_selection = self.selection.clone();
                let cross_page = page_id.is_some() && target_page_index != original_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed =
                    apply_import_svg_on_active_page(self, &svg, x, y, &target_parent, allocator);
                if cross_page {
                    self.ui.active_page_index = original_page_index;
                    self.selection = original_selection.clone();
                    if matches!(&changed, Ok(true)) {
                        if let Some(snapshot) = self.history.past.back_mut() {
                            snapshot.active_page_index = original_page_index;
                            snapshot.selection = original_selection;
                        }
                    }
                }
                changed?
            }

            // --- Tool + viewport + history -------------------------
            EditorCommand::SetActiveTool { tool } => {
                let Some(new_tool) = parse_tool(&tool) else {
                    return Ok(false);
                };
                self.tool = new_tool;
                true
            }
            EditorCommand::SetViewport {
                pan_x,
                pan_y,
                zoom_percent,
            } => {
                let mut changed = false;
                if let Some(x) = pan_x {
                    self.viewport.pan_x = x as f32;
                    changed = true;
                }
                if let Some(y) = pan_y {
                    self.viewport.pan_y = y as f32;
                    changed = true;
                }
                if let Some(z) = zoom_percent {
                    let zoom = (z as f32 / 100.0).clamp(Viewport::MIN_ZOOM, Viewport::MAX_ZOOM);
                    self.viewport.zoom = zoom;
                    changed = true;
                }
                changed
            }
            EditorCommand::Undo => self.undo(),
            EditorCommand::Redo => self.redo(),

            // --- Component commands -------------------------------
            EditorCommand::InstantiateComponent { component_id } => self
                .instantiate_component_with_allocator(&component_id, allocator)?
                .is_some(),
            EditorCommand::CreateComponent { node_id, name } => {
                self.create_component_from_node(&node_id, &name)
            }
            EditorCommand::DeleteComponent { component_id } => self.delete_component(&component_id),
            EditorCommand::RenameComponent { component_id, name } => {
                self.rename_component(&component_id, &name)
            }

            // --- UIKit element insert -------------------------------
            EditorCommand::InstantiateKitComponent {
                kit_id,
                component_id,
                doc_x,
                doc_y,
                target_parent,
                page_id,
                overrides_json,
            } => apply_kit_component_on_page(
                self,
                &kit_id,
                &component_id,
                &target_parent,
                doc_x.unwrap_or(0.0),
                doc_y.unwrap_or(0.0),
                overrides_json.as_deref(),
                page_id.as_deref(),
                allocator,
            )?,

            // --- Layout / text property writer ----------------------
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value,
            } => self.cmd_set_node_layout_prop(&node_id, &property, &value),
            EditorCommand::ReplaceFontFamily { from, to } => {
                self.replace_font_family_everywhere(&from, &to) > 0
            }
            EditorCommand::ReplaceAllMatchingProperties {
                page_id,
                parent_ids,
                replacements,
            } => self.cmd_replace_all_matching_properties(&page_id, &parent_ids, &replacements),
            EditorCommand::Batch { commands } => {
                self.cmd_batch_with_allocator(commands, allocator)?
            }
            EditorCommand::MergeAppState { plan_idx, state } => {
                self.merge_app_state(plan_idx, state)
            }
            // `promote_legacy_widgets` owns its history snapshot — it
            // pushes onto the undo stack only when at least one frame is
            // promoted, so a zero-promotion run is a clean no-op. The
            // promotion count + per-node notes are surfaced by the
            // dedicated method; here `apply` reports only changed-or-not.
            EditorCommand::PromoteLegacyWidgets => self.promote_legacy_widgets().changed(),
        };
        if changed && marks_document_dirty && self.revision == revision_before {
            self.mark_document_changed();
        }
        Ok(changed)
    }
}
