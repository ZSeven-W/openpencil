//! Raw-node command application — `InsertNode` / `UpdateNode` /
//! `DeleteNode` / `MoveNode` / `CopyNode` / `ReplaceNode` /
//! `BatchInsert`.
//!
//! shell-core's `mcp_apply.rs` built its flat `Node` struct directly;
//! `op-editor-core` operates on the canonical `jian_ops_schema::PenNode`
//! tree, so this module ports the build / find / detach / replace
//! helpers onto `PenNode`. Carved off `command_apply.rs` to keep both
//! files under the 800-line cap.
//!
//! Every helper preserves the pre-validate-then-mutate discipline: a
//! caller validates kind / geometry / hex / id space BEFORE any tree
//! write, so a bad arg never leaves the document half-mutated.
//!
//! This file is the slim spine; the implementation lives in sibling
//! modules under `command_node/` and is re-exported here so every
//! `crate::command_node::…` import path keeps working:
//!
//! | File                     | Purpose                                            |
//! | ------------------------ | -------------------------------------------------- |
//! | `command_node/builders.rs`  | `build_leaf_node` / widget builders / kind table |
//! | `command_node/tree_ops.rs`  | slot replace / parent insert / id remap         |
//! | `command_node/state_ops.rs` | `impl EditorState` command methods             |

mod builders;
mod state_ops;
mod tree_ops;

pub use builders::{build_leaf_node, kind_is_valid, WIDGET_KINDS};
pub(crate) use tree_ops::remap_subtree_ids_with_allocator;
pub use tree_ops::{
    remap_subtree_ids, remap_subtree_ids_mapping, remap_subtree_ids_mapping_with_allocator,
    replace_node_in_children,
};
