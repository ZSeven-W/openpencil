//! Typed failure domains for entering / re-solving a preview session.
//!
//! Split out of `preview/mod.rs` (already near the repo's 800-line ceiling)
//! and `preview/app_mode.rs`. Both enums' `Display` impls reproduce the exact
//! sentences these paths used to return as `String`, because the host renders
//! them into `EditorState.editor_ui.preview.warnings` (`"preview: {message}"`
//! / `"preview: relayout failed: {e}"`), which is user-visible chrome.

/// Why the per-root layout solve for a preview runtime failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewLayoutError {
    /// The runtime carried no document when the solve started.
    NoDocument,
    /// `LayoutEngine::build` rejected the runtime's node tree. Payload is the
    /// rendered taffy/jian error.
    BuildTree(String),
    /// `build` succeeded but the runtime's document was gone afterwards —
    /// surfaced instead of panicking, to keep the no-panic contract.
    DocumentVanished,
    /// `LayoutEngine::compute` failed for one page-root. Payload is the
    /// rendered taffy/jian error.
    Compute(String),
}

impl std::fmt::Display for PreviewLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewLayoutError::NoDocument => f.write_str("preview runtime has no document"),
            PreviewLayoutError::BuildTree(error) => write!(f, "build layout tree: {error}"),
            PreviewLayoutError::DocumentVanished => {
                f.write_str("preview runtime document vanished after layout build")
            }
            PreviewLayoutError::Compute(error) => write!(f, "compute layout: {error}"),
        }
    }
}

impl std::error::Error for PreviewLayoutError {}

/// Why `PreviewSession::enter` declined to build a preview session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEnterError {
    /// Serializing the prepared document to canonical JSON failed.
    Serialize(String),
    /// Re-parsing that JSON through the widget-promoting compat loader failed.
    Parse(String),
    /// App mode was projected but the projected document carried no routes.
    LostRoutes,
    /// `Runtime::new_from_document` rejected the promoted document.
    BuildRuntime(String),
    /// The initial per-root layout solve failed.
    Layout(PreviewLayoutError),
}

impl std::fmt::Display for PreviewEnterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewEnterError::Serialize(error) => write!(f, "serialize document: {error}"),
            PreviewEnterError::Parse(error) => write!(f, "parse document for preview: {error}"),
            PreviewEnterError::LostRoutes => f.write_str("projected document lost its routes"),
            PreviewEnterError::BuildRuntime(error) => write!(f, "build runtime: {error}"),
            PreviewEnterError::Layout(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PreviewEnterError {}

impl From<PreviewLayoutError> for PreviewEnterError {
    fn from(error: PreviewLayoutError) -> Self {
        PreviewEnterError::Layout(error)
    }
}
