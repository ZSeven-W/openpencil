//! Side-by-side design directions ("variants") of one Studio run.
//!
//! A variants run asks the orchestrator for N complete designs of the same
//! brief, each pinned to a different style guide, and lands them next to
//! each other on the active page. The workspace records which top-level
//! boards belong to which direction so the user can keep one of them
//! ("use this"): the chosen direction stays where it is and becomes the
//! working design, and the others move to a page of their own — nothing
//! the run produced is deleted.

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableDefinition;

use super::WorkspaceState;
use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::EditorState;

/// Directions a Studio variants run asks for by default.
pub const DEFAULT_VARIANT_COUNT: u8 = 3;
/// The most directions one run may ask for. Every direction is a full
/// orchestrator run, so this is a cost ceiling as much as a layout one.
pub const MAX_VARIANT_COUNT: u8 = 4;

/// Clamp a requested direction count into `[2, MAX_VARIANT_COUNT]` —
/// one direction is an ordinary run, not a comparison.
pub fn clamp_variant_count(count: u8) -> u8 {
    count.clamp(2, MAX_VARIANT_COUNT)
}

/// The letter a direction is known by (`0` → `A`).
pub fn variant_letter(index: usize) -> char {
    char::from(b'A' + (index % 26) as u8)
}

/// One landed direction: the boards it put on the active page and the
/// palette it was generated against.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceVariant {
    /// Slot of this direction in the run (`0` = A).
    pub index: usize,
    /// Display name, e.g. `方案 A`.
    pub name: String,
    /// Catalogue id of the style guide the direction was pinned to.
    pub style_guide: String,
    /// The style guide's human name, shown beside the direction name.
    pub style_label: String,
    /// Prefix stamped on each of the direction's board names so the
    /// canvas frame labels say which direction a board belongs to.
    pub name_prefix: String,
    /// Live ids of the direction's top-level boards, in document order.
    pub root_ids: Vec<String>,
    /// The variables table the direction was generated against. The
    /// boards themselves carry resolved values (every direction shares
    /// one document), so this only matters once the direction is kept:
    /// it becomes the document's palette again.
    pub variables: Option<BTreeMap<String, VariableDefinition>>,
    /// The theme axes that belong with [`Self::variables`].
    pub themes: Option<BTreeMap<String, Vec<String>>>,
}

impl WorkspaceVariant {
    /// `方案 A · Editorial Dark` — the direction's full label.
    pub fn label(&self) -> String {
        if self.style_label.is_empty() {
            self.name.clone()
        } else {
            format!("{} · {}", self.name, self.style_label)
        }
    }
}

impl WorkspaceState {
    /// Whether this workspace's run asked for side-by-side directions.
    pub fn is_variants_run(&self) -> bool {
        self.variant_count > 0
    }

    /// Mark the run as a variants run of `count` directions (clamped).
    /// Directions land later, one by one, through [`Self::record_variant`].
    pub fn begin_variants(&mut self, count: u8) {
        self.variant_count = clamp_variant_count(count);
        self.variants.clear();
        // Directions sit side by side: only the all-boards camera shows
        // them together, whatever the family's usual view is.
        self.view = super::WorkspaceView::AllBoards;
    }

    /// Record (or replace) a landed direction, keeping slot order.
    pub fn record_variant(&mut self, variant: WorkspaceVariant) {
        match self
            .variants
            .iter_mut()
            .find(|existing| existing.index == variant.index)
        {
            Some(existing) => *existing = variant,
            None => {
                self.variants.push(variant);
                self.variants.sort_by_key(|v| v.index);
            }
        }
    }

    /// Forget the run's directions (a new run, a new document, or a pick).
    pub fn clear_variants(&mut self) {
        self.variants.clear();
        self.variant_count = 0;
    }

    /// Whether the per-direction "use this" actions are live: the run has
    /// settled (a still-running run is still moving boards around) and at
    /// least one direction landed.
    pub fn variant_pick_enabled(&self) -> bool {
        self.active && !self.variants.is_empty() && self.phase != super::WorkspacePhase::Generating
    }

    /// Record a direction a variants run reports as landed. Returns whether
    /// the workspace changed (nothing is recorded without an open
    /// workspace). Every host folds the run's landing reports through this,
    /// whichever transport carried them.
    pub fn adopt_landed_variant(&mut self, variant: WorkspaceVariant) -> bool {
        if !self.active {
            return false;
        }
        if !self.is_variants_run() {
            // A variants run launched from outside the workspace flow
            // still records its directions under the count it reports.
            self.variant_count = DEFAULT_VARIANT_COUNT;
        }
        self.record_variant(variant);
        true
    }
}

/// Pin a Studio Home brief's launch route. Home briefs are whole-design
/// requests (the orchestrator pipeline); with the directions toggle on
/// and offered, the brief becomes [`DEFAULT_VARIANT_COUNT`] side-by-side
/// directions and the workspace is told to expect them.
pub fn pin_home_brief_route(state: &mut EditorState) {
    state.chat.launch_route = crate::LaunchRoute::Orchestrator;
    let home = &state.editor_ui.home;
    if home.variants_on && !home.variants_unavailable {
        let count = DEFAULT_VARIANT_COUNT;
        state.editor_ui.workspace.begin_variants(count);
        state.chat.launch_route = crate::LaunchRoute::Variants(count);
    }
}

