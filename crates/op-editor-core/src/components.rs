//! Component library — reusable design-system subtrees.
//!
//! Faithful copy of `openpencil-shell-core::document::components`
//! adapted to `op-editor-core`. The crucial difference: a `Component`'s
//! `root` is a canonical `jian_ops_schema::node::PenNode` (the one
//! document model `EditorState` is built on), not shell-core's private
//! `Node`. Component ids stay `op-editor-core::NodeId` string ids so a
//! component is addressable through the same id space as the tree.
//!
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::conversion::ConversionKind;
use jian_ops_schema::node::PenNode;

/// One reusable design fragment. `root` is the canonical subtree that
/// gets cloned into a design when the user creates an instance.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub id: NodeId,
    pub name: String,
    pub root: PenNode,
}

/// Per-document component registry. Populated by the canonical loader;
/// queried by instance-insertion + drag-drop UX.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComponentLibrary {
    pub components: Vec<Component>,
}

impl ComponentLibrary {
    pub fn find_by_id(&self, id: &NodeId) -> Option<&Component> {
        self.components.iter().find(|c| &c.id == id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }

    /// Insert a component. Replace-on-duplicate-id mirrors the TS
    /// app's behavior on "Save as Component" of an already-component'd
    /// Frame.
    pub fn insert(&mut self, c: Component) {
        if let Some(pos) = self.components.iter().position(|x| x.id == c.id) {
            self.components[pos] = c;
        } else {
            self.components.push(c);
        }
    }

    /// Rename a registered component by id. Returns true when found +
    /// renamed. The empty-string rename is rejected.
    pub fn rename(&mut self, id: &NodeId, new_name: impl Into<String>) -> bool {
        let name: String = new_name.into();
        if name.trim().is_empty() {
            return false;
        }
        if let Some(c) = self.components.iter_mut().find(|c| &c.id == id) {
            c.name = name;
            true
        } else {
            false
        }
    }

    /// Remove a component by id. Returns true when a component was
    /// removed. Live instances (clones already on a page) are
    /// unaffected — the registry only holds the prototype.
    pub fn remove(&mut self, id: &NodeId) -> bool {
        if let Some(pos) = self.components.iter().position(|c| &c.id == id) {
            self.components.remove(pos);
            true
        } else {
            false
        }
    }

    /// True when no components are registered.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Count of registered components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Build a runtime component registry from persisted canonical
    /// `reusable: true` frame nodes. TS treats reusable nodes in the
    /// document tree as component definitions; this keeps loaded `.op`
    /// files addressable even though the registry itself is transient.
    pub fn from_document(doc: &jian_ops_schema::PenDocument) -> Self {
        fn walk(node: &PenNode, lib: &mut ComponentLibrary) {
            if let PenNode::Frame(frame) = node {
                if frame.reusable == Some(true) {
                    let id = NodeId::new(frame.base.id.clone());
                    let name = frame
                        .base
                        .name
                        .clone()
                        .unwrap_or_else(|| frame.base.id.clone());
                    lib.insert(Component {
                        id,
                        name,
                        root: node.clone(),
                    });
                }
            }
            if let Some(children) = node.children() {
                for child in children {
                    walk(child, lib);
                }
            }
        }

        let mut lib = ComponentLibrary::default();
        if let Some(pages) = doc.pages.as_ref() {
            for page in pages {
                for child in &page.children {
                    walk(child, &mut lib);
                }
            }
        }
        for child in &doc.children {
            walk(child, &mut lib);
        }
        add_conversion_components(doc, &mut lib);
        lib
    }
}

fn add_conversion_components(doc: &jian_ops_schema::PenDocument, lib: &mut ComponentLibrary) {
    let Some(spec) = doc.conversion.as_ref() else {
        return;
    };
    for entry in &spec.entries {
        if entry.kind != ConversionKind::Component {
            continue;
        }
        let Some(node_id) = entry.node_id.as_deref().filter(|id| !id.is_empty()) else {
            continue;
        };
        let Some(root) = find_node_in_document(doc, node_id) else {
            continue;
        };
        if !is_component_root(root) {
            continue;
        }
        let name = root
            .base()
            .name
            .clone()
            .unwrap_or_else(|| entry.key.clone());
        lib.insert(Component {
            id: NodeId::new(node_id),
            name,
            root: root.clone(),
        });
    }
}

fn find_node_in_document<'a>(
    doc: &'a jian_ops_schema::PenDocument,
    node_id: &str,
) -> Option<&'a PenNode> {
    fn walk<'a>(nodes: &'a [PenNode], node_id: &str) -> Option<&'a PenNode> {
        for node in nodes {
            if node.id_str() == node_id {
                return Some(node);
            }
            if let Some(children) = node.children() {
                if let Some(hit) = walk(children, node_id) {
                    return Some(hit);
                }
            }
        }
        None
    }
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(hit) = walk(&page.children, node_id) {
                return Some(hit);
            }
        }
    }
    walk(&doc.children, node_id)
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

