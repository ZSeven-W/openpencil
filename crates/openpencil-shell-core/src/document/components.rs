//! Component library — reusable design-system Frames. Each
//! `Component` carries a root subtree the user can drop into a
//! design as an Instance. v1 scope: data storage + lookup by id /
//! name. Instance NodeKind (a NodeKind variant that references a
//! Component by id and resolves at paint time) + the Components
//! panel UI are follow-ups; the data shape lands first so the
//! canonical `.op` loader has somewhere to put what it reads.

use super::{Node, NodeId};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::NodeKind;

    fn comp(id: u64, name: &str) -> Component {
        Component {
            id: NodeId::new(id),
            name: name.into(),
            root: Node::leaf(id, NodeKind::Frame, name),
        }
    }

    #[test]
    fn find_by_id_returns_match() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp(10, "Button"));
        lib.insert(comp(11, "Card"));
        assert_eq!(lib.find_by_id(NodeId::new(10)).unwrap().name, "Button");
        assert_eq!(lib.find_by_id(NodeId::new(11)).unwrap().name, "Card");
        assert!(lib.find_by_id(NodeId::new(99)).is_none());
    }

    #[test]
    fn find_by_name_returns_first_match() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp(10, "Button"));
        lib.insert(comp(11, "Card"));
        assert_eq!(lib.find_by_name("Card").unwrap().id, NodeId::new(11));
        assert!(lib.find_by_name("Unknown").is_none());
    }

    #[test]
    fn insert_replaces_duplicate_id() {
        let mut lib = ComponentLibrary::default();
        lib.insert(comp(10, "Button"));
        lib.insert(comp(10, "ButtonV2"));
        assert_eq!(lib.components.len(), 1);
        assert_eq!(lib.find_by_id(NodeId::new(10)).unwrap().name, "ButtonV2");
    }
}
