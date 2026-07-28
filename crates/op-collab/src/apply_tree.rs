use std::collections::HashSet;

use jian_ops_schema::{node::PenNode, PenDocument};

use crate::{apply_budget::ValidationBudget, ApplyLimits, CollabApplyError, PageRef};

#[derive(Clone, Copy)]
pub(crate) struct DocumentStats {
    pub(crate) node_count: usize,
}

pub(crate) struct SubtreeIds {
    pub(crate) ids: HashSet<String>,
    pub(crate) node_count: usize,
}

pub(crate) fn document_ids(
    document: &PenDocument,
    limits: &ApplyLimits,
    budget: &mut ValidationBudget,
) -> Result<(HashSet<String>, DocumentStats), CollabApplyError> {
    if document
        .pages
        .as_ref()
        .is_some_and(|pages| !pages.is_empty())
        && !document.children.is_empty()
    {
        return Err(CollabApplyError::MixedDocumentRoots);
    }
    let mut ids = HashSet::new();
    let mut node_count = 0;
    if let Some(pages) = &document.pages {
        for page in pages {
            budget.charge(1)?;
            insert_unique_id(&mut ids, &page.id, limits)?;
            node_count += 1;
            node_count += collect_document_node_ids(&page.children, &mut ids, limits, budget)?;
            enforce_document_node_limit(node_count, limits)?;
        }
    }
    node_count += collect_document_node_ids(&document.children, &mut ids, limits, budget)?;
    enforce_document_node_limit(node_count, limits)?;
    Ok((ids, DocumentStats { node_count }))
}

pub(crate) fn subtree_ids(
    node: &PenNode,
    limits: &ApplyLimits,
    budget: &mut ValidationBudget,
    operation: usize,
    previously_processed: usize,
) -> Result<SubtreeIds, CollabApplyError> {
    let mut ids = HashSet::new();
    let mut node_count = 0_usize;
    let mut stack = vec![(node, 1_usize)];
    while let Some((node, depth)) = stack.pop() {
        budget.charge(1)?;
        if depth > limits.max_tree_depth {
            return Err(CollabApplyError::TreeTooDeep {
                actual: depth,
                limit: limits.max_tree_depth,
            });
        }
        node_count = node_count.saturating_add(1);
        let processed = previously_processed.saturating_add(node_count);
        if processed > limits.max_processed_subtree_nodes_per_op {
            return Err(CollabApplyError::SubtreeTooLarge {
                operation,
                actual: processed,
                limit: limits.max_processed_subtree_nodes_per_op,
            });
        }
        insert_unique_id(&mut ids, node_id_of(node), limits)?;
        if let Some(children) = children(node) {
            stack.extend(children.iter().rev().map(|child| (child, depth + 1)));
        }
    }
    Ok(SubtreeIds { ids, node_count })
}

fn collect_document_node_ids(
    nodes: &[PenNode],
    ids: &mut HashSet<String>,
    limits: &ApplyLimits,
    budget: &mut ValidationBudget,
) -> Result<usize, CollabApplyError> {
    let mut count = 0_usize;
    let mut stack: Vec<(&PenNode, usize)> = nodes.iter().rev().map(|node| (node, 1)).collect();
    while let Some((node, depth)) = stack.pop() {
        budget.charge(1)?;
        if depth > limits.max_tree_depth {
            return Err(CollabApplyError::TreeTooDeep {
                actual: depth,
                limit: limits.max_tree_depth,
            });
        }
        count = count.saturating_add(1);
        if count > limits.max_document_nodes {
            return Err(CollabApplyError::TooManyDocumentNodes {
                actual: count,
                limit: limits.max_document_nodes,
            });
        }
        insert_unique_id(ids, node_id_of(node), limits)?;
        if let Some(children) = children(node) {
            stack.extend(children.iter().rev().map(|child| (child, depth + 1)));
        }
    }
    Ok(count)
}

fn insert_unique_id(
    ids: &mut HashSet<String>,
    id: &str,
    limits: &ApplyLimits,
) -> Result<(), CollabApplyError> {
    if id.is_empty() {
        return Err(CollabApplyError::EmptyId);
    }
    if id.len() > limits.max_identifier_bytes {
        return Err(CollabApplyError::IdentifierTooLong {
            actual: id.len(),
            limit: limits.max_identifier_bytes,
        });
    }
    if !ids.insert(id.to_owned()) {
        return Err(CollabApplyError::DuplicateId { id: id.to_owned() });
    }
    Ok(())
}

