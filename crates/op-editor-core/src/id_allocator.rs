//! Document id allocation policies.
//!
//! The canonical schema accepts any non-empty string id. Standalone editing
//! keeps the historical `n{counter}` convention, while collaboration can use
//! an owner-assigned namespace without requiring randomness in this crate.
//! Callers provide the live id set so both policies share the same collision
//! check and immediately reserve every returned id.

use crate::pen_node_ext::PenNodeExt;
use crate::NodeId;
use jian_ops_schema::PenDocument;
use op_util::collab_id::NamespacedId;
use std::collections::HashSet;

pub use op_util::collab_id::{PeerNamespace, MAX_PEER_NAMESPACE_LEN};

/// Typed failure from a document id allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdAllocError {
    /// Advancing the allocator's `u64` counter would overflow.
    CounterExhausted,
}

impl std::fmt::Display for IdAllocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CounterExhausted => f.write_str("document id counter exhausted"),
        }
    }
}

impl std::error::Error for IdAllocError {}

/// A document id source that reserves fresh ids in the caller's live set.
pub trait IdAllocator {
    /// Allocate and reserve one id.
    ///
    /// Existing candidates are skipped. No fallback id is produced when the
    /// counter is exhausted.
    fn allocate(&mut self, taken: &mut HashSet<NodeId>) -> Result<NodeId, IdAllocError>;
}

/// Session-storable document allocator policy.
///
/// Hosts can keep one value for the lifetime of a standalone document or
/// collaboration peer and pass it through every allocating editor operation.
#[derive(Debug, PartialEq, Eq)]
pub enum DocumentIdAllocator {
    /// Historical standalone `n{counter}` allocation.
    Sequential(SequentialIdAllocator),
    /// Owner-assigned collaboration namespace allocation.
    Namespaced(NamespacedIdAllocator),
}

impl DocumentIdAllocator {
    /// Build the standalone policy at an explicit counter.
    pub const fn sequential(next: u64) -> Self {
        Self::Sequential(SequentialIdAllocator::new(next))
    }

    /// Build the collaboration policy at an explicit persisted high-water.
    pub const fn namespaced(namespace: PeerNamespace, next: u64) -> Self {
        Self::Namespaced(NamespacedIdAllocator::new(namespace, next))
    }

    /// Build a standalone allocator above every existing canonical id.
    pub fn sequential_for_document(doc: &PenDocument) -> Result<Self, IdAllocError> {
        Ok(Self::sequential(next_sequential_counter(doc)?))
    }

    /// Resume a namespace above every id already authored by that peer.
    pub fn namespaced_for_document(
        doc: &PenDocument,
        namespace: PeerNamespace,
    ) -> Result<Self, IdAllocError> {
        let next = next_namespaced_counter(doc, &namespace)?;
        Ok(Self::namespaced(namespace, next))
    }

    /// Counter that will be considered by the next allocation attempt.
    pub const fn next_counter(&self) -> u64 {
        match self {
            Self::Sequential(allocator) => allocator.next_counter(),
            Self::Namespaced(allocator) => allocator.next_counter(),
        }
    }
}

impl IdAllocator for DocumentIdAllocator {
    fn allocate(&mut self, taken: &mut HashSet<NodeId>) -> Result<NodeId, IdAllocError> {
        match self {
            Self::Sequential(allocator) => allocator.allocate(taken),
            Self::Namespaced(allocator) => allocator.allocate(taken),
        }
    }
}

/// Standalone allocator preserving OpenPencil's historical `n{counter}` ids.
#[derive(Debug, PartialEq, Eq)]
pub struct SequentialIdAllocator {
    next: u64,
}

impl SequentialIdAllocator {
    /// Start from the caller-provided counter, clamping zero to legacy `n1`.
    pub const fn new(next: u64) -> Self {
        Self {
            next: if next == 0 { 1 } else { next },
        }
    }

    /// Counter that will be considered by the next allocation attempt.
    pub const fn next_counter(&self) -> u64 {
        self.next
    }

    /// Start no lower than both a caller-owned cursor and document high-water.
    pub fn for_document(doc: &PenDocument, requested_next: u64) -> Result<Self, IdAllocError> {
        Ok(Self::new(requested_next.max(next_sequential_counter(doc)?)))
    }
}

impl IdAllocator for SequentialIdAllocator {
    fn allocate(&mut self, taken: &mut HashSet<NodeId>) -> Result<NodeId, IdAllocError> {
        allocate_with(taken, &mut self.next, |counter| format!("n{counter}"))
    }
}