/// Pin a workspace retry's launch route: the orchestrator again, and a
/// side-by-side run retries as one — the same number of directions.
pub fn pin_workspace_retry_route(state: &mut EditorState) {
    state.chat.launch_route = crate::LaunchRoute::Orchestrator;
    let workspace = &mut state.editor_ui.workspace;
    if workspace.is_variants_run() {
        let count = workspace.variant_count;
        workspace.begin_variants(count);
        state.chat.launch_route = crate::LaunchRoute::Variants(count);
    }
}

/// The host-free half of a direction's "use this" press: keep direction
/// `index` (see [`pick_workspace_variant`]) and reset the workspace camera
/// bookkeeping so the host refits the direction that stayed. `None` when
/// the pick is not offered right now; otherwise whether the document
/// changed.
pub fn use_workspace_variant(state: &mut EditorState, index: usize) -> Option<bool> {
    if !state.editor_ui.workspace.variant_pick_enabled() {
        return None;
    }
    let other_page = op_i18n::translate(state.editor_ui.locale, "workspace.variants.otherPage");
    let changed = pick_workspace_variant(state, index, other_page);
    let workspace = &mut state.editor_ui.workspace;
    workspace.view = super::WorkspaceView::default_for(workspace.family);
    workspace.selected = 0;
    workspace.fitted_board_count = 0;
    workspace.fitted_bounds = None;
    Some(changed)
}

/// Keep direction `index` as the working design.
///
/// The least destructive pick: the chosen direction's boards stay where
/// they are (their direction prefix is dropped from their names), the
/// other directions' boards move — not delete — to a new page named
/// `other_page_name`, and the chosen direction's palette becomes the
/// document's variables again. One undo step reverts all of it.
///
/// Returns whether the document changed. The workspace forgets its
/// directions either way once the chosen one is found, so a pick is not
/// offered twice.
pub fn pick_workspace_variant(
    state: &mut EditorState,
    index: usize,
    other_page_name: &str,
) -> bool {
    let variants = state.editor_ui.workspace.variants.clone();
    let Some(chosen) = variants.iter().find(|v| v.index == index) else {
        return false;
    };
    let commands = variant_pick_commands(state, chosen, &variants, other_page_name);
    let changed = !commands.is_empty() && state.apply(EditorCommand::Batch { commands });
    state.editor_ui.workspace.clear_variants();
    changed
}

/// The commands a pick of `chosen` runs, in order. Every command in the
/// list changes something — `EditorCommand::Batch` rolls the whole pick
/// back when any sub-command reports no change.
pub fn variant_pick_commands(
    state: &EditorState,
    chosen: &WorkspaceVariant,
    variants: &[WorkspaceVariant],
    other_page_name: &str,
) -> Vec<EditorCommand> {
    let roots = state.active_children();
    let find = |id: &str| roots.iter().find(|node| node.id_str() == id);
    let mut commands = Vec::new();

    let mut moved = Vec::new();
    let mut moved_ids = Vec::new();
    for variant in variants.iter().filter(|v| v.index != chosen.index) {
        for id in &variant.root_ids {
            if let Some(node) = find(id) {
                moved.push(node.clone());
                moved_ids.push(id.clone());
            }
        }
    }
    if !moved.is_empty() {
        let original_page = state.ui.active_page_index as u32;
        // `AddPage` appends the page and switches to it; the original
        // index stays valid, so switching back is exact.
        commands.push(EditorCommand::AddPage {
            name: Some(other_page_name.to_string()),
            children: Some(moved),
        });
        commands.push(EditorCommand::SetActivePage {
            index: original_page,
        });
        for id in moved_ids {
            commands.push(EditorCommand::DeleteNode {
                node_id: NodeId::new(id),
                page_id: None,
            });
        }
    }

    if !chosen.name_prefix.is_empty() {
        for id in &chosen.root_ids {
            let Some(node) = find(id) else {
                continue;
            };
            let Some(name) = node.base().name.as_deref() else {
                continue;
            };
            let Some(stripped) = name.strip_prefix(chosen.name_prefix.as_str()) else {
                continue;
            };
            let stripped = stripped.trim();
            if !stripped.is_empty() && stripped != name {
                commands.push(EditorCommand::SetNodeName {
                    node_id: NodeId::new(id.clone()),
                    name: stripped.to_string(),
                });
            }
        }
    }

    if let Some(variables) = &chosen.variables {
        if state.doc.variables.as_ref() != Some(variables) {
            commands.push(EditorCommand::SetVariables {
                variables: variables.clone(),
                replace: true,
            });
        }
    }
    if let Some(themes) = &chosen.themes {
        if state.doc.themes.as_ref() != Some(themes) {
            commands.push(EditorCommand::SetThemes {
                themes: themes.clone(),
                replace: true,
            });
        }
    }
    commands
}

#[cfg(test)]
#[path = "workspace_variants_tests.rs"]
mod tests;