/// Swap the node with `id` for `replacement` in place (same sibling
/// slot), anywhere in `children`. True when the swap happened.
fn replace_node_in_children(children: &mut [PenNode], id: &NodeId, replacement: PenNode) -> bool {
    let mut slot = Some(replacement);
    replace_node_inner(children, id, &mut slot)
}

fn replace_node_inner(children: &mut [PenNode], id: &NodeId, slot: &mut Option<PenNode>) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id_str() == id.as_str()) {
        if let Some(replacement) = slot.take() {
            children[idx] = replacement;
            return true;
        }
        return false;
    }
    for child in children.iter_mut() {
        if let Some(grand) = child.children_mut() {
            if replace_node_inner(grand, id, slot) {
                return true;
            }
        }
    }
    false
}

fn clear_reusable_in_document(doc: &mut jian_ops_schema::PenDocument, id: &NodeId) {
    if let Some(pages) = doc.pages.as_mut() {
        for page in pages {
            if let Some(live) = walkers::find_node_mut(&mut page.children, id) {
                set_reusable(live, false);
                return;
            }
        }
    }
    if let Some(live) = walkers::find_node_mut(&mut doc.children, id) {
        set_reusable(live, false);
    }
}

impl EditorState {
    /// Promote an active-page Frame / Group / Rectangle into the
    /// runtime component registry. Frames also carry the canonical
    /// persisted `reusable` flag so reloads can rebuild the registry.
    pub fn create_component_from_node(&mut self, node_id: &NodeId, name: &str) -> bool {
        let name = name.trim();
        if !node_id.is_real() || name.is_empty() {
            return false;
        }
        let Some(mut root) = walkers::find_node(self.active_children(), node_id).cloned() else {
            return false;
        };
        if !is_component_root(&root) {
            return false;
        }

        let snap = self.snapshot_for_history();
        set_reusable(&mut root, true);
        if let Some(live) = walkers::find_node_mut(self.active_children_mut(), node_id) {
            set_reusable(live, true);
        }
        self.components.insert(Component {
            id: node_id.clone(),
            name: name.to_string(),
            root,
        });
        self.history_push_past(snap);
        true
    }

    /// Promote a node using its current layer name as the component
    /// label. Used by direct UI affordances that do not prompt.
    pub fn create_component_from_node_name(&mut self, node_id: &NodeId) -> bool {
        let Some(node) = walkers::find_node(self.active_children(), node_id) else {
            return false;
        };
        let name = node
            .base()
            .name
            .clone()
            .unwrap_or_else(|| node.id_str().to_string());
        self.create_component_from_node(node_id, &name)
    }

    /// Clone a registered component onto the active page with fresh
    /// ids. The inserted clone is standalone, so any reusable marker
    /// on the prototype root is cleared.
    pub fn instantiate_component(&mut self, component_id: &NodeId) -> Option<NodeId> {
        let (template, name) = {
            let component = self.components.find_by_id(component_id)?;
            (component.root.clone(), component.name.clone())
        };
        let snap = self.snapshot_for_history();
        let mut next_id = self.next_node_id_seed()?;
        let mut taken = self.collect_node_ids();
        let mut clone = walkers::deep_clone_with_new_ids(&template, &mut next_id, &mut taken);
        set_reusable(&mut clone, false);
        walkers::translate_subtree(&mut clone, 20.0, 20.0);
        clone.base_mut().name = Some(name);
        let new_id = NodeId::new(clone.base().id.clone());
        self.active_children_mut().push(clone);
        self.set_single_selection(new_id.clone());
        self.history_push_past(snap);
        Some(new_id)
    }

