//! Atomic installation of a pre-validated collaboration document.
//!
//! The collaboration protocol owns wire validation, canonical hashes, and
//! operation replay. This neutral editor-core seam validates document identity
//! invariants, prebuilds editor-derived state, and swaps the document without
//! depending on transport or collaboration types.

use crate::components::ComponentLibrary;
use crate::edit_transaction::EditOrigin;
use crate::fills::{first_solid_fill_hex, first_solid_stroke_hex};
use crate::history::History;
use crate::pen_node_ext::PenNodeExt;
use crate::selection::SelectionState;
use crate::state::EditorState;
use crate::ui_draft::{UiDraftState, VariableUiState};
use crate::variables_resolve::default_theme;
use crate::NodeId;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Summary of a successful document installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentInstallReport {
    /// Source that selected the install semantics.
    pub origin: EditOrigin,
    /// Number of previously selected node ids retained on the active page.
    pub retained_selection: usize,
    /// Whether the active logical page changed.
    pub active_page_changed: bool,
}

/// Structural document failure detected before installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentInstallError {
    /// An explicit page has no stable id.
    EmptyPageId { page_index: usize },
    /// A node has no stable id.
    EmptyNodeId,
    /// A page or node id occurs more than once in the document.
    DuplicateId { id: String },
    /// Explicit pages and legacy root children cannot both carry content.
    ExplicitPagesWithRootChildren,
}

impl std::fmt::Display for DocumentInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPageId { page_index } => {
                write!(f, "page at index {page_index} has an empty id")
            }
            Self::EmptyNodeId => f.write_str("document contains a node with an empty id"),
            Self::DuplicateId { id } => write!(f, "document id is not globally unique: {id}"),
            Self::ExplicitPagesWithRootChildren => {
                f.write_str("a document with explicit pages must have empty root children")
            }
        }
    }
}

impl std::error::Error for DocumentInstallError {}

struct PreparedInstall {
    components: ComponentLibrary,
    active_page_index: usize,
    active_page_changed: bool,
    selection: SelectionState,
    variables: VariableUiState,
}

impl EditorState {
    /// Atomically install a collaboration document after neutral validation.
    ///
    /// Viewport, tool, editor settings, chat sessions, and clipboard remain
    /// untouched. Document-derived drafts and caches are rebuilt or cleared.
    /// `RemoteCommit` and `Replay` preserve the current generation and legacy
    /// history without adding an undo entry. `Snapshot` starts a new document
    /// generation and clears stale undo history.
    pub fn install_verified_document(
        &mut self,
        doc: PenDocument,
        origin: EditOrigin,
    ) -> Result<DocumentInstallReport, DocumentInstallError> {
        validate_install_document(&doc)?;
        let prepared = self.prepare_document_install(&doc);
        let local_snapshot = (origin == EditOrigin::Local).then(|| self.snapshot_for_history());

        let old_doc = std::mem::replace(&mut self.doc, doc);
        self.components = prepared.components;
        self.selection = prepared.selection;
        self.clear_document_drafts(prepared.variables.active_theme);
        self.ui.active_page_index = prepared.active_page_index;
        self.ui.variables.fill_refs = prepared.variables.fill_refs;
        self.ui.variables.stroke_refs = prepared.variables.stroke_refs;
        self.editor_ui.clear_document_derived();
        self.codegen.reset_for_document_replacement();
        self.app_state_owner.clear();
        self.instance_write_virtual_anchor = None;

        match origin {
            EditOrigin::Local => {
                self.history_push_past(local_snapshot.expect("local snapshot was prepared"));
            }
            EditOrigin::RemoteCommit | EditOrigin::Replay => {
                self.mark_document_changed();
            }
            EditOrigin::Snapshot => {
                self.history = History::new();
                self.history_push_count = 0;
                self.document_generation = self.document_generation.saturating_add(1);
                self.mark_document_changed();
            }
        }

        jian_ops_schema::image_thumbs::activate_for_document(&self.doc);
        crate::state::drop_document_after_replace(old_doc);
        Ok(DocumentInstallReport {
            origin,
            retained_selection: self.selection.len(),
            active_page_changed: prepared.active_page_changed,
        })
    }

    fn prepare_document_install(&self, doc: &PenDocument) -> PreparedInstall {
        let old_page = active_page_identity(&self.doc, self.ui.active_page_index);
        let active_page_index =
            choose_active_page(doc, self.ui.active_page_index, old_page.as_deref());
        let new_page = active_page_identity(doc, active_page_index);
        let active_page_changed = old_page != new_page;
        let live_selection_ids = collect_forest_ids(active_children_for(doc, active_page_index));
        let set: Vec<_> = self
            .selection
            .set
            .iter()
            .filter(|id| live_selection_ids.contains(*id))
            .cloned()
            .collect();
        let selection = SelectionState {
            anchor: set.last().cloned().unwrap_or(NodeId::NONE),
            set,
        };
        let (fill_refs, stroke_refs) = collect_color_refs(doc);
        let active_theme = reconcile_active_theme(doc, &self.ui.variables.active_theme);

        PreparedInstall {
            components: ComponentLibrary::from_document(doc),
            active_page_index,
            active_page_changed,
            selection,
            variables: VariableUiState {
                active_theme,
                fill_refs,
                stroke_refs,
            },
        }
    }

