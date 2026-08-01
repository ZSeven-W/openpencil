//! Grouped sub-states carved out of [`super::EditorUiState`]'s flat
//! field list.
//!
//! The struct is cloned wholesale on request paths and had grown to
//! ~190 flat fields; these are the clusters whose fields are only ever
//! read together, so folding each into a named struct (the same shape
//! `git_panel: GitPanelState` already had) shortens the field list
//! without changing any behavior.
//!
//! `EditorUiState` carries no `serde` derive and no snapshot
//! serialization reaches into these fields (settings persistence lives
//! in `op-editor-host-core::settings_payload`, which never names them),
//! so the regrouping has no wire / settings-format impact.

use super::{DesignMdRequest, PreviewDeviceKind};
use crate::design_md_button_state::DesignMdButton;
use crate::prompt_center_catalog::PromptCategory;
use crate::scene_template_catalog::TemplateScene;
use serde::{Deserialize, Serialize};

/// Canvas **Preview** (Play) mode state.
///
/// Entering Preview stops painting selection handles + editor chrome
/// and drives a live jian runtime host-side; the runtime itself is
/// `!Send`, so only these plain flags live on the wasm32-clean state.
/// `EditorUiState::{enter,exit,toggle}_preview` own the transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewState {
    /// Whether the canvas is in Preview (Play) mode. Entering does NOT
    /// mutate `doc`; exiting drops the runtime and leaves `doc`
    /// byte-identical. The TopBar Play/Stop button + `Esc` toggle it.
    pub mode: bool,
    /// Non-fatal warnings raised when the runtime was last built from
    /// `doc` for Preview — e.g. legacy role-frames promoted to widget
    /// nodes (`LegacyRolePromoted`). Surfaced for diagnostics; cleared
    /// on exit. Never serialized.
    pub warnings: Vec<String>,
    /// RESOLVED device-frame kind while previewing (`None` outside
    /// preview; never serialized). Three writers: `enter_preview`
    /// inference (host-side), the switcher, and the host's app-mode
    /// screen-switch re-inference — every re-inference writes back
    /// here so the switcher and the frame can never disagree.
    pub device: Option<PreviewDeviceKind>,
    /// Switcher segment under the cursor (hover wash).
    pub switcher_hover: Option<PreviewDeviceKind>,
    /// Switcher segment currently pressed (activates on RELEASE
    /// inside the same segment).
    pub switcher_pressed: Option<PreviewDeviceKind>,
    /// APP MODE screen-switcher pill under the cursor (hover wash),
    /// indexed into the session's current screen list. `None` outside
    /// hover and outside APP MODE. Never serialized.
    pub screen_switcher_hover: Option<usize>,
    /// APP MODE screen-switcher pill currently pressed (activates on
    /// RELEASE inside the same pill, mirroring [`Self::switcher_pressed`]).
    pub screen_switcher_pressed: Option<usize>,
}

/// PropertyPanel Size-section fill / hug / clip toggles.
///
/// Mirrors the same-named fields on the panel's own
/// `PropertyPanelSnapshot`; the panel derives them per frame from the
/// selected node, so these are the editor-level echo of that state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeToggleState {
    pub fill_width: bool,
    pub fill_height: bool,
    pub hug_width: bool,
    pub hug_height: bool,
    pub clip_content: bool,
}

/// Floating Design-MD panel state.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignMdPanelState {
    /// Whether the floating Design-MD panel is shown.
    pub open: bool,
    /// Which design-md-panel button the cursor is over (close / import
    /// / export / remove / section header) — drives the hover wash.
    pub hover: Option<DesignMdButton>,
    /// Top-left corner of the panel in logical px. `None` until first
    /// opened — the host then centres it on the viewport.
    pub pos: Option<(f32, f32)>,
    /// Bitmask of expanded sections (bit 0 = theme, 1 = colors, 2 =
    /// typography, 3 = components, 4 = layout, 5 = notes). Defaults to
    /// theme + colors + typography expanded.
    pub expanded: u8,
    /// Vertical scroll offset (px) of the panel body.
    pub scroll: jian_core::scroll::ScrollState,
    /// True while the desktop host is waiting for an AI-generated
    /// design.md brief. Transient: never serialized.
    pub generating: bool,
    /// A queued import / export request — set by a panel click, drained
    /// by the desktop host (which owns the native file dialog).
    /// Transient: never serialized.
    pub request: Option<DesignMdRequest>,
}

