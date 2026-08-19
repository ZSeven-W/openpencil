//! App-chrome state types: theme / embed host, file-menu actions,
//! recent files, theme-preset IO, the auto-update probe result, the
//! Design-MD panel request, and the pencil-cursor style.
//!
//! Split out of the `editor_ui_state` spine (800-line file ceiling);
//! every type is re-exported from there, so import paths are unchanged.

/// Light / dark UI theme switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn flipped(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

/// Which embedding container the editor chrome renders inside. Each embed
/// host hides the chrome its container already provides; `None` is the
/// full standalone chrome. Unknown query values stay `None` so a newer
/// page URL never breaks an older bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbedHost {
    #[default]
    None,
    /// VS Code / Cursor custom-editor webview (`?embed=vscode`).
    VsCode,
}

impl EmbedHost {
    /// Parse a `window.location.search` value (leading `?` optional).
    pub fn from_query(search: &str) -> Self {
        let trimmed = search.strip_prefix('?').unwrap_or(search);
        for pair in trimmed.split('&') {
            if pair.strip_prefix("embed=") == Some("vscode") {
                return Self::VsCode;
            }
        }
        Self::None
    }
}

/// A press on the TopBar's engine-painted window-control dots, raised for
/// the platform shell to execute.
///
/// The widget layer owns no window, so the dots can only record an intent.
/// The desktop runner reads the dots directly through
/// `TopBar::window_control_at` and never sets this; the embedded mobile
/// hosts (whose shells own the window) drain it through the C ABI's
/// shell-action channel instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlRequest {
    /// Red dot — close the window.
    Close,
    /// Yellow dot — minimise the window.
    Minimize,
    /// Green dot — toggle maximised / restored.
    Zoom,
}

/// File-menu actions the host runner has to handle (rfd dialogs +
/// serde live host-side, not here). `ExportImage` opens the picker;
/// `ExportImageConfirm` commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportImage,
    ExportImageConfirm,
    /// Slides rail ▸ Export PDF ▸ "Export selected slides" — host picks
    /// one output file and writes a slide-per-page PDF holding ONLY the
    /// boards `preview_slideshow::selected_page_boards` returns.
    ///
    /// Separate from `ExportImageConfirm` rather than a flag beside it
    /// because the scope is a property of the ACTION, not of the
    /// document: a flag would have to be cleared by every other export
    /// entry point, and the one that forgot would silently ship a
    /// one-slide PDF.
    ExportDeckPdfSelection,
    /// File ▸ "Export all frames" — host picks one output directory and
    /// writes every planned frame into it (see
    /// `op_editor_core::export_batch` for the plan, and the host's
    /// batch exporter for the render loop).
    ExportAllFrames,
    /// File ▸ "Export slideshow HTML" — host picks one output file and
    /// writes the deck as a self-contained slideshow page (see the
    /// host's `export_html` module for the render + markup).
    ExportSlideshowHtml,
    /// File ▸ "Export PowerPoint" — host picks one output file and
    /// writes the deck as an editable `.pptx` (see the host's
    /// `export_pptx` module for the package + DrawingML emission).
    ExportPptx,
    ImportFigma,
    /// Complete or cancel a prepared multi-page Figma import. Native
    /// desktop owns the prepared tree; web safely ignores this action.
    FinishFigmaImport(crate::figma_import_state::FigmaImportSelection),
    /// User chose `从 HTML 导入` in the top-bar import menu — host
    /// opens a file dialog for a saved page / snapshot and hands the
    /// path to the background HTML import worker.
    ImportHtml,
    /// User chose `Import image or SVG…` in the toolbar shape picker
    /// — host opens a file dialog, then inserts the raster image as a
    /// new Image node (or parses the SVG into nodes; SVG path lands
    /// as a follow-up).
    ImportImageOrSvg,
    /// User clicked the `图片` fill body row — host opens a file
    /// dialog and writes the chosen image into the selected node's
    /// primary fill as `PenFill::Image { url: <data-url> }`.
    PickFillImage,
    /// User clicked the image-section warning row's Relink button —
    /// host opens a file dialog and rewrites the selected image
    /// node's `src` (relative to the document path when possible,
    /// TS `toStoredAssetPath`).
    RelinkImage,
    OpenRecent(usize),
    ClearRecent,
}

/// Theme-preset file IO the host must run (rfd dialog + fs IO live
/// host-side). Raised by the preset dropdown's Import / Export rows
/// (TS `variable-theme-manager.tsx:153-164`); drained by the desktop
/// host (`theme_preset_host.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePresetIo {
    /// `variables.importPreset` — pick a `.optheme` file and merge
    /// its themes + variables into the document.
    Import,
    /// `variables.exportPreset` — save the current document themes +
    /// variables as `theme-preset.optheme`.
    Export,
}

/// Auto-update status surfaced in the settings modal's System tab.
///
/// The desktop host runs a background probe against the GitHub
/// releases API and writes the outcome here; the System tab paints
/// from it. `Idle` is the pre-probe state, `Checking` while the
/// request is in flight. `Available` carries the newer release tag
/// so the tab can name the version the user can upgrade to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    /// No check has run yet (also the web build's permanent state).
    #[default]
    Idle,
    /// A release-API request is in flight.
    Checking,
    /// The running build is the latest published release.
    UpToDate,
    /// A newer release exists — carries its version string.
    Available { version: String },
    /// The probe failed (offline, rate-limited, parse error).
    Error,
}

/// A Design-MD panel action that needs the desktop host's native
/// file dialog — set by the widget layer, drained by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdRequest {
    /// Pick a `.md` file, parse it, and set `design_md`.
    Import,
    /// Ask the selected AI model to generate a fresh design.md from
    /// the current document.
    AutoGenerate,
    /// Write the current `design_md` to a `.md` file.
    Export,
}

/// File-menu "Recent files" entry — host persists via settings IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    /// Unix seconds when last touched.
    pub modified_at: u64,
}

/// Maximum number of recent files shown in the File menu.
pub const RECENT_FILE_CAP: usize = 10;

/// Pencil-cursor silhouette variants (Settings > System > Pencil cursor).
/// `Rounded` is the shipped default (user-picked, 2026-07-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PencilCursorStyle {
    Classic,
    #[default]
    Rounded,
    Chubby,
    Crayon,
    Marker,
}

impl PencilCursorStyle {
    pub const ALL: [PencilCursorStyle; 5] = [
        PencilCursorStyle::Classic,
        PencilCursorStyle::Rounded,
        PencilCursorStyle::Chubby,
        PencilCursorStyle::Crayon,
        PencilCursorStyle::Marker,
    ];

    /// Stable id for persistence.
    pub fn id(self) -> &'static str {
        match self {
            PencilCursorStyle::Classic => "classic",
            PencilCursorStyle::Rounded => "rounded",
            PencilCursorStyle::Chubby => "chubby",
            PencilCursorStyle::Crayon => "crayon",
            PencilCursorStyle::Marker => "marker",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.id() == id)
    }

    /// Display label for the settings row.
    pub fn label(self) -> &'static str {
        match self {
            PencilCursorStyle::Classic => "Classic",
            PencilCursorStyle::Rounded => "Rounded",
            PencilCursorStyle::Chubby => "Chubby",
            PencilCursorStyle::Crayon => "Crayon",
            PencilCursorStyle::Marker => "Marker",
        }
    }
}
