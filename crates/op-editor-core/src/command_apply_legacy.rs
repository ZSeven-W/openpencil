//! Standalone compatibility wrapper for command application.

use crate::{DocumentIdAllocator, EditorCommand, EditorState};

impl EditorState {
    /// Apply one command with the historical sequential document id policy.
    ///
    /// Returns `false` on validation or id exhaustion. Collaboration hosts
    /// must call [`Self::apply_with_allocator`] with their session allocator.
    pub fn apply(&mut self, command: EditorCommand) -> bool {
        let Ok(mut allocator) = DocumentIdAllocator::sequential_for_document(&self.doc) else {
            return false;
        };
        self.apply_with_allocator(command, &mut allocator)
            .unwrap_or(false)
    }
}
