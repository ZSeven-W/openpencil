//! State for the image-node property section's Search / Generate
//! popovers and the local-asset warning (TS `image-section.tsx` +
//! `image-search-popover.tsx` + `image-generate-popover.tsx` +
//! `use-image-asset-state.ts`).
//!
//! The widget layer is platform-free: it only paints from this state
//! and raises intents (`search_epoch` / `generate_epoch` bumps). The
//! desktop host pumps the actual Openverse / Wikimedia / provider
//! requests and writes results back here.

use std::sync::Arc;

/// Which catalogue served the current search results (TS
/// `ImageSearchResponse.source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSearchSource {
    Openverse,
    Wikimedia,
}

/// One search hit ready for panel paint. The host downloads the
/// thumbnail into a `data:` URL so the wasm-clean widget layer can
/// paint it without IO; `Arc` keeps the per-frame panel snapshot
/// clone cheap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSearchHit {
    pub id: String,
    /// Thumbnail as a `data:` URL (also the value written to
    /// `ImageNode.src` on select — TS writes `result.thumbUrl`).
    pub thumb_data_url: Arc<String>,
    /// Attribution / license line used as the cell tooltip in TS.
    pub attribution: String,
}

/// Generate-popover phase (TS `type State = 'idle' | 'loading' |
/// 'preview' | 'error'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageGeneratePhase {
    #[default]
    Idle,
    Loading,
    Preview,
    Error,
}

/// Host-checked status of the selected image node's `src` asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAssetStatus {
    /// Remote / data URL, or a local file that exists.
    Ok,
    /// Relative path with no document path to resolve against
    /// (TS "Relative image path cannot be resolved yet").
    Unresolved,
    /// Local path that doesn't exist on disk (TS "Image file is
    /// missing").
    Missing,
}

/// One asset check result, keyed by the `(node id, src)` pair it was
/// computed for so the host re-checks only when either changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAssetCheck {
    pub node_id: String,
    pub src: String,
    pub status: ImageAssetStatus,
}

/// All image-node panel popover state. Lives on `EditorUiState`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImagePanelState {
    /// Whether the Search popover is open.
    pub search_open: bool,
    /// Live search-box draft (seeded from `imageSearchQuery ?? name`
    /// on open, mirrors TS `initialQuery`).
    pub search_query: String,
    /// True while a search request is in flight.
    pub search_loading: bool,
    /// Bumped on every search submit; the host spawns a job when it
    /// sees an epoch newer than the one it last spawned.
    pub search_epoch: u64,
    /// Results of the last completed search.
    pub search_results: Vec<ImageSearchHit>,
    pub search_source: Option<ImageSearchSource>,
    /// Whether a search ran since the popover opened (drives the
    /// "No results found" vs "Search for images" empty state).
    pub search_has_searched: bool,

    /// Whether the Generate popover is open.
    pub generate_open: bool,
    /// Prompt draft (seeded from `imagePrompt ?? name` on open).
    pub generate_prompt: String,
    pub generate_phase: ImageGeneratePhase,
    /// Bumped on every generate submit (same epoch contract as
    /// `search_epoch`).
    pub generate_epoch: u64,
    /// Generated image as a `data:` URL while in `Preview`.
    pub generate_preview: Option<Arc<String>>,
    /// Error detail while in `Error` (TS truncates to 200 chars).
    pub generate_error: String,

    /// Host-computed local-asset status for the selected image node.
    pub asset_check: Option<ImageAssetCheck>,
}

impl ImagePanelState {
    /// Close both popovers and drop transient search/generate state.
    /// Called when the pickers should collapse (Escape, selection
    /// change, other property popovers opening).
    pub fn close_popovers(&mut self) {
        self.search_open = false;
        self.search_loading = false;
        self.search_results.clear();
        self.search_source = None;
        self.search_has_searched = false;
        self.generate_open = false;
        self.generate_phase = ImageGeneratePhase::Idle;
        self.generate_preview = None;
        self.generate_error.clear();
    }

    /// The asset status for `(node_id, src)` if the host already
    /// checked exactly that pair, else `None` (treated as `Ok` by
    /// the warning row so it never flashes on stale data).
    pub fn status_for(&self, node_id: &str, src: &str) -> Option<ImageAssetStatus> {
        self.asset_check
            .as_ref()
            .filter(|c| c.node_id == node_id && c.src == src)
            .map(|c| c.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_popovers_resets_transients() {
        let mut s = ImagePanelState {
            search_open: true,
            search_loading: true,
            search_has_searched: true,
            generate_open: true,
            generate_phase: ImageGeneratePhase::Error,
            generate_error: "boom".into(),
            ..Default::default()
        };
        s.search_results.push(ImageSearchHit {
            id: "1".into(),
            thumb_data_url: Arc::new("data:image/png;base64,AA==".into()),
            attribution: String::new(),
        });
        s.close_popovers();
        assert!(!s.search_open && !s.generate_open);
        assert!(s.search_results.is_empty());
        assert_eq!(s.generate_phase, ImageGeneratePhase::Idle);
        assert!(s.generate_error.is_empty());
    }

    #[test]
    fn status_for_requires_exact_node_and_src_match() {
        let s = ImagePanelState {
            asset_check: Some(ImageAssetCheck {
                node_id: "n1".into(),
                src: "./a.png".into(),
                status: ImageAssetStatus::Missing,
            }),
            ..Default::default()
        };
        assert_eq!(
            s.status_for("n1", "./a.png"),
            Some(ImageAssetStatus::Missing)
        );
        assert_eq!(s.status_for("n1", "./b.png"), None);
        assert_eq!(s.status_for("n2", "./a.png"), None);
    }
}