/// Collaboration allocator producing `c_<namespace>_<counter>` ids.
///
/// This allocator is intentionally not `Clone`: one session-owned instance
/// must reserve every id for its peer namespace.
#[derive(Debug, PartialEq, Eq)]
pub struct NamespacedIdAllocator {
    namespace: PeerNamespace,
    next: u64,
}

impl NamespacedIdAllocator {
    /// Build an allocator for an already validated owner-assigned namespace.
    pub const fn new(namespace: PeerNamespace, next: u64) -> Self {
        Self { namespace, next }
    }

    /// The exact owner-assigned namespace used by this allocator.
    pub fn namespace(&self) -> &PeerNamespace {
        &self.namespace
    }

    /// Counter that will be considered by the next allocation attempt.
    pub const fn next_counter(&self) -> u64 {
        self.next
    }
}

impl IdAllocator for NamespacedIdAllocator {
    fn allocate(&mut self, taken: &mut HashSet<NodeId>) -> Result<NodeId, IdAllocError> {
        let namespace = &self.namespace;
        allocate_with(taken, &mut self.next, |counter| {
            NamespacedId::new(namespace.clone(), counter).to_string()
        })
    }
}

fn allocate_with(
    taken: &mut HashSet<NodeId>,
    next: &mut u64,
    format_id: impl Fn(u64) -> String,
) -> Result<NodeId, IdAllocError> {
    loop {
        let counter = *next;
        *next = next.checked_add(1).ok_or(IdAllocError::CounterExhausted)?;
        let candidate = NodeId::new(format_id(counter));
        if taken.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
}

/// Collect page and node ids across the complete canonical document.
///
/// Page ids and nodes share one collision domain. The walk is iterative so a
/// validated but deeply nested imported tree does not consume the call stack.
pub fn collect_document_ids(doc: &PenDocument) -> HashSet<NodeId> {
    let mut ids = HashSet::new();
    let mut stack: Vec<_> = doc.children.iter().collect();
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(id) = NodeId::new_opt(page.id.as_str()) {
                ids.insert(id);
            }
            stack.extend(page.children.iter());
        }
    }
    while let Some(node) = stack.pop() {
        if let Some(id) = NodeId::new_opt(node.id_str()) {
            ids.insert(id);
        }
        if let Some(children) = node.children() {
            stack.extend(children.iter());
        }
    }
    ids
}

/// First standalone counter above every canonical `n{counter}` id.
pub fn next_sequential_counter(doc: &PenDocument) -> Result<u64, IdAllocError> {
    let mut max = None;
    for id in collect_document_ids(doc) {
        if let Some(counter) = crate::walkers::parse_n_id(id.as_str()) {
            max = Some(max.map_or(counter, |current: u64| current.max(counter)));
        }
    }
    match max {
        Some(max) => max.checked_add(1).ok_or(IdAllocError::CounterExhausted),
        None => Ok(1),
    }
}

