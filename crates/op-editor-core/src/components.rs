//! Component library — reusable design-system subtrees.
//!
//! Faithful copy of `openpencil-shell-core::document::components`
//! adapted to `op-editor-core`. The crucial difference: a `Component`'s
//! `root` is a canonical `jian_ops_schema::node::PenNode` (the one
//! document model `EditorState` is built on), not shell-core's private
//! `Node`. Component ids stay `op-editor-core::NodeId` string ids so a
//! component is addressable through the same id space as the tree.
//!
//! v1 scope: data storage + lookup by id / name. Instance insertion
//! and the Components panel UI are follow-ups; the data shape lands
//! first so the canonical `.op` loader has somewhere to put what it
//! reads.

use crate::node_id::NodeId;
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
}
