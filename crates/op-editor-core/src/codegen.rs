//! UI-facing code-generation state, read by the Code panel painter and
//! shared by both hosts. Mirrors the TS `CodeGenProgress` shapes. Plain
//! data — keeps op-editor-core wasm-clean. Pipeline LOGIC lives in
//! op-codegen::ai; these are the types that crate returns (it depends on
//! op-editor-core, so the edge is acyclic).

/// Target framework for code generation. Wire tokens match TS `Framework`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    React,
    Vue,
    Svelte,
    Html,
    Flutter,
    SwiftUi,
    Compose,
    ReactNative,
}

impl Framework {
    pub const ALL: [Framework; 8] = [
        Framework::React,
        Framework::Vue,
        Framework::Svelte,
        Framework::Html,
        Framework::Flutter,
        Framework::SwiftUi,
        Framework::Compose,
        Framework::ReactNative,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            Framework::React => "react",
            Framework::Vue => "vue",
            Framework::Svelte => "svelte",
            Framework::Html => "html",
            Framework::Flutter => "flutter",
            Framework::SwiftUi => "swiftui",
            Framework::Compose => "compose",
            Framework::ReactNative => "react-native",
        }
    }

    pub fn from_wire(s: &str) -> Option<Framework> {
        Framework::ALL.into_iter().find(|f| f.as_wire() == s)
    }

    /// Human display name (capitalized), for UI labels. TS parity.
    pub fn display_name(self) -> &'static str {
        match self {
            Framework::React => "React",
            Framework::Vue => "Vue",
            Framework::Svelte => "Svelte",
            Framework::Html => "HTML",
            Framework::Flutter => "Flutter",
            Framework::SwiftUi => "SwiftUI",
            Framework::Compose => "Compose",
            Framework::ReactNative => "React Native",
        }
    }

    /// The framework-specific knowledge skill name (e.g. "codegen-react").
    pub fn skill_name(self) -> &'static str {
        match self {
            Framework::React => "codegen-react",
            Framework::Vue => "codegen-vue",
            Framework::Svelte => "codegen-svelte",
            Framework::Html => "codegen-html",
            Framework::Flutter => "codegen-flutter",
            Framework::SwiftUi => "codegen-swiftui",
            Framework::Compose => "codegen-compose",
            Framework::ReactNative => "codegen-react-native",
        }
    }
}

/// Per-chunk status. Parity with TS `ChunkStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkStatus {
    Pending,
    Running,
    Done,
    Degraded,
    Failed,
    Skipped,
}

/// Top-level phase the panel renders. `Idle` = empty state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenPhase {
    Idle,
    Generating,
    Complete,
    Error,
}

/// Byte-offset text selection inside the generated code preview.
/// Offsets are clamped by the painter/hit-test against the currently
/// visible code text, so stale ranges after regeneration are harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl CodeSelection {
    pub fn ordered(self) -> (usize, usize) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }
}

/// Code panel non-framework hover target. Framework chips keep their own
/// `framework_hover` because their state carries a selected framework value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenHover {
    Generate,
    Regenerate,
    Cancel,
    Copy,
    Download,
    ExportBundle,
    ScrollFrameworksLeft,
    ScrollFrameworksRight,
}

/// One chunk's progress row for the panel (id + display name + status).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkProgress {
    pub chunk_id: String,
    pub name: String,
    pub status: ChunkStatus,
}

/// Progress snapshot the pipeline produces and the panel paints. Parity
/// with the union TS `CodeGenProgress`, flattened into one struct so the
/// painter can render all three phase groups from a single value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeGenProgress {
    /// planning: None = not started, Some(false) = running, Some(true) = done.
    pub planning_done: Option<bool>,
    pub chunks: Vec<ChunkProgress>,
    /// assembly: None = not started, Some(false) = running, Some(true) = done.
    pub assembly_done: Option<bool>,
}

/// Lightweight asset descriptor for the "includes assets" notice. The
/// raw bytes live in op-codegen's `AssetFile`; the host maps them here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMeta {
    pub relative_path: String,
    pub byte_len: usize,
}