impl Default for DesignMdPanelState {
    fn default() -> Self {
        Self {
            open: false,
            hover: None,
            pos: None,
            // theme + colors + typography expanded.
            expanded: 0b0000_0111,
            scroll: jian_core::scroll::ScrollState::default(),
            generating: false,
            request: None,
        }
    }
}

/// Which input inside the Prompt Center owns keyboard editing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PromptCenterFocus {
    /// The catalogue search field.
    #[default]
    Search,
    /// The title field in the save-current-prompt form.
    SaveTitle,
}

/// The catalogue subset selected by the Prompt Center chip row.
///
/// "My" is a view over custom entries rather than a storage category,
/// so it remains distinct from [`PromptCategory`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PromptFilter {
    /// Every built-in prompt.
    #[default]
    All,
    /// One fixed built-in category.
    Category(PromptCategory),
    /// User-saved prompts only.
    Custom,
}

/// One user-saved prompt persisted by the native host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub id: String,
    pub title: String,
    pub body: String,
    pub category: PromptCategory,
    pub created_at: u64,
}

/// Grouped state for the non-modal Prompt Center panel.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptCenterState {
    /// Whether the floating panel is visible.
    pub open: bool,
    /// Search text, caret, selection, and IME state.
    pub search: jian_core::text_input::TextInputState,
    /// Active catalogue filter.
    pub filter: PromptFilter,
    /// Vertical card-grid scroll.
    pub scroll: jian_core::scroll::ScrollState,
    /// Widget-defined hover token. Card rows use their filtered index;
    /// chrome controls use reserved high values.
    pub hover: Option<usize>,
    /// Keyboard owner while the panel is open.
    pub focus: PromptCenterFocus,
    /// Whether the inline save-current-prompt form is expanded.
    pub save_open: bool,
    /// Title draft for a custom prompt.
    pub save_title: jian_core::text_input::TextInputState,
    /// Storage category selected for the custom prompt.
    pub save_category: PromptCategory,
    /// Prompts loaded from the host-owned config store.
    pub custom_prompts: Vec<CustomPrompt>,
    /// Whether this host can persist custom prompts.
    pub custom_store_writable: bool,
    /// Raised by save/delete and drained after a successful host persist.
    pub custom_store_dirty: bool,
}

impl Default for PromptCenterState {
    fn default() -> Self {
        Self {
            open: false,
            search: Default::default(),
            filter: PromptFilter::All,
            scroll: Default::default(),
            hover: None,
            focus: PromptCenterFocus::Search,
            save_open: false,
            save_title: Default::default(),
            save_category: PromptCategory::Starter,
            custom_prompts: Vec::new(),
            custom_store_writable: false,
            custom_store_dirty: false,
        }
    }
}

impl PromptCenterState {
    /// Open the panel with search focused and no stale hover/scroll.
    pub fn open(&mut self, now_ms: u64) {
        self.open = true;
        self.focus = PromptCenterFocus::Search;
        self.hover = None;
        self.scroll.offset = 0.0;
        self.search.touch(now_ms);
    }

    /// Close only this panel layer.
    pub fn close(&mut self) {
        self.open = false;
        self.hover = None;
        self.save_open = false;
        self.focus = PromptCenterFocus::Search;
    }

    /// Replace host-loaded custom prompts without raising persistence.
    pub fn install_custom_prompts(&mut self, prompts: Vec<CustomPrompt>, writable: bool) {
        self.custom_prompts = prompts;
        self.custom_store_writable = writable;
        self.custom_store_dirty = false;
    }