    pub(crate) fn clear_document_drafts(&mut self, active_theme: BTreeMap<String, String>) {
        let active_page_index = self.ui.active_page_index;
        let fill_refs = std::mem::take(&mut self.ui.variables.fill_refs);
        let stroke_refs = std::mem::take(&mut self.ui.variables.stroke_refs);
        self.ui = UiDraftState::new();
        self.ui.active_page_index = active_page_index;
        self.ui.variables = VariableUiState {
            active_theme,
            fill_refs,
            stroke_refs,
        };
    }
}

fn validate_install_document(doc: &PenDocument) -> Result<(), DocumentInstallError> {
    if doc
        .pages
        .as_ref()
        .is_some_and(|pages| !pages.is_empty() && !doc.children.is_empty())
    {
        return Err(DocumentInstallError::ExplicitPagesWithRootChildren);
    }

    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    if let Some(pages) = doc.pages.as_ref() {
        for (page_index, page) in pages.iter().enumerate() {
            if page.id.is_empty() {
                return Err(DocumentInstallError::EmptyPageId { page_index });
            }
            if !seen.insert(page.id.clone()) {
                return Err(DocumentInstallError::DuplicateId {
                    id: page.id.clone(),
                });
            }
            stack.extend(page.children.iter());
        }
    }
    stack.extend(doc.children.iter());
    while let Some(node) = stack.pop() {
        let id = node.id_str();
        if id.is_empty() {
            return Err(DocumentInstallError::EmptyNodeId);
        }
        if !seen.insert(id.to_string()) {
            return Err(DocumentInstallError::DuplicateId { id: id.to_string() });
        }
        if let Some(children) = node.children() {
            stack.extend(children.iter());
        }
    }
    Ok(())
}

fn active_page_identity(doc: &PenDocument, index: usize) -> Option<String> {
    match doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages
            .get(index.min(pages.len() - 1))
            .map(|page| page.id.clone()),
        _ => None,
    }
}

fn choose_active_page(doc: &PenDocument, old_index: usize, old_id: Option<&str>) -> usize {
    let Some(pages) = doc.pages.as_ref().filter(|pages| !pages.is_empty()) else {
        return 0;
    };
    old_id
        .and_then(|id| pages.iter().position(|page| page.id == id))
        .unwrap_or_else(|| old_index.min(pages.len() - 1))
}

fn active_children_for(doc: &PenDocument, index: usize) -> &[PenNode] {
    match doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => &pages[index.min(pages.len() - 1)].children,
        _ => &doc.children,
    }
}

fn collect_forest_ids(nodes: &[PenNode]) -> HashSet<NodeId> {
    let mut out = HashSet::new();
    let mut stack: Vec<_> = nodes.iter().collect();
    while let Some(node) = stack.pop() {
        out.insert(NodeId::new(node.id_str()));
        if let Some(children) = node.children() {
            stack.extend(children.iter());
        }
    }
    out
}

fn collect_color_refs(doc: &PenDocument) -> (HashMap<NodeId, String>, HashMap<NodeId, String>) {
    let mut fill_refs = HashMap::new();
    let mut stroke_refs = HashMap::new();
    let mut stack: Vec<_> = doc.children.iter().collect();
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            stack.extend(page.children.iter());
        }
    }
    while let Some(node) = stack.pop() {
        let id = NodeId::new(node.id_str());
        if let Some(name) = first_solid_fill_hex(node).and_then(variable_name) {
            fill_refs.insert(id.clone(), name.to_string());
        }
        if let Some(name) = first_solid_stroke_hex(node).and_then(variable_name) {
            stroke_refs.insert(id, name.to_string());
        }
        if let Some(children) = node.children() {
            stack.extend(children.iter());
        }
    }
    (fill_refs, stroke_refs)
}

fn variable_name(value: &str) -> Option<&str> {
    value.strip_prefix('$').filter(|name| !name.is_empty())
}

fn reconcile_active_theme(
    doc: &PenDocument,
    current: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut reconciled = default_theme(doc.themes.as_ref());
    let Some(themes) = doc.themes.as_ref() else {
        return reconciled;
    };
    for (axis, current_value) in current {
        if themes
            .get(axis)
            .is_some_and(|values| values.contains(current_value))
        {
            reconciled.insert(axis.clone(), current_value.clone());
        }
    }
    reconciled
}
