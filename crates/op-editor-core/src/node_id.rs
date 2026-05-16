//! Stable node identity for the editor.
//!
//! `NodeId` is a `String`-backed newtype matching the canonical `.op`
//! schema's `PenNodeBase.id` (and `PenPage.id`). The empty string is
//! the `NONE` sentinel meaning "no node"; `NodeId::new` rejects it.
//!
//! This mirrors `openpencil-shell-core::document::NodeId` (the just-
//! completed Task 4.3 string-newtype migration) so the later facade
//! swap (Task 4.7) is a straight re-export. The shell-core-specific
//! `to_widget_id` helper is intentionally omitted here — it depends on
//! `crate::widgets::WidgetId`, which lives in the widget layer, not the
//! editor-state layer.

/// Stable node identity — a string id matching the canonical `.op`
/// schema's `PenNodeBase.id`. The empty string is the `NONE` sentinel
/// meaning "no node"; `NodeId::new` rejects it.
///
/// Editor-minted ids follow the `n{N}` convention (`n1`, `n2`, …);
/// canonical-schema loads keep whatever arbitrary string the file
/// authored. Ordering is intentionally NOT derived — a node tree has
/// no meaningful id order, only insertion order in `children`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// The "no node" sentinel — an empty-string id. A `const` over an
    /// owned `String` is feasible because `String::new()` is a const
    /// fn, so call sites keep using `NodeId::NONE` unchanged.
    pub const NONE: NodeId = NodeId(String::new());

    /// True when this id is not the `NONE` sentinel.
    #[inline]
    pub fn is_real(&self) -> bool {
        !self.0.is_empty()
    }

    /// Construct a real (non-sentinel) id. Panics on an empty string
    /// (reserved for `NodeId::NONE`).
    #[inline]
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        if id.is_empty() {
            panic!("NodeId::new(\"\") — the empty id is reserved for NodeId::NONE");
        }
        Self(id)
    }

    /// Non-panicking construction. Returns `None` for an empty string
    /// (the NONE sentinel).
    #[inline]
    pub fn new_opt(id: impl Into<String>) -> Option<Self> {
        let id = id.into();
        if id.is_empty() { None } else { Some(Self(id)) }
    }

    /// Inner string id.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<NodeId> for String {
    fn from(id: NodeId) -> String {
        id.0
    }
}

impl From<&NodeId> for String {
    fn from(id: &NodeId) -> String {
        id.0.clone()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_sentinel_is_not_real() {
        assert!(!NodeId::NONE.is_real());
        assert_eq!(NodeId::NONE.as_str(), "");
    }

    #[test]
    fn new_builds_a_real_id() {
        let id = NodeId::new("n1");
        assert!(id.is_real());
        assert_eq!(id.as_str(), "n1");
    }

    #[test]
    #[should_panic]
    fn new_rejects_empty() {
        let _ = NodeId::new("");
    }

    #[test]
    fn new_opt_maps_empty_to_none() {
        assert_eq!(NodeId::new_opt(""), None);
        assert_eq!(NodeId::new_opt("n2"), Some(NodeId::new("n2")));
    }
}
