//! Shared image-slot enrichment primitives.
//!
//! Two consumers need the exact same slot-detection and write-back semantics:
//!
//! - the desktop host's background `ImageSearchSession` (which resolves slots
//!   on the live canvas as searches complete), and
//! - the headless MCP `enrich_images` tool in `op-host-services` (the
//!   in-process version of `openpencil-desktop --enrich-images`).
//!
//! Both were originally born in `op-host-desktop/src/image_search_session`;
//! they moved here as pure code motion so the dependency direction stays
//! legal (`op-host-services` must never depend on `op-host-desktop`). The
//! desktop session keeps its own provider fetches, memoization, and job
//! bookkeeping and re-exports these primitives under their original paths.
//!
//! `targets` owns the predicate vocabulary: which nodes want an image, what
//! query describes them, and whether the slot's acquisition mode is stock
//! search or AI generation. `apply` owns the write-back: how a resolved url
//! lands on an image node, a placeholder frame, or an empty image-fill
//! container, gated by the collaboration external-assets rule.

mod apply;
#[cfg(feature = "net")]
pub mod net;
mod targets;

pub use apply::{apply_result, collaboration_image_result_gate, SEARCH_FAILED_PLACEHOLDER_SRC};
pub use targets::{
    collect_targets, collect_targets_with_scene, has_empty_image_fill, image_request_mode,
    is_frame_placeholder_still_unfilled, is_image_area_rectangle_by_heuristic, ImageAspectRatio,
    ImageRequestMode, ImageSearchTarget,
};
