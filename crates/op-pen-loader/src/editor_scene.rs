//! Editor-state scene builders.
//!
//! The full builder remains available for export, MCP, and preview consumers.
//! Interactive hosts use the active-page builder so inactive pages do not pay
//! ref/token expansion, layout, payload, or scene-allocation costs.

use jian_scene::layout_scene::{LayoutScene, ScenePage};
use op_editor_core::scene_vars::VariableTable;

use crate::editor_state_var_table;
use crate::payload::DocPayload;

/// Build a paint-only [`LayoutScene`] from every page in an editor state.
pub fn editor_state_to_layout_scene(state: &op_editor_core::EditorState) -> LayoutScene {
    full_scene(state, state.editor_ui.workspace.visible)
}
pub fn editor_state_to_export_layout_scene(state: &op_editor_core::EditorState) -> LayoutScene {
    full_scene(state, true)
}
fn full_scene(state: &op_editor_core::EditorState, paged: bool) -> LayoutScene {
    let mut prepared = std::borrow::Cow::Borrowed(&state.doc);
    if op_editor_core::ref_resolve::document_has_refs(&prepared) {
        prepared = std::borrow::Cow::Owned(op_editor_core::ref_resolve::resolve_refs_for_canvas(
            &prepared,
        ));
    }
    if op_editor_core::variables_resolve::document_has_tokens(&prepared) {
        prepared = std::borrow::Cow::Owned(
            op_editor_core::variables_resolve::resolve_document_for_canvas(
                &prepared,
                &state.ui.variables.active_theme,
            ),
        );
    }
    let root_snapshot = crate::table_default_snapshot::roots(&prepared.children, &state.doc, paged);
    let page_snapshots: Vec<_> = prepared
        .pages
        .as_ref()
        .into_iter()
        .flatten()
        .map(|page| crate::table_default_snapshot::roots(&page.children, &state.doc, paged))
        .collect();
    if root_snapshot.is_some() || page_snapshots.iter().any(Option::is_some) {
        let mut snapshot = prepared.into_owned();
        if let Some(nodes) = root_snapshot {
            snapshot.children = nodes;
        }
        if let Some(pages) = &mut snapshot.pages {
            for (page, nodes) in pages.iter_mut().zip(page_snapshots) {
                if let Some(nodes) = nodes {
                    page.children = nodes;
                }
            }
        }
        prepared = std::borrow::Cow::Owned(snapshot);
    }
    let payload: DocPayload = if state.editor_ui.preserve_authored_geometry {
        crate::adapter::pen_document_to_payload_preserving_geometry(&prepared).payload
    } else {
        crate::adapter::pen_document_to_payload(&prepared).payload
    };
    let var_table: VariableTable = editor_state_var_table(state);

    LayoutScene {
        pages: payload
            .pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: page
                    .children
                    .iter()
                    .map(|node| crate::layout_scene::node_payload_to_scene(node, &var_table, 1.0))
                    .collect(),
            })
            .collect(),
        active_page_index: state
            .ui
            .active_page_index
            .min(payload.pages.len().saturating_sub(1)),
    }
}

/// Build the interactive editor scene for only the active page.
///
/// Page metadata stays in document order and `active_page_index` keeps its
/// canonical index, but inactive [`ScenePage`] entries have no render children.
pub fn editor_state_to_active_page_layout_scene(
    state: &op_editor_core::EditorState,
) -> LayoutScene {
    active_scene(state, state.editor_ui.workspace.visible)
}
pub fn editor_state_to_active_page_export_layout_scene(
    state: &op_editor_core::EditorState,
) -> LayoutScene {
    let scene = active_scene(state, true);
    if state.selection_count() == 1
        && state.selection.anchor.is_real()
        && scene
            .active_page()
            .is_some_and(|page| page.find(state.selection.anchor.as_str()).is_none())
    {
        // A specifically selected record remains exportable outside page one.
        active_scene(state, false)
    } else {
        scene
    }
}
fn active_scene(state: &op_editor_core::EditorState, paged: bool) -> LayoutScene {
    let (mut pages, active_page_index, roots): (
        Vec<ScenePage>,
        usize,
        &[jian_ops_schema::node::PenNode],
    ) = if let Some(doc_pages) = state.doc.pages.as_ref() {
        if doc_pages.is_empty() {
            return LayoutScene::default();
        }
        let active_page_index = state
            .ui
            .active_page_index
            .min(doc_pages.len().saturating_sub(1));
        let pages = doc_pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: Vec::new(),
            })
            .collect();
        (
            pages,
            active_page_index,
            &doc_pages[active_page_index].children,
        )
    } else {
        let (id, name) = if state.doc.children.is_empty() {
            ("n1".to_string(), "Page 1".to_string())
        } else {
            (
                "page-1".to_string(),
                state.doc.name.as_deref().unwrap_or("Page 1").to_string(),
            )
        };
        (
            vec![ScenePage {
                id,
                name,
                children: Vec::new(),
            }],
            0,
            state.doc.children.as_slice(),
        )
    };

    let mut prepared = std::borrow::Cow::Borrowed(roots);
    if op_editor_core::ref_resolve::roots_have_refs(prepared.as_ref()) {
        prepared = std::borrow::Cow::Owned(if state.document_revision() == 0 {
            // A freshly loaded/imported state's registry was built from this
            // exact document. Reusing it avoids indexing every inactive page
            // on each switch. Once any canonical edit has happened the
            // registry may be a stale prototype snapshot, so preserve the old
            // live-document semantics by taking the full lookup path instead.
            op_editor_core::ref_resolve::resolve_refs_for_canvas_roots_with_components(
                prepared.as_ref(),
                &state.components,
                &state.doc,
            )
        } else {
            op_editor_core::ref_resolve::resolve_refs_for_canvas_roots(
                prepared.as_ref(),
                &state.doc,
            )
        });
    }
    if op_editor_core::variables_resolve::roots_have_tokens(prepared.as_ref()) {
        let mut owned = prepared.into_owned();
        op_editor_core::variables_resolve::resolve_roots_for_canvas(
            &mut owned,
            &state.doc,
            &state.ui.variables.active_theme,
        );
        prepared = std::borrow::Cow::Owned(owned);
    }

    if let Some(nodes) = crate::table_default_snapshot::roots(prepared.as_ref(), &state.doc, paged)
    {
        prepared = std::borrow::Cow::Owned(nodes);
    }
    let active_meta = &pages[active_page_index];
    let payload = crate::adapter::pen_roots_to_page_payload(
        &active_meta.id,
        &active_meta.name,
        prepared.as_ref(),
        active_page_index,
        state.editor_ui.preserve_authored_geometry,
    );
    let var_table = editor_state_var_table(state);
    pages[active_page_index].children = payload
        .children
        .iter()
        .map(|node| crate::layout_scene::node_payload_to_scene(node, &var_table, 1.0))
        .collect();

    LayoutScene {
        pages,
        active_page_index,
    }
}
