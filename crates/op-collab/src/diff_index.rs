use std::collections::BTreeMap;

use jian_ops_schema::{node::PenNode, PenDocument};

use crate::{
    apply_tree::{children, node_id_of},
    DiffError, PageRef,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ScopeKey {
    DocumentRoot,
    Page(String),
}

impl ScopeKey {
    fn for_page(page: &PageRef) -> Self {
        match page {
            PageRef::DocumentRoot => Self::DocumentRoot,
            PageRef::PageId(id) => Self::Page(id.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SiblingKey {
    scope: ScopeKey,
    parent: Option<String>,
}

#[derive(Clone)]
pub(crate) struct NodeEntry<'a> {
    pub page: PageRef,
    pub parent: Option<String>,
    pub index: u32,
    pub node: &'a PenNode,
}

pub(crate) struct TreeIndex<'a> {
    pub nodes: BTreeMap<String, NodeEntry<'a>>,
    pub preorder: Vec<String>,
    siblings: BTreeMap<SiblingKey, Vec<String>>,
}

impl<'a> TreeIndex<'a> {
    pub fn build(document: &'a PenDocument) -> Result<Self, DiffError> {
        let mut index = Self {
            nodes: BTreeMap::new(),
            preorder: Vec::new(),
            siblings: BTreeMap::new(),
        };
        index.walk_roots(PageRef::DocumentRoot, &document.children)?;
        if let Some(pages) = &document.pages {
            for page in pages {
                index.walk_roots(PageRef::PageId(page.id.clone()), &page.children)?;
            }
        }
        Ok(index)
    }

    fn walk_roots(&mut self, page: PageRef, roots: &'a [PenNode]) -> Result<(), DiffError> {
        let mut stack = Vec::new();
        push_children(&mut stack, &page, None, roots)?;
        while let Some((page, parent, child_index, node)) = stack.pop() {
            let id = node_id_of(node).to_owned();
            self.preorder.push(id.clone());
            self.nodes.insert(
                id.clone(),
                NodeEntry {
                    page: page.clone(),
                    parent: parent.clone(),
                    index: child_index,
                    node,
                },
            );
            self.siblings
                .entry(SiblingKey {
                    scope: ScopeKey::for_page(&page),
                    parent,
                })
                .or_default()
                .push(id.clone());
            if let Some(node_children) = children(node) {
                push_children(&mut stack, &page, Some(id), node_children)?;
            }
        }
        Ok(())
    }

    pub fn entry(&self, id: &str) -> Option<&NodeEntry<'a>> {
        self.nodes.get(id)
    }

    pub fn sibling_len(&self, page: &PageRef, parent: Option<&str>) -> usize {
        self.siblings
            .get(&SiblingKey {
                scope: ScopeKey::for_page(page),
                parent: parent.map(str::to_owned),
            })
            .map_or(0, Vec::len)
    }
}

fn push_children<'a>(
    stack: &mut Vec<(PageRef, Option<String>, u32, &'a PenNode)>,
    page: &PageRef,
    parent: Option<String>,
    nodes: &'a [PenNode],
) -> Result<(), DiffError> {
    for (index, node) in nodes.iter().enumerate().rev() {
        stack.push((
            page.clone(),
            parent.clone(),
            u32::try_from(index).map_err(|_| DiffError::IndexOverflow)?,
            node,
        ));
    }
    Ok(())
}

pub(crate) fn node_kind(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::TextArea(_) => "text_area",
        PenNode::Select(_) => "select",
        PenNode::Switch(_) => "switch",
        PenNode::Checkbox(_) => "checkbox",
        PenNode::Slider(_) => "slider",
        PenNode::RadioGroup(_) => "radio_group",
        PenNode::NumberInput(_) => "number_input",
        PenNode::Progress(_) => "progress",
        PenNode::Tabs(_) => "tabs",
        PenNode::Ref(_) => "ref",
    }
}

pub(crate) fn is_m1_basic_kind(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_)
            | PenNode::Group(_)
            | PenNode::Rectangle(_)
            | PenNode::Ellipse(_)
            | PenNode::Line(_)
            | PenNode::Polygon(_)
            | PenNode::Path(_)
            | PenNode::Text(_)
    )
}
