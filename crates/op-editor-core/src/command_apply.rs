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
use crate::align::AlignAction;
use crate::command::{EditorCommand, VariableScalarPayload};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::tool::Tool;
use crate::viewport::Viewport;
use crate::walkers::find_node;
use jian_ops_schema::conversion::{ConversionEntry, ConversionKind};
use jian_ops_schema::variable::{VariableKind, VariableScalar};

/// Resolve an `align` action string into an [`AlignAction`].
fn parse_align_action(s: &str) -> Option<AlignAction> {
    match s {
        "left" => Some(AlignAction::Left),
        "center_h" => Some(AlignAction::CenterH),
        "right" => Some(AlignAction::Right),
        "top" => Some(AlignAction::Top),
        "center_v" => Some(AlignAction::CenterV),
        "bottom" => Some(AlignAction::Bottom),
        "distribute_h" => Some(AlignAction::DistributeH),
        "distribute_v" => Some(AlignAction::DistributeV),
        _ => None,
    }
}

/// Resolve a `tool` string into a [`Tool`]. Accepts each tool's
/// stable [`Tool::ident`] token, so the form-widget tools select via
/// their `snake_case` kind string (`text_input`, `slider`, …; the
/// dropdown select widget uses `select_widget` to disambiguate from
/// the `select` pointer tool).
fn parse_tool(s: &str) -> Option<Tool> {
    match s {
        "select" => Some(Tool::Select),
        "rect" => Some(Tool::Rect),
        "ellipse" => Some(Tool::Ellipse),
        "polygon" => Some(Tool::Polygon),
        "line" => Some(Tool::Line),
        "pen" => Some(Tool::Pen),
        "text" => Some(Tool::Text),
        "frame" => Some(Tool::Frame),
        "hand" => Some(Tool::Hand),
        "text_input" => Some(Tool::TextInput),
        "text_area" => Some(Tool::TextArea),
        "number_input" => Some(Tool::NumberInput),
        "select_widget" => Some(Tool::Select_),
        "radio_group" => Some(Tool::RadioGroup),
        "switch" => Some(Tool::Switch),
        "checkbox" => Some(Tool::Checkbox),
        "slider" => Some(Tool::Slider),
        "progress" => Some(Tool::Progress),
        "tabs" => Some(Tool::Tabs),
        _ => None,
    }
}

/// Resolve a variable `kind` string into a [`VariableKind`].
fn parse_variable_kind(s: &str) -> Option<VariableKind> {
    match s {
        "color" => Some(VariableKind::Color),
        "number" => Some(VariableKind::Number),
        "boolean" => Some(VariableKind::Boolean),
        "string" => Some(VariableKind::String),
        _ => None,
    }
}

fn command_page_index(state: &EditorState, page_id: Option<&str>) -> Option<usize> {
    let Some(raw) = page_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Some(
            state
                .ui
                .active_page_index
                .min(state.page_count().saturating_sub(1)),
        );
    };
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages
            .iter()
            .position(|page| page.id == raw)
            .or_else(|| raw.parse::<usize>().ok().filter(|idx| *idx < pages.len())),
        _ => raw.parse::<usize>().ok().filter(|idx| *idx == 0),
    }
}