fn enforce_document_node_limit(
    actual: usize,
    limits: &ApplyLimits,
) -> Result<(), CollabApplyError> {
    if actual > limits.max_document_nodes {
        return Err(CollabApplyError::TooManyDocumentNodes {
            actual,
            limit: limits.max_document_nodes,
        });
    }
    Ok(())
}

pub(crate) fn page_nodes<'a>(
    document: &'a PenDocument,
    page: &PageRef,
    budget: &mut ValidationBudget,
) -> Result<&'a [PenNode], CollabApplyError> {
    match page {
        PageRef::DocumentRoot => Ok(&document.children),
        PageRef::PageId(page_id) => {
            let Some(pages) = document.pages.as_deref() else {
                return Err(CollabApplyError::PageNotFound {
                    page_id: page_id.clone(),
                });
            };
            for page in pages {
                budget.charge(1)?;
                if page.id == *page_id {
                    return Ok(&page.children);
                }
            }
            Err(CollabApplyError::PageNotFound {
                page_id: page_id.clone(),
            })
        }
    }
}

pub(crate) fn page_nodes_mut<'a>(
    document: &'a mut PenDocument,
    page: &PageRef,
    budget: &mut ValidationBudget,
) -> Result<&'a mut Vec<PenNode>, CollabApplyError> {
    match page {
        PageRef::DocumentRoot => Ok(&mut document.children),
        PageRef::PageId(page_id) => {
            let Some(pages) = document.pages.as_deref_mut() else {
                return Err(CollabApplyError::PageNotFound {
                    page_id: page_id.clone(),
                });
            };
            for page in pages {
                budget.charge(1)?;
                if page.id == *page_id {
                    return Ok(&mut page.children);
                }
            }
            Err(CollabApplyError::PageNotFound {
                page_id: page_id.clone(),
            })
        }
    }
}

pub(crate) fn siblings<'a>(
    roots: &'a [PenNode],
    parent_id: Option<&str>,
    budget: &mut ValidationBudget,
) -> Result<&'a [PenNode], CollabApplyError> {
    let Some(parent_id) = parent_id else {
        return Ok(roots);
    };
    let parent =
        find_node(roots, parent_id, budget)?.ok_or_else(|| CollabApplyError::ParentNotFound {
            parent_id: parent_id.to_owned(),
        })?;
    let slot = child_slot(parent).ok_or_else(|| CollabApplyError::ParentNotContainer {
        parent_id: parent_id.to_owned(),
    })?;
    Ok(slot.as_deref().unwrap_or_default())
}

pub(crate) fn siblings_mut<'a>(
    roots: &'a mut Vec<PenNode>,
    parent_id: Option<&str>,
    budget: &mut ValidationBudget,
) -> Result<&'a mut Vec<PenNode>, CollabApplyError> {
    let Some(parent_id) = parent_id else {
        return Ok(roots);
    };
    let parent = find_node_mut(roots, parent_id, budget)?.ok_or_else(|| {
        CollabApplyError::ParentNotFound {
            parent_id: parent_id.to_owned(),
        }
    })?;
    let slot = child_slot_mut(parent).ok_or_else(|| CollabApplyError::ParentNotContainer {
        parent_id: parent_id.to_owned(),
    })?;
    Ok(slot.get_or_insert_with(Vec::new))
}