/// First counter above every canonical id in `namespace`.
pub fn next_namespaced_counter(
    doc: &PenDocument,
    namespace: &PeerNamespace,
) -> Result<u64, IdAllocError> {
    let mut max = None;
    for id in collect_document_ids(doc) {
        let Ok(parsed) = NamespacedId::parse(id.as_str()) else {
            continue;
        };
        if parsed.namespace() == namespace {
            let counter = parsed.counter();
            max = Some(max.map_or(counter, |current: u64| current.max(counter)));
        }
    }
    match max {
        Some(max) => max.checked_add(1).ok_or(IdAllocError::CounterExhausted),
        None => Ok(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{frame, rect};

    fn taken(ids: &[&str]) -> HashSet<NodeId> {
        ids.iter().copied().map(NodeId::new).collect()
    }

    #[test]
    fn id_allocator_legacy_sequence_uses_n_prefix() {
        let mut allocator = SequentialIdAllocator::new(1);
        let mut ids = HashSet::new();

        assert_eq!(allocator.allocate(&mut ids).unwrap(), NodeId::new("n1"));
        assert_eq!(allocator.allocate(&mut ids).unwrap(), NodeId::new("n2"));
        assert_eq!(allocator.next_counter(), 3);
    }

    #[test]
    fn id_allocator_legacy_zero_seed_clamps_to_one() {
        let mut allocator = SequentialIdAllocator::new(0);
        let mut ids = HashSet::new();

        assert_eq!(allocator.allocate(&mut ids).unwrap(), NodeId::new("n1"));
        assert_eq!(allocator.next_counter(), 2);
    }

    #[test]
    fn id_allocator_skips_taken_candidates() {
        let mut allocator = SequentialIdAllocator::new(1);
        let mut ids = taken(&["n1", "n2"]);

        assert_eq!(allocator.allocate(&mut ids).unwrap(), NodeId::new("n3"));
        assert!(ids.contains(&NodeId::new("n3")));
        assert_eq!(allocator.next_counter(), 4);
    }

    #[test]
    fn id_allocator_namespaced_uses_owner_namespace() {
        let namespace = PeerNamespace::try_from("peer-a7").unwrap();
        let mut allocator = NamespacedIdAllocator::new(namespace, 4);
        let mut ids = HashSet::new();

        assert_eq!(
            allocator.allocate(&mut ids).unwrap(),
            NodeId::new("c_peer-a7_4")
        );
        assert_eq!(allocator.namespace().as_str(), "peer-a7");
        assert_eq!(allocator.next_counter(), 5);
    }

    #[test]
    fn id_allocator_namespaced_skips_page_or_node_collision() {
        let namespace = PeerNamespace::try_from("peer").unwrap();
        let mut allocator = NamespacedIdAllocator::new(namespace, 0);
        let mut ids = taken(&["c_peer_0"]);

        assert_eq!(
            allocator.allocate(&mut ids).unwrap(),
            NodeId::new("c_peer_1")
        );
        assert_eq!(allocator.next_counter(), 2);
    }

    #[test]
    fn id_allocator_reports_counter_overflow_without_fallback() {
        let mut sequential = SequentialIdAllocator::new(u64::MAX);
        let namespace = PeerNamespace::try_from("peer").unwrap();
        let mut namespaced = NamespacedIdAllocator::new(namespace, u64::MAX);
        let mut ids = HashSet::new();

        assert_eq!(
            sequential.allocate(&mut ids),
            Err(IdAllocError::CounterExhausted)
        );
        assert_eq!(
            namespaced.allocate(&mut ids),
            Err(IdAllocError::CounterExhausted)
        );
        assert!(ids.is_empty());
    }

    #[test]
    fn owner_and_two_peers_allocate_ten_thousand_nodes_and_pages_without_collision() {
        let owner_namespace = PeerNamespace::try_from("owner").unwrap();
        let first_namespace = PeerNamespace::try_from("peer-a").unwrap();
        let second_namespace = PeerNamespace::try_from("peer-b").unwrap();
        let mut allocators = [
            NamespacedIdAllocator::new(owner_namespace, 0),
            NamespacedIdAllocator::new(first_namespace, 0),
            NamespacedIdAllocator::new(second_namespace, 0),
        ];
        let mut ids = HashSet::new();

        for _ in 0..10_000 {
            for allocator in &mut allocators {
                // Page and node ids intentionally share one collision domain.
                allocator.allocate(&mut ids).unwrap();
                allocator.allocate(&mut ids).unwrap();
            }
        }

        assert_eq!(ids.len(), 60_000);
        assert!(allocators
            .iter()
            .all(|allocator| allocator.next_counter() == 20_000));
    }

    #[test]
    fn document_id_helpers_cover_pages_nodes_and_namespace_high_water() {
        let mut doc = crate::EditorState::new().doc;
        doc.pages = Some(vec![jian_ops_schema::page::PenPage {
            id: "n8".into(),
            name: "Page".into(),
            children: vec![frame(
                "c_peer-a_7",
                "Frame",
                0.0,
                0.0,
                10.0,
                10.0,
                vec![rect("n12", "Child", 0.0, 0.0, 1.0, 1.0)],
            )],
            background_color: None,
            state: None,
            lifecycle: None,
        }]);
        let namespace = PeerNamespace::try_from("peer-a").unwrap();

        let ids = collect_document_ids(&doc);
        assert_eq!(ids.len(), 3);
        assert_eq!(next_sequential_counter(&doc), Ok(13));
        assert_eq!(next_namespaced_counter(&doc, &namespace), Ok(8));

        let mut allocator = DocumentIdAllocator::namespaced_for_document(&doc, namespace).unwrap();
        let mut taken = ids;
        assert_eq!(
            allocator.allocate(&mut taken).unwrap(),
            NodeId::new("c_peer-a_8")
        );
    }

    #[test]
    fn core_creation_paths_share_one_namespaced_allocator() {
        let mut state =
            crate::test_support::state_with(vec![rect("n1", "Original", 0.0, 0.0, 10.0, 10.0)]);
        state.set_single_selection(NodeId::new("n1"));
        let namespace = PeerNamespace::try_from("peer").unwrap();
        let mut allocator = DocumentIdAllocator::namespaced(namespace, 0);

        assert_eq!(
            state
                .duplicate_selected_with_allocator(&mut allocator, 10.0)
                .unwrap(),
            Some(NodeId::new("c_peer_0"))
        );
        assert_eq!(
            state.group_selected_with_allocator(&mut allocator).unwrap(),
            Some(NodeId::new("c_peer_1"))
        );
        assert!(state.copy_selected());
        assert_eq!(
            state
                .paste_clipboard_with_allocator(&mut allocator, 10.0)
                .unwrap()
                .first(),
            Some(&NodeId::new("c_peer_2"))
        );
        assert_eq!(
            state
                .start_pen_path_with_allocator(&mut allocator, (2.0, 3.0))
                .unwrap(),
            Some(NodeId::new("c_peer_4"))
        );
        assert_eq!(
            state
                .add_page_with_allocator(None, None, &mut allocator)
                .unwrap(),
            Some(1)
        );

        let ids = collect_document_ids(&state.doc);
        assert_eq!(ids.len(), 9);
        for counter in 0..8 {
            assert!(ids.contains(&NodeId::new(format!("c_peer_{counter}"))));
        }
        assert_eq!(allocator.next_counter(), 8);
    }

    #[test]
    fn page_creation_never_falls_back_after_sequential_exhaustion() {
        let max_id = format!("n{}", u64::MAX);
        let mut state =
            crate::test_support::state_with(vec![rect(&max_id, "Max", 0.0, 0.0, 10.0, 10.0)]);
        let before = state.doc.clone();

        assert_eq!(state.add_page(), None);
        assert_eq!(state.doc, before);
        assert!(state.doc.pages.is_none());
    }

    #[test]
    fn typed_deep_clone_reports_exhaustion_without_fabricated_id() {
        let node = rect("source", "Source", 0.0, 0.0, 10.0, 10.0);
        let mut next = u64::MAX;
        let mut taken = HashSet::new();

        assert_eq!(
            crate::walkers::try_deep_clone_with_new_ids(&node, &mut next, &mut taken),
            Err(IdAllocError::CounterExhausted)
        );
        assert!(taken.is_empty());
    }

    #[test]
    fn allocator_aware_remap_is_atomic_on_mid_subtree_exhaustion() {
        let mut nodes = vec![frame(
            "root",
            "Root",
            0.0,
            0.0,
            10.0,
            10.0,
            vec![rect("child", "Child", 0.0, 0.0, 1.0, 1.0)],
        )];
        let before = nodes.clone();
        let mut allocator = SequentialIdAllocator::new(u64::MAX - 1);
        let mut taken = HashSet::new();

        assert_eq!(
            crate::command_node::remap_subtree_ids_mapping_with_allocator(
                &mut nodes,
                &mut allocator,
                &mut taken,
            ),
            Err(IdAllocError::CounterExhausted)
        );
        assert_eq!(nodes, before);
    }

    #[test]
    fn allocator_aware_multi_paste_is_atomic_on_late_exhaustion() {
        let mut state =
            crate::test_support::state_with(vec![rect("live", "Live", 0.0, 0.0, 10.0, 10.0)]);
        state.clipboard = vec![
            rect("copy-a", "A", 0.0, 0.0, 1.0, 1.0),
            rect("copy-b", "B", 0.0, 0.0, 1.0, 1.0),
        ];
        state.set_single_selection(NodeId::new("live"));
        let before_doc = state.doc.clone();
        let before_selection = state.selection.clone();
        let mut allocator = SequentialIdAllocator::new(u64::MAX - 1);

        assert_eq!(
            state.paste_clipboard_with_allocator(&mut allocator, 10.0),
            Err(IdAllocError::CounterExhausted)
        );
        assert_eq!(state.doc, before_doc);
        assert_eq!(state.selection, before_selection);
    }

    #[test]
    fn allocator_aware_add_page_rolls_back_legacy_migration_on_late_exhaustion() {
        let mut state =
            crate::test_support::state_with(vec![rect("live", "Live", 0.0, 0.0, 10.0, 10.0)]);
        let before = state.doc.clone();
        let mut allocator = SequentialIdAllocator::new(u64::MAX - 1);

        assert_eq!(
            state.add_page_with_allocator(None, None, &mut allocator),
            Err(IdAllocError::CounterExhausted)
        );
        assert_eq!(state.doc, before);
        assert!(state.doc.pages.is_none());
    }

    #[test]
    fn legacy_group_noop_does_not_advance_the_caller_cursor() {
        let mut state =
            crate::test_support::state_with(vec![rect("n10", "Live", 0.0, 0.0, 10.0, 10.0)]);
        state.selection = crate::SelectionState {
            anchor: NodeId::new("missing"),
            set: vec![NodeId::new("missing")],
        };
        let mut next = 1;

        assert_eq!(state.group_selected(&mut next), None);
        assert_eq!(next, 1);
    }
}
