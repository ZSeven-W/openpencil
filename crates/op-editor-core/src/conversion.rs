use std::collections::{BTreeMap, HashSet};

use jian_ops_schema::conversion::{ConversionEntry, ConversionKind, ConversionSpec};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

use crate::components::Component;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;

pub fn upsert_conversion_entry(doc: &mut PenDocument, entry: ConversionEntry) {
    let spec = doc.conversion.get_or_insert_with(ConversionSpec::default);
    match spec
        .entries
        .iter_mut()
        .find(|existing| existing.kind == entry.kind && existing.key == entry.key)
    {
        Some(existing) => *existing = entry,
        None => spec.entries.push(entry),
    }
}

pub fn find_conversion_entry<'a>(
    doc: &'a PenDocument,
    kind: ConversionKind,
    key: &str,
) -> Option<&'a ConversionEntry> {
    doc.conversion
        .as_ref()?
        .entries
        .iter()
        .find(|entry| entry.kind == kind && entry.key == key)
}

pub fn node_exists(doc: &PenDocument, node_id: &str) -> bool {
    let id = NodeId::new(node_id);
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if walkers::find_node(&page.children, &id).is_some() {
                return true;
            }
        }
    }
    walkers::find_node(&doc.children, &id).is_some()
}

pub fn upsert_component(
    state: &mut EditorState,
    key: String,
    name: String,
    root: PenNode,
    source_path: Option<String>,
    source_hash: Option<String>,
) -> bool {
    let key = key.trim().to_string();
    let name = name.trim().to_string();
    if key.is_empty() || name.is_empty() || !is_component_root(&root) {
        return false;
    }
    let existing_entry =
        find_conversion_entry(&state.doc, ConversionKind::Component, &key).cloned();
    let existing_root = existing_entry
        .as_ref()
        .and_then(|entry| entry.node_id.as_deref())
        .and_then(|node_id| find_node_in_doc(&state.doc, node_id))
        .cloned();

    let Some((master_id, root, node_ids)) = prepare_component_root(
        state,
        root,
        &name,
        existing_root.as_ref(),
        existing_entry.as_ref(),
    ) else {
        return false;
    };

    if !replace_or_insert_component_master(state, &master_id, root.clone()) {
        return false;
    }
    state.components.insert(Component {
        id: NodeId::new(master_id.clone()),
        name,
        root,
    });
    upsert_conversion_entry(
        &mut state.doc,
        ConversionEntry {
            kind: ConversionKind::Component,
            key,
            source_path,
            source_hash,
            node_id: Some(master_id),
            node_ids: Some(node_ids),
        },
    );
    true
}

pub fn upsert_screen(
    state: &mut EditorState,
    key: String,
    root: PenNode,
    source_path: Option<String>,
    source_hash: Option<String>,
) -> bool {
    let key = key.trim().to_string();
    if key.is_empty() || !matches!(root, PenNode::Frame(_)) {
        return false;
    }
    let existing_entry = find_conversion_entry(&state.doc, ConversionKind::Screen, &key).cloned();
    let existing_root = existing_entry
        .as_ref()
        .and_then(|entry| entry.node_id.as_deref())
        .and_then(|node_id| find_node_in_doc(&state.doc, node_id))
        .cloned();

    let Some((screen_id, root, node_ids)) =
        prepare_screen_root(state, root, existing_root.as_ref(), existing_entry.as_ref())
    else {
        return false;
    };

    if !replace_or_insert_screen(state, &screen_id, root) {
        return false;
    }
    upsert_conversion_entry(
        &mut state.doc,
        ConversionEntry {
            kind: ConversionKind::Screen,
            key,
            source_path,
            source_hash,
            node_id: Some(screen_id),
            node_ids: Some(node_ids),
        },
    );
    true
}

fn prepare_component_root(
    state: &EditorState,
    mut root: PenNode,
    name: &str,
    existing_root: Option<&PenNode>,
    existing_entry: Option<&ConversionEntry>,
) -> Option<(String, PenNode, BTreeMap<String, String>)> {
    let mut next_id = state.next_node_id_seed()?;
    let mut taken = state.collect_node_ids();
    if let Some(existing) = existing_root {
        remove_subtree_ids_from_taken(existing, &mut taken);
    }
    let existing_ids = existing_node_ids_for_rerun(&root, existing_entry);
    let node_ids =
        remap_subtree_ids_by_source_id(&mut root, &mut next_id, &mut taken, existing_ids.as_ref())?;
    root.base_mut().name = Some(name.to_string());
    set_reusable(&mut root, true);
    Some((root.id_str().to_string(), root, node_ids))
}

