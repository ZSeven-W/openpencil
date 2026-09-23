//! Layout + hit-test unit tests for [`super::AIChatPlaceholder`].
//! Split into a sibling file to keep `ai_chat_panel.rs` under the
//! 800-line cap; the cases themselves are split again across
//! `tests/` for the same reason.
//!
//! | File                      | Coverage                             |
//! | ------------------------- | ------------------------------------ |
//! | `tests/support.rs`        | fixtures + recording `RenderBackend` |
//! | `tests/layout.rs`         | layout + `from_editor` snapshots     |
//! | `tests/hit_test.rs`       | hit-test / resize-edge coverage      |
//! | `tests/footer_layout.rs`  | bottom toolbar + agents picker       |

// `ai_chat_panel.rs` declares this module with an explicit `#[path]`, so
// the child modules need explicit paths too (rustc otherwise resolves
// them against `ai_chat_panel/` instead of `ai_chat_panel/tests/`).
#[path = "tests/footer_image_gen.rs"]
mod footer_image_gen;
#[path = "tests/footer_layout.rs"]
mod footer_layout;
#[path = "tests/hit_test.rs"]
mod hit_test;
#[path = "tests/layout.rs"]
mod layout;
#[path = "tests/support.rs"]
mod support;

// Re-exported so the `tests_paint` / `tests_transcript` siblings keep
// reaching the shared fixtures through `super::tests::…`.
pub(in crate::widgets) use support::*;