/// The Code panel's full state. Mirror of `ChatState`'s role for chat.
/// `PartialEq` only (not `Eq`) — `framework_scroll` carries an `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenState {
    pub framework: Framework,
    /// Horizontal scroll offset (px, ≥ 0) of the framework tab strip, so the
    /// single-row selector scrolls to reach off-screen frameworks (TS parity).
    pub framework_scroll: f32,
    /// The inactive framework chip the cursor is hovering, for a subtle
    /// background highlight. `None` when the cursor is off the strip.
    pub framework_hover: Option<Framework>,
    /// Non-framework button / chevron the cursor is hovering.
    pub action_hover: Option<CodegenHover>,
    pub phase: CodegenPhase,
    pub progress: CodeGenProgress,
    pub code: String,
    /// Vertical scroll offset (px, >= 0) inside the generated-code preview.
    pub code_scroll: f32,
    /// Text selection inside the generated-code preview.
    pub code_selection: Option<CodeSelection>,
    pub degraded: bool,
    pub assets: Vec<AssetMeta>,
    /// Node ids the last generation ran against — to detect selection drift.
    pub selection_snapshot: Vec<String>,
    pub error: Option<String>,
    /// Frame at which "Copied" was shown, to time the transient label.
    pub copied_at: Option<u64>,
    /// Set by the Code panel's Generate action; drained by the host codegen
    /// session (P3). Mirror of `chat.pending_send`.
    pub pending_generate: bool,
    /// Set by Regenerate; drained by the host codegen session (P3).
    pub pending_regenerate: bool,
    /// Set by the Code panel's Download action; drained by the desktop
    /// codegen-export drain (Task 5) which pops a save dialog + writes the
    /// generated code (single file, or a .zip when there are image assets).
    pub pending_download: bool,
    /// Set by Export AI Bundle; drained by the desktop codegen-export drain
    /// which writes a structure-bundle .zip.
    pub pending_export_bundle: bool,
    /// Set by the Code panel's Cancel action; drained by the desktop
    /// codegen-session cancel drain, which raises the in-flight worker's
    /// shared abort flag so the run actually stops (TS parity:
    /// `abortRef.current?.abort()`), not just the painted phase.
    pub pending_cancel: bool,
}

impl Default for CodegenState {
    fn default() -> Self {
        Self {
            framework: Framework::React,
            framework_scroll: 0.0,
            framework_hover: None,
            action_hover: None,
            phase: CodegenPhase::Idle,
            progress: CodeGenProgress::default(),
            code: String::new(),
            code_scroll: 0.0,
            code_selection: None,
            degraded: false,
            assets: Vec::new(),
            selection_snapshot: Vec::new(),
            error: None,
            copied_at: None,
            pending_generate: false,
            pending_regenerate: false,
            pending_download: false,
            pending_export_bundle: false,
            pending_cancel: false,
        }
    }
}

impl CodegenState {
    pub fn selected_code_text(&self) -> Option<&str> {
        let selection = self.code_selection?;
        if selection.is_collapsed() || self.code.is_empty() {
            return None;
        }
        let (start, end) = selection.ordered();
        let start = previous_char_boundary(&self.code, start.min(self.code.len()));
        let end = previous_char_boundary(&self.code, end.min(self.code.len()));
        if start >= end {
            return None;
        }
        Some(&self.code[start..end])
    }
}

fn previous_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_round_trips_its_wire_token() {
        for fw in Framework::ALL {
            assert_eq!(Framework::from_wire(fw.as_wire()), Some(fw));
        }
        assert_eq!(Framework::from_wire("react"), Some(Framework::React));
        assert_eq!(
            Framework::from_wire("react-native"),
            Some(Framework::ReactNative)
        );
        assert_eq!(Framework::from_wire("nope"), None);
        // display_name is the capitalized UI label (TS parity).
        assert_eq!(Framework::React.display_name(), "React");
        assert_eq!(Framework::ReactNative.display_name(), "React Native");
        assert_eq!(Framework::SwiftUi.display_name(), "SwiftUI");
    }

    #[test]
    fn codegen_state_defaults_to_idle_react() {
        let s = CodegenState::default();
        assert_eq!(s.framework, Framework::React);
        assert_eq!(s.phase, CodegenPhase::Idle);
        assert!(s.code.is_empty());
        assert_eq!(s.code_scroll, 0.0);
        assert!(!s.degraded);
        assert!(s.error.is_none());
        assert!(!s.pending_generate);
        assert!(!s.pending_regenerate);
        assert!(!s.pending_download);
        assert!(!s.pending_export_bundle);
        assert!(!s.pending_cancel);
    }

    #[test]
    fn selected_code_text_returns_non_collapsed_range() {
        let s = CodegenState {
            code: "import React\nconst n = 1".into(),
            code_selection: Some(CodeSelection {
                anchor: 0,
                focus: 6,
            }),
            ..CodegenState::default()
        };

        assert_eq!(s.selected_code_text(), Some("import"));
    }
}
