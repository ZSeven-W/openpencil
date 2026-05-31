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
//! apply-time validation failure.
//!
use crate::align::AlignAction;
use crate::command::{EditorCommand, VariableScalarPayload};
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::tool::Tool;
use crate::viewport::Viewport;
use crate::walkers::find_node;
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

/// Resolve a `tool` string into a [`Tool`].
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

impl EditorState {
    /// Apply one [`EditorCommand`]. Returns `true` when the command
    /// actually changed the document / editor state, `false` on an
    /// apply-time validation failure.
    pub fn apply(&mut self, cmd: EditorCommand) -> bool {
        match cmd {
            // --- Raw node CRUD -------------------------------------
            EditorCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
            } => self.cmd_insert_node(&kind, &name, x, y, width, height, &fill_hex),
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
            } => self.cmd_update_node(&node_id, x, y, width, height, &name, &fill_hex),
            EditorCommand::DeleteNode { node_id } => self.cmd_delete_node(&node_id),
            EditorCommand::MoveNode {
                node_id,
                target_parent,
            } => self.cmd_move_node(&node_id, &target_parent),
            EditorCommand::CopyNode {
                node_id,
                target_parent,
            } => self.cmd_copy_node(&node_id, &target_parent),
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
            } => self.cmd_replace_node(
                &node_id,
                &kind,
                &name,
                x,
                y,
                width,
                height,
                &fill_hex,
                drop_children,
            ),
            EditorCommand::BatchInsert { items } => self.cmd_batch_insert(&items),
            EditorCommand::InsertSubtree { nodes, parent_id } => {
                let snap = self.snapshot_for_history();
                if self.cmd_insert_subtree(nodes, &parent_id) {
                    self.history_push_past(snap);
                    true
                } else {
                    false
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
            EditorCommand::SetThemes { themes, replace } => self.set_themes_bulk(themes, replace),
            EditorCommand::MergeThemePreset { variables, themes } => {
                self.set_variables_bulk(variables, false) && self.set_themes_bulk(themes, false)
            }
            EditorCommand::SetDesignMd { spec } => {
                self.doc.design_md = Some(*spec);
                true
            }
            EditorCommand::SetActiveAxisValue { axis, value } => {
                self.set_active_axis_value(&axis, &value)
            }
            EditorCommand::CycleActiveAxisValue { axis } => self.cycle_active_axis_value(&axis),

            // --- Pages ---------------------------------------------
            EditorCommand::SetActivePage { index } => self.set_active_page(index as usize),
            EditorCommand::AddPage { name } => self.add_page_with_name(name).is_some(),
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
                self.translate_selected(dx as f64, dy as f64);
                self.history_push_past(snap);
                true
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
            EditorCommand::ImportSvg { svg, x, y } => {
                let Some(mut next_id) = self.next_node_id_seed() else {
                    return false;
                };
                // `import_svg` pushes its own history snapshot when it
                // inserts ≥ 1 node.
                self.import_svg(&mut next_id, &svg, (x as f64, y as f64)) > 0
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
            } => self
                .instantiate_kit_component(
                    &kit_id,
                    &component_id,
                    doc_x.unwrap_or(0.0),
                    doc_y.unwrap_or(0.0),
                )
                .is_some(),

            // --- Layout / text property writer ----------------------
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value,
            } => self.cmd_set_node_layout_prop(&node_id, &property, &value),
        }
    }
}