    /// Save a custom prompt and return its stable generated id.
    pub fn add_custom_prompt(
        &mut self,
        title: String,
        body: String,
        category: PromptCategory,
        created_at: u64,
    ) -> Option<String> {
        let title = title.trim();
        let body = body.trim();
        if !self.custom_store_writable || title.is_empty() || body.is_empty() {
            return None;
        }
        let mut suffix = 0_u32;
        let id = loop {
            let candidate = if suffix == 0 {
                format!("custom-{created_at}")
            } else {
                format!("custom-{created_at}-{suffix}")
            };
            if self
                .custom_prompts
                .iter()
                .all(|prompt| prompt.id != candidate)
            {
                break candidate;
            }
            suffix = suffix.saturating_add(1);
        };
        self.custom_prompts.push(CustomPrompt {
            id: id.clone(),
            title: title.to_owned(),
            body: body.to_owned(),
            category,
            created_at,
        });
        self.custom_store_dirty = true;
        self.filter = PromptFilter::Custom;
        self.save_open = false;
        self.focus = PromptCenterFocus::Search;
        self.save_title.set_text("");
        self.scroll.offset = 0.0;
        Some(id)
    }

    /// Delete one custom prompt by id.
    pub fn delete_custom_prompt(&mut self, id: &str) -> bool {
        if !self.custom_store_writable {
            return false;
        }
        let before = self.custom_prompts.len();
        self.custom_prompts.retain(|prompt| prompt.id != id);
        let changed = self.custom_prompts.len() != before;
        self.custom_store_dirty |= changed;
        changed
    }

    /// Mutable keyboard-owned field.
    pub fn focused_input_mut(&mut self) -> &mut jian_core::text_input::TextInputState {
        match self.focus {
            PromptCenterFocus::Search => &mut self.search,
            PromptCenterFocus::SaveTitle => &mut self.save_title,
        }
    }
}

/// The catalogue subset selected by the Scene Template Center chip row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SceneFilter {
    /// Every shipped template.
    #[default]
    All,
    /// One scene.
    Scene(TemplateScene),
}

/// Grouped state for the non-modal Scene Template Center panel.
///
/// Deliberately smaller than [`PromptCenterState`]: a template is opened,
/// never authored, so there is no save form and no user-owned entries to
/// persist. Adding those later means adding fields here, not reshaping the
/// panel.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneTemplateCenterState {
    /// Whether the floating panel is visible.
    pub open: bool,
    /// Search text, caret, selection, and IME state.
    pub search: jian_core::text_input::TextInputState,
    /// Active catalogue filter.
    pub filter: SceneFilter,
    /// Vertical card-grid scroll.
    pub scroll: jian_core::scroll::ScrollState,
    /// Widget-defined hover token. Card rows use their filtered index;
    /// chrome controls use reserved high values.
    pub hover: Option<usize>,
    /// Raised when a card is chosen; the host drains it to load the
    /// document. Kept as a request rather than applied inline because
    /// opening a document is a host capability (file dialogs, unsaved-work
    /// prompts), not a widget one.
    pub pending_open: Option<String>,
}

impl Default for SceneTemplateCenterState {
    fn default() -> Self {
        Self {
            open: false,
            search: Default::default(),
            filter: SceneFilter::All,
            scroll: Default::default(),
            hover: None,
            pending_open: None,
        }
    }
}

impl SceneTemplateCenterState {
    /// Open the panel with search focused and no stale hover/scroll.
    pub fn open(&mut self, now_ms: u64) {
        self.open = true;
        self.hover = None;
        self.scroll.offset = 0.0;
        self.search.touch(now_ms);
    }

    /// Close only this panel layer.
    pub fn close(&mut self) {
        self.open = false;
        self.hover = None;
    }

    /// Request that the host open `template_id` and close the panel.
    pub fn request_open(&mut self, template_id: impl Into<String>) {
        self.pending_open = Some(template_id.into());
        self.close();
    }

    /// Drain a pending open request.
    pub fn take_pending_open(&mut self) -> Option<String> {
        self.pending_open.take()
    }
}
