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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenState {
    pub framework: Framework,
    pub phase: CodegenPhase,
    pub progress: CodeGenProgress,
    pub code: String,
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
}

impl Default for CodegenState {
    fn default() -> Self {
        Self {
            framework: Framework::React,
            phase: CodegenPhase::Idle,
            progress: CodeGenProgress::default(),
            code: String::new(),
            degraded: false,
            assets: Vec::new(),
            selection_snapshot: Vec::new(),
            error: None,
            copied_at: None,
            pending_generate: false,
            pending_regenerate: false,
        }
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
    }

    #[test]
    fn codegen_state_defaults_to_idle_react() {
        let s = CodegenState::default();
        assert_eq!(s.framework, Framework::React);
        assert_eq!(s.phase, CodegenPhase::Idle);
        assert!(s.code.is_empty());
        assert!(!s.degraded);
        assert!(s.error.is_none());
        assert!(!s.pending_generate);
        assert!(!s.pending_regenerate);
    }
}
