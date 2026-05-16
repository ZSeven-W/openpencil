//! Component library — reusable design-system Frames. Each
//! `Component` carries a root subtree the user can drop into a
//! design as an Instance. v1 scope: data storage + lookup by id /
//! name. Instance NodeKind (a NodeKind variant that references a
//! Component by id and resolves at paint time) + the Components
//! panel UI are follow-ups; the data shape lands first so the
//! canonical `.op` loader has somewhere to put what it reads.

use super::{Document, Node, NodeId};

/// One reusable design fragment. `root` is the subtree that gets
/// cloned into a design when the user creates an instance.
#[derive(Debug, Clone)]
pub struct Component {
    pub id: NodeId,
    pub name: String,
    pub root: Node,
}

/// Per-document component registry. Populated by the canonical
/// loader; queried by future instance-insertion + drag-drop UX.
#[derive(Debug, Clone, Default)]
pub struct ComponentLibrary {
    pub components: Vec<Component>,
}

impl ComponentLibrary {
    pub fn find_by_id(&self, id: NodeId) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }
    pub fn find_by_name(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }
    pub fn insert(&mut self, c: Component) {
        // Replace-on-duplicate-id mirrors the TS app's behavior on
        // "Save as Component" of an already-component'd Frame.
        if let Some(pos) = self.components.iter().position(|x| x.id == c.id) {
            self.components[pos] = c;
        } else {
            self.components.push(c);
        }
    }
    /// Rename a registered component by id. Returns true when the
    /// component was found + renamed. The empty-string rename is
    /// rejected (components without a name are useless in the UI
    /// + would mislead LLM clients that browse via list_components).
    pub fn rename(&mut self, id: NodeId, new_name: impl Into<String>) -> bool {
        let name: String = new_name.into();
        if name.trim().is_empty() {
            return false;
        }
        if let Some(c) = self.components.iter_mut().find(|c| c.id == id) {
            c.name = name;
            true
        } else {
            false
        }
    }
    /// Remove a component by id. Returns true when a component was
    /// removed, false when the id didn't resolve. Live instances
    /// (clones already on the page) are unaffected — they carry
    /// their own ids; the registry only holds the prototype.
    pub fn remove(&mut self, id: NodeId) -> bool {
        if let Some(pos) = self.components.iter().position(|c| c.id == id) {
            self.components.remove(pos);
            true
        } else {
            false
        }
    }
}

impl Document {
    /// "Save as Component" mutator. Promotes the anchor-selected node
    /// (must be a Frame or Group) to a registered Component in the
    /// library + leaves the node in place. Returns the new component's
    /// id, or None when nothing is selected / selection isn't a
    /// container kind. Mirrors TS "Make Component" in the right-click
    /// context menu.
    /// Like `create_component_from_selected`, but takes the target
    /// node id explicitly instead of reading `self.selected`.
    /// Used by the MCP `create_component` write tool where there
    /// is no UI selection state. Returns the component's id on
    /// success (== the source node id), or `None` if the id
    /// doesn't resolve or the node isn't a Frame / Group.
    pub fn create_component_from_node(
        &mut self,
        node_id: NodeId,
        name: impl Into<String>,
    ) -> Option<NodeId> {
        let page = self.active_page()?;
        let node = page.find(&node_id)?;
        if !matches!(node.kind, super::NodeKind::Frame | super::NodeKind::Group) {
            return None;
        }
        let comp = Component {
            id: node_id.clone(),
            name: name.into(),
            root: node.clone(),
        };
        self.components.insert(comp);
        Some(node_id)
    }

    pub fn create_component_from_selected(&mut self, name: impl Into<String>) -> Option<NodeId> {
        if !self.selected.is_real() {
            return None;
        }
        let target = self.selected.clone();
        let page = self.active_page()?;
        let node = page.find(&target)?;
        if !matches!(node.kind, super::NodeKind::Frame | super::NodeKind::Group) {
            return None;
        }
        let root = node.clone();
        let comp = Component {
            id: target.clone(),
            name: name.into(),
            root,
        };
        self.components.insert(comp);
        Some(target)
    }

    /// "Insert Instance" mutator. Deep-clones the component's root
    /// subtree with fresh `NodeId`s and appends it to the active
    /// page's top-level children. Returns the new instance's root
    /// id, or None when the component id is unknown / the next-id
    /// allocator can't advance. Mirrors TS drag-from-Components-
    /// panel insertion.
    pub fn instantiate_component(
        &mut self,
        component_id: NodeId,
        next_id: &mut u64,
    ) -> Option<NodeId> {
        let comp = self.components.find_by_id(component_id)?.clone();
        let pre = self.snapshot_for_history();
        // Mint fresh ids past the high-water mark (same guard as
        // duplicate_selected / group_selected).
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let mut taken = self.collect_node_ids();
        let new_root = clone_node_with_new_ids(&comp.root, next_id, &mut taken)?;
        let new_id = new_root.id.clone();
        let active = self.active_page_index;
        self.pages.get_mut(active)?.children.push(new_root);
        self.selected_set.clear();
        self.selected_set.push(new_id.clone());
        self.selected = new_id.clone();
        self.history_push_past(pre);
        Some(new_id)
    }
}