fn replace_or_insert_component_master(
    state: &mut EditorState,
    master_id: &str,
    root: PenNode,
) -> bool {
    let id = NodeId::new(master_id);
    if replace_node_in_doc(&mut state.doc, &id, root.clone()) {
        return true;
    }
    state.append_components_page_masters(vec![root]) == 1
}

fn prepare_screen_root(
    state: &EditorState,
    mut root: PenNode,
    existing_root: Option<&PenNode>,
    existing_entry: Option<&ConversionEntry>,
) -> Option<(String, PenNode, BTreeMap<String, String>)> {
    let mut next_id = state.next_node_id_seed()?;
    let mut taken = state.collect_node_ids();
    if let Some(existing) = existing_root {
        remove_subtree_ids_from_taken(existing, &mut taken);
    }
    let existing_ids = existing_node_ids_for_rerun(&root, existing_entry);
    let node_ids =
        remap_subtree_ids_by_source_id(&mut root, &mut next_id, &mut taken, existing_ids.as_ref())?;
    set_reusable(&mut root, false);
    Some((root.id_str().to_string(), root, node_ids))
}

fn existing_node_ids_for_rerun(
    incoming_root: &PenNode,
    existing_entry: Option<&ConversionEntry>,
) -> Option<BTreeMap<String, String>> {
    let entry = existing_entry?;
    let mut ids = entry.node_ids.clone().unwrap_or_default();
    if let Some(node_id) = &entry.node_id {
        ids.entry(incoming_root.id_str().to_string())
            .or_insert_with(|| node_id.clone());
    }
    (!ids.is_empty()).then_some(ids)
}

fn remap_subtree_ids_by_source_id(
    node: &mut PenNode,
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
    existing_ids: Option<&BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    remap_node_by_source_id(node, existing_ids, next_id, taken, &mut out).then_some(out)
}

fn remap_node_by_source_id(
    node: &mut PenNode,
    existing_ids: Option<&BTreeMap<String, String>>,
    next_id: &mut u64,
    taken: &mut HashSet<NodeId>,
    out: &mut BTreeMap<String, String>,
) -> bool {
    let source_id = node.id_str().to_string();
    if source_id.trim().is_empty() || out.contains_key(&source_id) {
        return false;
    }
    let (target_id, already_reserved) = match existing_ids.and_then(|ids| ids.get(&source_id)) {
        Some(existing_id) if !existing_id.trim().is_empty() => (existing_id.clone(), false),
        _ => {
            let Some(fresh) = walkers::alloc_n_id(next_id, taken) else {
                return false;
            };
            (fresh.as_str().to_string(), true)
        }
    };
    if !already_reserved && !taken.insert(NodeId::new(target_id.clone())) {
        return false;
    }
    node.base_mut().id = target_id.clone();
    out.insert(source_id, target_id);
    if let Some(children) = node.children_mut() {
        for child in children {
            if !remap_node_by_source_id(child, existing_ids, next_id, taken, out) {
                return false;
            }
        }
    }
    true
}

fn remove_subtree_ids_from_taken(node: &PenNode, taken: &mut HashSet<NodeId>) {
    if let Some(id) = NodeId::new_opt(node.id_str()) {
        taken.remove(&id);
    }
    if let Some(children) = node.children() {
        for child in children {
            remove_subtree_ids_from_taken(child, taken);
        }
    }
}

fn replace_or_insert_screen(state: &mut EditorState, screen_id: &str, root: PenNode) -> bool {
    let id = NodeId::new(screen_id);
    if replace_node_in_doc(&mut state.doc, &id, root.clone()) {
        return true;
    }
    state.active_children_mut().push(root);
    true
}

fn replace_node_in_doc(doc: &mut PenDocument, id: &NodeId, replacement: PenNode) -> bool {
    if let Some(pages) = doc.pages.as_mut() {
        for page in pages {
            if let Some(live) = walkers::find_node_mut(&mut page.children, id) {
                *live = replacement;
                return true;
            }
        }
    }
    if let Some(live) = walkers::find_node_mut(&mut doc.children, id) {
        *live = replacement;
        return true;
    }
    false
}

fn find_node_in_doc<'a>(doc: &'a PenDocument, node_id: &str) -> Option<&'a PenNode> {
    let id = NodeId::new(node_id);
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(node) = walkers::find_node(&page.children, &id) {
                return Some(node);
            }
        }
    }
    walkers::find_node(&doc.children, &id)
}

fn is_component_root(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_)
    )
}

fn set_reusable(node: &mut PenNode, reusable: bool) {
    if let PenNode::Frame(frame) = node {
        frame.reusable = reusable.then_some(true);
    }
}
