//! UI-facing code-generation state, read by the Code panel painter and
//! shared by both hosts. Mirrors the TS `CodeGenProgress` shapes. Plain
//! data — keeps op-editor-core wasm-clean. Pipeline LOGIC lives in
//! op-codegen::ai; these are the types that crate returns (it depends on
//! op-editor-core, so the edge is acyclic).

use std::sync::Arc;

use jian_core::text_input::prev_char_boundary;

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

/// A completed framework's generated artifacts while another framework is
/// active. The active framework keeps using the flat fields on
/// [`CodegenState`], so painters and host pipelines do not need a second
/// lookup; switching tabs snapshots/restores these persistent result fields.
#[derive(Debug, PartialEq)]
pub struct CodegenCacheEntry {
    pub framework: Framework,
    pub code: String,
    pub code_scroll: jian_core::scroll::ScrollState,
    pub code_selection: Option<CodeSelection>,
    pub degraded: bool,
    pub assets: Vec<AssetMeta>,
    pub selection_snapshot: Vec<String>,
}

/// The Code panel's full state. Mirror of `ChatState`'s role for chat.
/// `PartialEq` only (not `Eq`) — scroll offsets carry `f32` values.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenState {
    /// Monotonic runtime lifetime of document-scoped codegen state.
    ///
    /// This is intentionally not part of the `.op` document. Every document
    /// install/reset advances it, including collaboration RemoteCommit,
    /// Replay, and rollback paths that deliberately preserve the editor's
    /// broader document generation.
    #[doc(hidden)]
    pub document_reset_epoch: u64,
    pub framework: Framework,
    /// Generated results keyed by framework. This mirrors the retired web
    /// panel's `codeCache` and lets a generated tab survive a round trip
    /// through another framework without ever relabelling its code. The two
    /// Arc layers keep both per-frame PropertyPanel snapshots and the
    /// copy-on-write framework switch shallow.
    pub framework_cache: Arc<Vec<Arc<CodegenCacheEntry>>>,
    /// Horizontal scroll offset (px, ≥ 0) of the framework tab strip, so the
    /// single-row selector scrolls to reach off-screen frameworks (TS parity).
    pub framework_scroll: jian_core::scroll::ScrollState,
    /// The inactive framework chip the cursor is hovering, for a subtle
    /// background highlight. `None` when the cursor is off the strip.
    pub framework_hover: Option<Framework>,
    /// Non-framework button / chevron the cursor is hovering.
    pub action_hover: Option<CodegenHover>,
    pub phase: CodegenPhase,
    pub progress: CodeGenProgress,
    pub code: String,
    /// Vertical scroll offset (px, >= 0) inside the generated-code preview.
    pub code_scroll: jian_core::scroll::ScrollState,
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
            document_reset_epoch: 0,
            framework: Framework::React,
            framework_cache: Arc::new(Vec::new()),
            framework_scroll: Default::default(),
            framework_hover: None,
            action_hover: None,
            phase: CodegenPhase::Idle,
            progress: CodeGenProgress::default(),
            code: String::new(),
            code_scroll: Default::default(),
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
    /// Drop every artifact and in-flight action derived from the document that
    /// is being replaced, while retaining the user's framework-tab choice.
    ///
    /// `EditorState::replace_document*` preserves application chrome, so this
    /// reset must be explicit: generated code and its per-framework cache are
    /// document-scoped and must never survive Open/New/import/live-sync.
    pub fn reset_for_document_replacement(&mut self) {
        let document_reset_epoch = self.document_reset_epoch.saturating_add(1);
        let framework = self.framework;
        let framework_scroll = self.framework_scroll;
        *self = Self {
            document_reset_epoch,
            framework,
            framework_scroll,
            ..Self::default()
        };
    }

    /// Runtime lifetime of the document-scoped generation/cache state.
    pub fn document_reset_epoch(&self) -> u64 {
        self.document_reset_epoch
    }

    /// Select a different output framework, caching the current completed
    /// result and restoring any result previously generated for the target.
    /// A never-generated target still opens in the empty state, so code is
    /// never shown, copied, or exported under the wrong framework.
    ///
    /// The UI disables framework tabs while generation is active. Keeping the
    /// same guard here prevents a synthetic/stale action from relabelling an
    /// in-flight run whose completion still targets the original framework.
    pub fn select_framework(&mut self, framework: Framework) -> bool {
        if framework == self.framework || self.phase == CodegenPhase::Generating {
            return false;
        }

        self.cache_active_result();
        let cached = self
            .framework_cache
            .iter()
            .find(|entry| entry.framework == framework)
            .cloned();

        self.framework = framework;
        self.framework_hover = None;
        self.action_hover = None;
        self.phase = CodegenPhase::Idle;
        self.progress = CodeGenProgress::default();
        self.code.clear();
        self.code_scroll = Default::default();
        self.code_selection = None;
        self.degraded = false;
        self.assets.clear();
        self.selection_snapshot.clear();
        self.error = None;
        self.copied_at = None;
        self.pending_generate = false;
        self.pending_regenerate = false;
        self.pending_download = false;
        self.pending_export_bundle = false;
        self.pending_cancel = false;

        if let Some(cached) = cached {
            self.phase = CodegenPhase::Complete;
            self.code = cached.code.clone();
            self.code_scroll = cached.code_scroll;
            self.code_selection = cached.code_selection;
            self.degraded = cached.degraded;
            self.assets = cached.assets.clone();
            self.selection_snapshot = cached.selection_snapshot.clone();
        }
        true
    }

    fn cache_active_result(&mut self) {
        // An Error state can still carry the last successful output after a
        // failed regeneration. Preserve that output just like the retired
        // web panel's per-framework codeCache; a bare error has no result to
        // retain.
        if self.phase != CodegenPhase::Complete && self.code.is_empty() {
            return;
        }

        let entry = Arc::new(CodegenCacheEntry {
            framework: self.framework,
            code: self.code.clone(),
            code_scroll: self.code_scroll,
            code_selection: self.code_selection,
            degraded: self.degraded,
            assets: self.assets.clone(),
            selection_snapshot: self.selection_snapshot.clone(),
        });
        let cache = Arc::make_mut(&mut self.framework_cache);
        if let Some(cached) = cache
            .iter_mut()
            .find(|cached| cached.framework == self.framework)
        {
            *cached = entry;
        } else {
            cache.push(entry);
        }
    }

    pub fn selected_code_text(&self) -> Option<&str> {
        let selection = self.code_selection?;
        if selection.is_collapsed() || self.code.is_empty() {
            return None;
        }
        let (start, end) = selection.ordered();
        let start = prev_char_boundary(&self.code, start.min(self.code.len()));
        let end = prev_char_boundary(&self.code, end.min(self.code.len()));
        if start >= end {
            return None;
        }
        Some(&self.code[start..end])
    }
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
        assert_eq!(s.code_scroll.offset, 0.0);
        assert!(!s.degraded);
        assert!(s.error.is_none());
        assert!(!s.pending_generate);
        assert!(!s.pending_regenerate);
        assert!(!s.pending_download);
        assert!(!s.pending_export_bundle);
        assert!(!s.pending_cancel);
    }

    #[test]
    fn codegen_scroll_fields_use_scroll_state() {
        let mut s = CodegenState::default();

        s.framework_scroll.offset = 16.0;
        s.code_scroll.offset = 32.0;

        assert_eq!(s.framework_scroll.offset, 16.0);
        assert_eq!(s.code_scroll.offset, 32.0);
    }

    #[test]
    fn document_reset_epoch_is_monotonic_and_preserves_framework_chrome() {
        let mut state = CodegenState {
            framework: Framework::Compose,
            framework_scroll: jian_core::scroll::ScrollState { offset: 27.0 },
            phase: CodegenPhase::Complete,
            code: "old output".into(),
            ..CodegenState::default()
        };
        let initial = state.document_reset_epoch();

        state.reset_for_document_replacement();
        let first_reset = state.document_reset_epoch();
        assert!(first_reset > initial);
        assert_eq!(state.framework, Framework::Compose);
        assert_eq!(state.framework_scroll.offset, 27.0);
        assert_eq!(state.phase, CodegenPhase::Idle);
        assert!(state.code.is_empty());

        let mut cloned = state.clone();
        assert_eq!(cloned.document_reset_epoch(), first_reset);
        cloned.reset_for_document_replacement();
        assert!(cloned.document_reset_epoch() > first_reset);
        assert_eq!(state.document_reset_epoch(), first_reset);
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

    #[test]
    fn failed_regeneration_caches_the_previous_successful_targets() {
        let mut s = CodegenState {
            phase: CodegenPhase::Complete,
            code: "previous successful output".into(),
            selection_snapshot: vec!["old-node".into()],
            ..CodegenState::default()
        };

        // Hosts keep the new run's targets on the session until Done. An
        // Error with previous code therefore still carries the successful
        // targets and must cache/restore them as one coherent result.
        s.phase = CodegenPhase::Error;
        s.error = Some("regeneration failed".into());
        assert!(s.select_framework(Framework::Vue));
        assert!(s.select_framework(Framework::React));
        assert_eq!(s.code, "previous successful output");
        assert_eq!(s.selection_snapshot, ["old-node"]);
    }

    #[test]
    fn selecting_a_never_generated_framework_shows_empty_state() {
        let mut s = CodegenState {
            framework_scroll: jian_core::scroll::ScrollState { offset: 18.0 },
            framework_hover: Some(Framework::Vue),
            action_hover: Some(CodegenHover::Copy),
            phase: CodegenPhase::Error,
            progress: CodeGenProgress {
                planning_done: Some(true),
                chunks: vec![ChunkProgress {
                    chunk_id: "hero".into(),
                    name: "Hero".into(),
                    status: ChunkStatus::Failed,
                }],
                assembly_done: Some(false),
            },
            code: "export const App = () => null".into(),
            code_scroll: jian_core::scroll::ScrollState { offset: 42.0 },
            code_selection: Some(CodeSelection {
                anchor: 0,
                focus: 6,
            }),
            degraded: true,
            assets: vec![AssetMeta {
                relative_path: "assets/hero.png".into(),
                byte_len: 128,
            }],
            selection_snapshot: vec!["hero".into()],
            error: Some("chunk failed".into()),
            copied_at: Some(99),
            pending_download: true,
            pending_export_bundle: true,
            ..CodegenState::default()
        };

        assert!(s.select_framework(Framework::Vue));
        assert_eq!(s.framework, Framework::Vue);
        assert_eq!(s.framework_scroll.offset, 18.0);
        assert_eq!(s.phase, CodegenPhase::Idle);
        assert_eq!(s.progress, CodeGenProgress::default());
        assert!(s.code.is_empty());
        assert_eq!(s.code_scroll.offset, 0.0);
        assert!(s.code_selection.is_none());
        assert!(!s.degraded);
        assert!(s.assets.is_empty());
        assert!(s.selection_snapshot.is_empty());
        assert!(s.error.is_none());
        assert!(s.copied_at.is_none());
        assert!(!s.pending_download);
        assert!(!s.pending_export_bundle);
    }

    #[test]
    fn generated_html_survives_switching_away_and_back() {
        let mut s = CodegenState {
            framework: Framework::Html,
            phase: CodegenPhase::Complete,
            code: "<!doctype html><main>Hello</main>".into(),
            code_scroll: jian_core::scroll::ScrollState { offset: 24.0 },
            code_selection: Some(CodeSelection {
                anchor: 16,
                focus: 20,
            }),
            degraded: true,
            assets: vec![AssetMeta {
                relative_path: "assets/hero.png".into(),
                byte_len: 128,
            }],
            selection_snapshot: vec!["hero".into()],
            ..CodegenState::default()
        };

        assert!(s.select_framework(Framework::Vue));
        assert_eq!(s.phase, CodegenPhase::Idle);
        assert!(s.code.is_empty());

        s.phase = CodegenPhase::Complete;
        s.code = "<template>Vue result</template>".into();
        assert!(s.select_framework(Framework::Html));
        assert_eq!(s.phase, CodegenPhase::Complete);
        assert_eq!(s.code, "<!doctype html><main>Hello</main>");
        assert_eq!(s.code_scroll.offset, 24.0);
        assert_eq!(
            s.code_selection,
            Some(CodeSelection {
                anchor: 16,
                focus: 20
            })
        );
        assert!(s.degraded);
        assert_eq!(s.assets.len(), 1);
        assert_eq!(s.selection_snapshot, ["hero"]);

        let cloned = s.clone();
        assert!(
            Arc::ptr_eq(&s.framework_cache, &cloned.framework_cache),
            "panel snapshots must not deep-clone every cached source string"
        );
        let cloned_vue = cloned
            .framework_cache
            .iter()
            .find(|entry| entry.framework == Framework::Vue)
            .expect("cached Vue result");
        assert!(s.select_framework(Framework::React));
        let live_vue = s
            .framework_cache
            .iter()
            .find(|entry| entry.framework == Framework::Vue)
            .expect("cached Vue result after copy-on-write");
        assert!(
            Arc::ptr_eq(cloned_vue, live_vue),
            "copy-on-write must retain untouched cached source by reference"
        );
    }

    #[test]
    fn selecting_the_current_or_an_in_flight_framework_is_a_noop() {
        let mut complete = CodegenState {
            code: "react output".into(),
            phase: CodegenPhase::Complete,
            ..CodegenState::default()
        };
        assert!(!complete.select_framework(Framework::React));
        assert_eq!(complete.code, "react output");

        let mut generating = CodegenState {
            phase: CodegenPhase::Generating,
            ..CodegenState::default()
        };
        assert!(!generating.select_framework(Framework::Vue));
        assert_eq!(generating.framework, Framework::React);
        assert_eq!(generating.phase, CodegenPhase::Generating);
    }
}
