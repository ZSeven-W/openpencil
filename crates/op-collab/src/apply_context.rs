use crate::{
    PeerNamespace, Role, WireLimits, MAX_DOCUMENT_NODES, MAX_IDENTIFIER_BYTES, MAX_OPS_PER_TXN,
    MAX_PROCESSED_SUBTREE_NODES_PER_OP, MAX_TREE_DEPTH, MAX_VALIDATION_NODE_VISITS_PER_TXN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyLimits {
    pub max_ops_per_txn: usize,
    pub max_processed_subtree_nodes_per_op: usize,
    pub max_document_nodes: usize,
    pub max_tree_depth: usize,
    pub max_identifier_bytes: usize,
    pub max_validation_node_visits_per_txn: usize,
}

impl Default for ApplyLimits {
    fn default() -> Self {
        Self {
            max_ops_per_txn: usize_from_u32(MAX_OPS_PER_TXN),
            max_processed_subtree_nodes_per_op: usize_from_u32(MAX_PROCESSED_SUBTREE_NODES_PER_OP),
            max_document_nodes: usize_from_u32(MAX_DOCUMENT_NODES),
            max_tree_depth: usize_from_u32(MAX_TREE_DEPTH),
            max_identifier_bytes: usize_from_u32(MAX_IDENTIFIER_BYTES),
            max_validation_node_visits_per_txn: usize_from_u32(MAX_VALIDATION_NODE_VISITS_PER_TXN),
        }
    }
}

impl From<WireLimits> for ApplyLimits {
    fn from(limits: WireLimits) -> Self {
        Self {
            max_ops_per_txn: usize_from_u32(limits.max_ops_per_txn),
            max_processed_subtree_nodes_per_op: usize_from_u32(
                limits.max_processed_subtree_nodes_per_op,
            ),
            max_document_nodes: usize_from_u32(limits.max_document_nodes),
            max_tree_depth: usize_from_u32(limits.max_tree_depth),
            max_identifier_bytes: usize_from_u32(limits.max_identifier_bytes),
            max_validation_node_visits_per_txn: usize_from_u32(
                limits.max_validation_node_visits_per_txn,
            ),
        }
    }
}

/// Policy for ids introduced by `InsertExact` or `ReplaceExact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertPolicy {
    /// Used only outside an active collaboration session.
    StandaloneTrusted,
    /// Require every newly authored id to belong to this namespace and to
    /// advance its retained monotonic counter.
    RequireNamespace {
        namespace: PeerNamespace,
        next_counter: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyContext {
    pub role: Role,
    pub insert_policy: InsertPolicy,
    pub limits: ApplyLimits,
}

impl ApplyContext {
    pub fn standalone_trusted() -> Self {
        Self {
            role: Role::Owner,
            insert_policy: InsertPolicy::StandaloneTrusted,
            limits: ApplyLimits::default(),
        }
    }

    pub fn for_peer_namespace(namespace: PeerNamespace, role: Role) -> Self {
        Self::for_peer_namespace_at(namespace, role, Some(0))
    }

    pub fn for_peer_namespace_at(
        namespace: PeerNamespace,
        role: Role,
        next_counter: Option<u64>,
    ) -> Self {
        Self {
            role,
            insert_policy: InsertPolicy::RequireNamespace {
                namespace,
                next_counter,
            },
            limits: ApplyLimits::default(),
        }
    }
}

fn usize_from_u32(value: u32) -> usize {
    usize::try_from(value).expect("u32 limits fit every supported OpenPencil target")
}