/// Deep-clone `src` with a fresh `n{N}` id per node. Returns `None`
/// on `u64` counter exhaustion.
fn clone_node_with_new_ids(
    src: &Node,
    next_id: &mut u64,
    taken: &mut std::collections::HashSet<NodeId>,
) -> Option<Node> {
    let mut out = src.clone();
    out.id = super::walkers::alloc_n_id(next_id, taken)?;
    let mut children = Vec::with_capacity(src.children.len());
    for c in &src.children {
        children.push(clone_node_with_new_ids(c, next_id, taken)?);
    }
    out.children = children;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::NodeKind;

    fn comp(id: &str, name: &str) -> Component {
        Component {
            id: NodeId::new(id),
            name: name.into(),
            root: Node::leaf(id, NodeKind::Frame, name),
        }
    }

    #[test]
    fn find_by_id_returns_match() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        lib.insert(comp("n11", "Card"));
        assert_eq!(lib.find_by_id(NodeId::new("n10")).unwrap().name, "Button");
        assert_eq!(lib.find_by_id(NodeId::new("n11")).unwrap().name, "Card");
        assert!(lib.find_by_id(NodeId::new("n99")).is_none());
    }

    #[test]
    fn find_by_name_returns_first_match() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        lib.insert(comp("n11", "Card"));
        assert_eq!(lib.find_by_name("Card").unwrap().id, NodeId::new("n11"));
        assert!(lib.find_by_name("Unknown").is_none());
    }

    #[test]
    fn create_component_from_selected_promotes_frame() {
        use crate::document::NodeKind;
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        page.children.push(Node::leaf("n10", NodeKind::Frame, "Frame"));
        doc.set_single_selection(NodeId::new("n10"));
        let id = doc.create_component_from_selected("Button");
        assert_eq!(id, Some(NodeId::new("n10")));
        let lib_entry = doc.components.find_by_id(NodeId::new("n10")).unwrap();
        assert_eq!(lib_entry.name, "Button");
        // Original node still exists on the page.
        assert!(doc
            .active_page()
            .unwrap()
            .find(&NodeId::new("n10"))
            .is_some());
    }

    #[test]
    fn create_component_rejects_non_container_kinds() {
        use crate::document::NodeKind;
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        page.children.push(Node::leaf("n10", NodeKind::Rect, "r"));
        doc.set_single_selection(NodeId::new("n10"));
        assert!(doc.create_component_from_selected("Card").is_none());
        assert!(doc.components.components.is_empty());
    }

    #[test]
    fn instantiate_component_clones_subtree_with_fresh_ids() {
        use crate::document::NodeKind;
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut frame = Node::with_children(
            "n10",
            NodeKind::Frame,
            "F",
            vec![
                Node::leaf("n11", NodeKind::Rect, "r1"),
                Node::leaf("n12", NodeKind::Rect, "r2"),
            ],
        );
        // Bound the frame so the resulting clone is meaningful.
        frame.bounds = crate::Rect::xywh(0.0, 0.0, 100.0, 100.0);
        page.children.push(frame);
        doc.set_single_selection(NodeId::new("n10"));
        doc.create_component_from_selected("Card");
        // Now instantiate.
        let mut next = 100u64;
        let inst_id = doc.instantiate_component(NodeId::new("n10"), &mut next).unwrap();
        // Fresh root id is a freshly-minted `n{N}` id, distinct
        // from the source frame's `n10`.
        assert_ne!(inst_id, NodeId::new("n10"));
        let inst = doc.active_page().unwrap().find(&inst_id).unwrap();
        // Same shape: 2 children.
        assert_eq!(inst.children.len(), 2);
        // Child ids fresh (not 11/12).
        for c in &inst.children {
            assert_ne!(c.id, NodeId::new("n11"));
            assert_ne!(c.id, NodeId::new("n12"));
        }
        // Selection landed on the new instance root.
        assert_eq!(doc.selected, inst_id);
        // History snapshot pushed.
        assert_eq!(doc.history.past.len(), 1);
    }

    #[test]
    fn instantiate_component_unknown_id_returns_none() {
        let mut doc = Document::empty();
        let mut next = 100u64;
        assert!(doc
            .instantiate_component(NodeId::new("n99"), &mut next)
            .is_none());
    }

    #[test]
    fn create_component_no_op_without_selection() {
        let mut doc = Document::empty();
        doc.clear_selection();
        assert!(doc.create_component_from_selected("X").is_none());
    }

    #[test]
    fn insert_replaces_duplicate_id() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        lib.insert(comp("n10", "ButtonV2"));
        assert_eq!(lib.components.len(), 1);
        assert_eq!(lib.find_by_id(NodeId::new("n10")).unwrap().name, "ButtonV2");
    }
}