pub(crate) fn command_marks_document_dirty(cmd: &EditorCommand) -> bool {
    use EditorCommand as C;
    if let C::Batch { commands } = cmd {
        return commands.iter().any(command_marks_document_dirty);
    }
    !matches!(
        cmd,
        C::SetActiveTool { .. }
            | C::SetViewport { .. }
            | C::Undo
            | C::Redo
            | C::CopySelected
            | C::ClearSelection
            | C::SetSelection { .. }
            | C::SetSelectionSet { .. }
            | C::ToggleNodeSelection { .. }
            | C::SetActivePage { .. }
            | C::SetActiveAxisValue { .. }
            | C::CycleActiveAxisValue { .. }
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_insert_node_on_active_page(
    state: &mut EditorState,
    kind: &str,
    name: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill_hex: &Option<String>,
    target_parent: &NodeId,
) -> bool {
    state.cmd_insert_node(kind, name, x, y, width, height, fill_hex, target_parent)
}

fn apply_import_svg_on_active_page(
    state: &mut EditorState,
    svg: &str,
    x: i32,
    y: i32,
    target_parent: &NodeId,
) -> bool {
    let Some(mut next_id) = state.next_node_id_seed() else {
        return false;
    };
    if target_parent.is_real() {
        match find_node(state.active_children(), target_parent) {
            Some(parent) if parent.is_container() => {}
            _ => return false,
        }
    }
    // `import_svg` pushes its own history snapshot when it inserts ≥ 1
    // node.
    let count = state.import_svg(&mut next_id, svg, (x as f64, y as f64));
    if count == 0 {
        return false;
    }
    if target_parent.is_real() {
        let Some(imported_root) = state
            .active_children()
            .last()
            .map(|node| NodeId::new(node.id_str()))
        else {
            return false;
        };
        imported_root.is_real() && state.cmd_move_node(&imported_root, target_parent, None)
    } else {
        true
    }
}

impl EditorState {
    /// Apply one [`EditorCommand`]. Returns `true` when the command was
    /// processed (which for most commands means it changed the document
    /// / editor state), `false` on an apply-time validation failure.
    /// Exception: an additive [`EditorCommand::MergeAppState`] whose
    /// every key defers to an existing owner is a designed no-op and
    /// still returns `true` — see [`Self::merge_app_state`].
    pub fn apply(&mut self, cmd: EditorCommand) -> bool {
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
                    return false;
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
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
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
                    return false;
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
                    return false;
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
                    return false;
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
                    return false;
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
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed =
                    self.cmd_copy_node(&node_id, &target_parent, overrides_json.as_deref());
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
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
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_replace_node(
                    &node_id,
                    &kind,
                    &name,
                    x,
                    y,
                    width,
                    height,
                    &fill_hex,
                    drop_children,
                );
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::ReplaceSubtree {
                node_id,
                node,
                drop_children,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_replace_subtree(&node_id, *node, drop_children);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::BatchInsert { items, page_id } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self.cmd_batch_insert(&items);
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                changed
            }
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let snap = self.snapshot_for_history();
                let changed = if self.cmd_insert_subtree(nodes, &parent_id) {
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
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id,
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return false;
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
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let snap = self.snapshot_for_history();
                let accepted = match self.cmd_refine_design(&root_id, canvas_width) {
                    Some(changed) => {
                        if changed {
                            self.history_push_past(snap);
                        }
                        true
                    }
                    None => false,
                };
                if page_id.is_some() && target_page_index != original_page_index {
                    self.ui.active_page_index = original_page_index;
                }
                accepted
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
                    return false;
                };
                // The default value is parsed per kind; a bad value
                // (non-numeric Number, unparseable Boolean) rejects.
                let default = match kind {
                    VariableKind::Color | VariableKind::String => {
                        VariableScalar::Str(default_value)
                    }
                    VariableKind::Number => match default_value.trim().parse::<f64>() {
                        Ok(n) => VariableScalar::Num(n),
                        Err(_) => return false,
                    },
                    VariableKind::Boolean => match default_value.trim() {
                        "true" => VariableScalar::Bool(true),
                        "false" => VariableScalar::Bool(false),
                        _ => return false,
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
                    return false;
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
            } => crate::conversion::upsert_component(
                self,
                key,
                name,
                *root,
                source_path,
                source_hash,
            ),
            EditorCommand::UpsertScreen {
                key,
                root,
                source_path,
                source_hash,
            } => crate::conversion::upsert_screen(self, key, *root, source_path, source_hash),
            EditorCommand::SetActiveAxisValue { axis, value } => {
                self.set_active_axis_value(&axis, &value)
            }
            EditorCommand::CycleActiveAxisValue { axis } => self.cycle_active_axis_value(&axis),

            // --- Pages ---------------------------------------------
            EditorCommand::SetActivePage { index } => self.set_active_page(index as usize),
            EditorCommand::AddPage { name, children } => self
                .add_page_with_name_and_children(name, children)
                .is_some(),
            EditorCommand::RenamePage { index, name } => self.rename_page(index as usize, name),
            EditorCommand::DeletePage { index } => self.remove_page(index as usize),
            EditorCommand::DuplicatePage { index, name } => self
                .duplicate_page_with_name(index as usize, name)
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
                    return false;
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
                    self.selection.anchor = resolved.last().cloned().unwrap();
                    self.selection.set = resolved;
                }
                true
            }
            EditorCommand::ToggleNodeSelection { node_id } => {
                if !node_id.is_real() || find_node(self.active_children(), &node_id).is_none() {
                    return false;
                }
                self.toggle_selection(node_id);
                true
            }

            // --- Selection-scoped tree ops -------------------------
            EditorCommand::DuplicateSelected { offset_px } => {
                let Some(mut next_id) = self.next_node_id_seed() else {
                    return false;
                };
                self.duplicate_selected(&mut next_id, offset_px as f64)
                    .is_some()
            }
            EditorCommand::DeleteSelected => {
                if self.selection.set.is_empty() {
                    return false;
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
                    return false;
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
                let Some(mut next_id) = self.next_node_id_seed() else {
                    return false;
                };
                let snap = self.snapshot_for_history();
                if self.group_selected(&mut next_id).is_some() {
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
                    return false;
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
                    return false;
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
                let Some(mut next_id) = self.next_node_id_seed() else {
                    return false;
                };
                let snap = self.snapshot_for_history();
                let new_ids = self.paste_clipboard(&mut next_id, offset_px as f64);
                if new_ids.is_empty() {
                    return false;
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
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                let original_selection = self.selection.clone();
                let cross_page = page_id.is_some() && target_page_index != original_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = apply_import_svg_on_active_page(self, &svg, x, y, &target_parent);
                if cross_page {
                    self.ui.active_page_index = original_page_index;
                    self.selection = original_selection.clone();
                    if changed {
                        if let Some(snapshot) = self.history.past.back_mut() {
                            snapshot.active_page_index = original_page_index;
                            snapshot.selection = original_selection;
                        }
                    }
                }
                changed
            }

            // --- Tool + viewport + history -------------------------
            EditorCommand::SetActiveTool { tool } => {
                let Some(new_tool) = parse_tool(&tool) else {
                    return false;
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
            EditorCommand::InstantiateComponent { component_id } => {
                self.instantiate_component(&component_id).is_some()
            }
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
            } => {
                let Some(target_page_index) = command_page_index(self, page_id.as_deref()) else {
                    return false;
                };
                let original_page_index = self.ui.active_page_index;
                let original_selection = self.selection.clone();
                let cross_page = page_id.is_some() && target_page_index != original_page_index;
                if page_id.is_some() {
                    self.ui.active_page_index = target_page_index;
                }
                let changed = self
                    .instantiate_kit_component_under_parent(
                        &kit_id,
                        &component_id,
                        &target_parent,
                        doc_x.unwrap_or(0.0),
                        doc_y.unwrap_or(0.0),
                        overrides_json.as_deref(),
                    )
                    .is_some();
                if cross_page {
                    self.ui.active_page_index = original_page_index;
                    self.selection = original_selection.clone();
                    if changed {
                        if let Some(snapshot) = self.history.past.back_mut() {
                            snapshot.active_page_index = original_page_index;
                            snapshot.selection = original_selection;
                        }
                    }
                }
                changed
            }

            // --- Layout / text property writer ----------------------
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value,
            } => self.cmd_set_node_layout_prop(&node_id, &property, &value),
            EditorCommand::ReplaceAllMatchingProperties {
                page_id,
                parent_ids,
                replacements,
            } => self.cmd_replace_all_matching_properties(&page_id, &parent_ids, &replacements),
            EditorCommand::Batch { commands } => self.cmd_batch(commands),
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
        changed
    }

    /// Apply [`EditorCommand::MergeAppState`]. Backward-compat: never
    /// overwrites a key that already lived in the document root before
    /// this run; among generation-added keys the lower `plan_idx` wins.
    ///
    /// Order-independence is achieved via `self.app_state_owner`: a
    /// side map of `key → owning_plan_idx` for every key written during
    /// this session. On a new key the owner is recorded and the value is
    /// inserted. On a conflicting key the incoming `plan_idx` is compared
    /// to the registered owner; if it is strictly lower it replaces both
    /// the owner record and the document value.
    ///
    /// ## Return contract
    ///
    /// The return value signals **"command processed"**, not **"keys
    /// landed"**. `MergeAppState` is additive by design: doc-owned keys
    /// always win, and among generation-added keys the lower `plan_idx`
    /// wins. A run where every incoming key was skipped (already
    /// doc-owned, or lost the `plan_idx` ownership race) is the designed
    /// steady-state outcome, not a failure — it MUST return `true`.
    ///
    /// This matters beyond the local call site: `MergeAppState` rides
    /// inside `EditorCommand::Batch` alongside a node insert/replace on
    /// every generation path (`hoist_generation_state` +
    /// `with_hoisted_state` in `op-mcp`), and `Batch`'s apply loop
    /// (`command_batch.rs::cmd_batch`) treats the first sub-command that
    /// returns `false` as a hard failure and rolls the ENTIRE batch back.
    /// Returning `false` for a legitimate no-op merge would silently
    /// reject an otherwise-valid insert/replace every time a regenerated
    /// section declares a state key the document root already carries —
    /// a completely normal flow, not a collision. There is currently no
    /// invalid-command shape for `MergeAppState` (any `plan_idx` /
    /// `StateEntry` payload is well-formed), so every path below returns
    /// `true`.
    fn merge_app_state(
        &mut self,
        plan_idx: usize,
        incoming: std::collections::BTreeMap<String, jian_ops_schema::state::StateEntry>,
    ) -> bool {
        if incoming.is_empty() {
            // Nothing to merge is a no-op, not a failure — see the
            // return-contract note above. Kept as an early return
            // (rather than falling into the loop) purely to skip the
            // `get_or_insert_with` allocation on doc.state when there is
            // nothing to write into it.
            return true;
        }
        let root = self
            .doc
            .state
            .get_or_insert_with(std::collections::BTreeMap::new);
        for (key, entry) in incoming {
            match self.app_state_owner.entry(key.clone()) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    // Pre-existing doc-root key: owned by the file, skip.
                    // Not a failure — the file's value is authoritative
                    // and is left untouched.
                    if root.contains_key(&key) {
                        continue;
                    }
                    root.insert(key, entry);
                    slot.insert(plan_idx);
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    // Generation-added key: lower plan_idx wins. Losing
                    // the race is not a failure — the earlier subtask's
                    // value already won and stays in place.
                    if plan_idx < *slot.get() {
                        tracing::warn!(
                            target: "op.skills",
                            key = %key,
                            winning_plan_idx = plan_idx,
                            losing_plan_idx = *slot.get(),
                            "MergeAppState key conflict — lower plan_idx wins"
                        );
                        root.insert(key, entry);
                        slot.insert(plan_idx);
                    }
                }
            }
        }
        true
    }
}