pub(crate) fn find_node<'a>(
    nodes: &'a [PenNode],
    node_id: &str,
    budget: &mut ValidationBudget,
) -> Result<Option<&'a PenNode>, CollabApplyError> {
    for node in nodes {
        budget.charge(1)?;
        if node_id_of(node) == node_id {
            return Ok(Some(node));
        }
        if let Some(children) = children(node) {
            if let Some(found) = find_node(children, node_id, budget)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn find_node_mut<'a>(
    nodes: &'a mut [PenNode],
    node_id: &str,
    budget: &mut ValidationBudget,
) -> Result<Option<&'a mut PenNode>, CollabApplyError> {
    for node in nodes {
        budget.charge(1)?;
        if node_id_of(node) == node_id {
            return Ok(Some(node));
        }
        if let Some(children) = children_mut(node) {
            if let Some(found) = find_node_mut(children, node_id, budget)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

pub(crate) fn find_location(
    nodes: &[PenNode],
    node_id: &str,
    parent_id: Option<&str>,
    budget: &mut ValidationBudget,
) -> Result<Option<(Option<String>, usize)>, CollabApplyError> {
    for (index, node) in nodes.iter().enumerate() {
        budget.charge(1)?;
        if node_id_of(node) == node_id {
            return Ok(Some((parent_id.map(str::to_owned), index)));
        }
        if let Some(children) = children(node) {
            if let Some(found) = find_location(children, node_id, Some(node_id_of(node)), budget)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

pub(crate) fn remove_node(
    nodes: &mut Vec<PenNode>,
    node_id: &str,
    budget: &mut ValidationBudget,
) -> Result<Option<PenNode>, CollabApplyError> {
    for index in 0..nodes.len() {
        budget.charge(1)?;
        if node_id_of(&nodes[index]) == node_id {
            return Ok(Some(nodes.remove(index)));
        }
        if let Some(children) = children_mut(&mut nodes[index]) {
            if let Some(removed) = remove_node(children, node_id, budget)? {
                return Ok(Some(removed));
            }
        }
    }
    Ok(None)
}

pub(crate) fn replace_node(
    nodes: &mut [PenNode],
    node_id: &str,
    replacement: PenNode,
    budget: &mut ValidationBudget,
) -> Result<bool, CollabApplyError> {
    let mut replacement = Some(replacement);
    replace_node_inner(nodes, node_id, &mut replacement, budget)
}

fn replace_node_inner(
    nodes: &mut [PenNode],
    node_id: &str,
    replacement: &mut Option<PenNode>,
    budget: &mut ValidationBudget,
) -> Result<bool, CollabApplyError> {
    for node in nodes {
        budget.charge(1)?;
        if node_id_of(node) == node_id {
            *node = replacement
                .take()
                .expect("replacement is consumed only once");
            return Ok(true);
        }
        if let Some(children) = children_mut(node) {
            if replace_node_inner(children, node_id, replacement, budget)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub(crate) fn node_id_of(node: &PenNode) -> &str {
    match node {
        PenNode::Frame(node) => &node.base.id,
        PenNode::Group(node) => &node.base.id,
        PenNode::Rectangle(node) => &node.base.id,
        PenNode::Ellipse(node) => &node.base.id,
        PenNode::Line(node) => &node.base.id,
        PenNode::Polygon(node) => &node.base.id,
        PenNode::Path(node) => &node.base.id,
        PenNode::Text(node) => &node.base.id,
        PenNode::TextInput(node) => &node.base.id,
        PenNode::Image(node) => &node.base.id,
        PenNode::IconFont(node) => &node.base.id,
        PenNode::TextArea(node) => &node.base.id,
        PenNode::Select(node) => &node.base.id,
        PenNode::Switch(node) => &node.base.id,
        PenNode::Checkbox(node) => &node.base.id,
        PenNode::Slider(node) => &node.base.id,
        PenNode::RadioGroup(node) => &node.base.id,
        PenNode::NumberInput(node) => &node.base.id,
        PenNode::Progress(node) => &node.base.id,
        PenNode::Tabs(node) => &node.base.id,
        PenNode::Ref(node) => &node.base.id,
    }
}

pub(crate) fn children(node: &PenNode) -> Option<&[PenNode]> {
    child_slot(node).and_then(Option::as_deref)
}

fn children_mut(node: &mut PenNode) -> Option<&mut Vec<PenNode>> {
    child_slot_mut(node).and_then(Option::as_mut)
}

pub(crate) fn child_slot(node: &PenNode) -> Option<&Option<Vec<PenNode>>> {
    match node {
        PenNode::Frame(node) => Some(&node.children),
        PenNode::Group(node) => Some(&node.children),
        PenNode::Rectangle(node) => Some(&node.children),
        PenNode::Tabs(node) => Some(&node.children),
        PenNode::Ref(node) => Some(&node.children),
        _ => None,
    }
}

pub(crate) fn child_slot_mut(node: &mut PenNode) -> Option<&mut Option<Vec<PenNode>>> {
    match node {
        PenNode::Frame(node) => Some(&mut node.children),
        PenNode::Group(node) => Some(&mut node.children),
        PenNode::Rectangle(node) => Some(&mut node.children),
        PenNode::Tabs(node) => Some(&mut node.children),
        PenNode::Ref(node) => Some(&mut node.children),
        _ => None,
    }
}