    /// TS `detachComponent`. A reusable component sheds its flag
    /// (and registry entry); a `Ref` instance materializes into an
    /// independent subtree — `descendants` overrides applied,
    /// instance props overlaid, fresh ids — replacing the Ref in
    /// place. Returns the surviving node's id.
    pub fn detach_component(&mut self, node_id: &NodeId) -> Option<NodeId> {
        let node = walkers::find_node(self.active_children(), node_id)?.clone();
        let registered_component = self.components.find_by_id(node_id).is_some();
        let reusable_frame = matches!(&node, PenNode::Frame(frame) if frame.reusable == Some(true));
        if is_component_root(&node) && (registered_component || reusable_frame) {
            let snap = self.snapshot_for_history();
            if let Some(live) = walkers::find_node_mut(self.active_children_mut(), node_id) {
                set_reusable(live, false);
            }
            self.components.remove(node_id);
            self.history_push_past(snap);
            return Some(node_id.clone());
        }

        match &node {
            PenNode::Ref(reference) => {
                let component =
                    crate::ref_resolve::find_component_node(&self.doc, &reference.target)?;
                let merged = crate::ref_resolve::materialize_instance(&node, &component)?;
                let snap = self.snapshot_for_history();
                let mut next_id = self.next_node_id_seed()?;
                let mut taken = self.collect_node_ids();
                let detached = walkers::deep_clone_with_new_ids(&merged, &mut next_id, &mut taken);
                let new_id = NodeId::new(detached.base().id.clone());
                if !replace_node_in_children(self.active_children_mut(), node_id, detached) {
                    return None;
                }
                self.set_single_selection(new_id.clone());
                self.history_push_past(snap);
                Some(new_id)
            }
            _ => None,
        }
    }

    /// Remove a component registration. If the source frame is still
    /// on the active page, clear its persisted reusable marker too.
    pub fn delete_component(&mut self, component_id: &NodeId) -> bool {
        if self.components.find_by_id(component_id).is_none() {
            return false;
        }
        let snap = self.snapshot_for_history();
        self.components.remove(component_id);
        clear_reusable_in_document(&mut self.doc, component_id);
        self.history_push_past(snap);
        true
    }

    /// Rename a registered component. This updates the runtime
    /// registry label; the source node's visual/layer name is left
    /// unchanged.
    pub fn rename_component(&mut self, component_id: &NodeId, name: &str) -> bool {
        self.components.rename(component_id, name.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::frame;

    fn sample_node() -> PenNode {
        // A minimal canonical container node fixture.
        frame("f1", "Frame", 0.0, 0.0, 100.0, 100.0, Vec::new())
    }

    fn comp(id: &str, name: &str) -> Component {
        Component {
            id: NodeId::new(id),
            name: name.into(),
            root: sample_node(),
        }
    }

    #[test]
    fn find_by_id_returns_match() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        lib.insert(comp("n11", "Card"));
        assert_eq!(lib.find_by_id(&NodeId::new("n10")).unwrap().name, "Button");
        assert!(lib.find_by_id(&NodeId::new("n99")).is_none());
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
    fn insert_replaces_duplicate_id() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        lib.insert(comp("n10", "ButtonV2"));
        assert_eq!(lib.len(), 1);
        assert_eq!(
            lib.find_by_id(&NodeId::new("n10")).unwrap().name,
            "ButtonV2"
        );
    }

    #[test]
    fn rename_rejects_empty() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        assert!(!lib.rename(&NodeId::new("n10"), "  "));
        assert!(lib.rename(&NodeId::new("n10"), "Renamed"));
        assert_eq!(lib.find_by_id(&NodeId::new("n10")).unwrap().name, "Renamed");
    }

    #[test]
    fn remove_returns_false_for_unknown() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp("n10", "Button"));
        assert!(!lib.remove(&NodeId::new("n99")));
        assert!(lib.remove(&NodeId::new("n10")));
        assert!(lib.is_empty());
    }

    #[test]
    fn from_document_indexes_reusable_frames() {
        let mut doc = jian_ops_schema::PenDocument {
            version: "1.0.0".into(),
            name: None,
            themes: None,
            variables: None,
            pages: None,
            children: vec![sample_node()],
            format_version: None,
            id: None,
            app: None,
            routes: None,
            state: None,
            lifecycle: None,
            logic_modules: None,
            design_md: None,
            conversion: None,
        };
        if let PenNode::Frame(f) = &mut doc.children[0] {
            f.reusable = Some(true);
        }
        let lib = ComponentLibrary::from_document(&doc);
        assert_eq!(lib.len(), 1);
        assert_eq!(lib.find_by_id(&NodeId::new("f1")).unwrap().name, "Frame");
    }
}
