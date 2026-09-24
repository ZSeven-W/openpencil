//! "Share + make one like this": the recipe a Studio document carries.
//!
//! A design generated from Home is the product of a brief, a task family
//! with its options, and (optionally) a pinned style guide. The recipe is
//! that triple, written into the `editorMeta` of the `.op` a share package
//! carries (never by an ordinary save) so a shared file tells the
//! recipient's app how it was made. Opening such a file offers
//! "Make one like this": Home comes back with the same task and options,
//! the same style guide pinned, and the original brief as an editable
//! starting point — the recipient generates their own version in the same
//! style rather than copying the boards.
//!
//! The recipe leaves the machine inside a file someone else opens, so
//! everything user-typed is passed through [`sanitize_share_text`] before
//! it is captured: local file paths, API keys and caller-named terms (the
//! OS user name) never travel with it.

use super::{EntrySurface, HomeDevice, HomeFamily, InfoKind, SlideRatio, TaskDraft};
use crate::editor_ui_state::workspace::WorkspacePhase;
use crate::editor_ui_state::EditorUiState;

/// What a redacted span of a shared brief reads as.
pub const SHARE_REDACTED: &str = "[…]";

/// Longest brief a recipe keeps, in chars. A recipe is a starting point
/// for someone else's brief, not an archive of the author's.
pub const SHARE_BRIEF_MAX_CHARS: usize = 2_000;

/// How a Studio document was made: the brief, the task and its options,
/// and the style guide the run was pinned to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareRecipe {
    /// The brief as the author typed it (sanitized on capture).
    pub brief: String,
    pub family: HomeFamily,
    pub device: HomeDevice,
    pub ratio: SlideRatio,
    pub info_kind: InfoKind,
    /// Style-guide registry name, when the run was pinned to one.
    pub style_guide: Option<String>,
}

impl ShareRecipe {
    /// Capture a recipe from a task, its brief and options. The brief is
    /// sanitized here so no caller can forget to.
    pub fn capture(
        family: HomeFamily,
        brief: &str,
        options: &TaskDraft,
        style_guide: Option<&str>,
    ) -> Self {
        Self {
            brief: String::new(),
            family,
            device: options.device,
            ratio: options.ratio,
            info_kind: options.info_kind,
            style_guide: style_guide.map(str::to_string),
        }
        .with_brief(brief, &[])
    }

    /// This recipe with `brief` sanitized against `extra_terms` and
    /// clamped to [`SHARE_BRIEF_MAX_CHARS`].
    pub fn with_brief(mut self, brief: &str, extra_terms: &[&str]) -> Self {
        let clean = sanitize_share_text(brief.trim(), extra_terms);
        self.brief = clean.chars().take(SHARE_BRIEF_MAX_CHARS).collect();
        self
    }

    /// Re-sanitize the whole recipe against host-known terms (the OS
    /// user name, the home directory). A style-guide name that is not a
    /// plain registry name is dropped rather than carried.
    pub fn sanitized(&self, extra_terms: &[&str]) -> Self {
        let mut next = self.clone().with_brief(&self.brief, extra_terms);
        next.style_guide = self
            .style_guide
            .as_deref()
            .filter(|name| is_registry_name(name))
            .map(str::to_string);
        next
    }

    /// The task options this recipe restores, with the brief as text.
    pub fn options(&self) -> TaskDraft {
        TaskDraft {
            text: self.brief.clone(),
            device: self.device,
            ratio: self.ratio,
            info_kind: self.info_kind,
        }
    }
}

/// A style-guide name travels only when it looks like a registry name:
/// short, one line, and free of path separators.
fn is_registry_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.chars().count() <= 80
        && !name.contains(['/', '\\', '\n', '\r'])
        && !redact_token(name)
}

impl HomeDevice {
    /// Stable persistence id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mobile => "mobile",
            Self::Desktop => "desktop",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        [Self::Mobile, Self::Desktop]
            .into_iter()
            .find(|device| device.id() == value)
    }
}

impl SlideRatio {
    /// Stable persistence id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Wide169 => "16:9",
            Self::Classic43 => "4:3",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        [Self::Wide169, Self::Classic43]
            .into_iter()
            .find(|ratio| ratio.id() == value)
    }
}

impl InfoKind {
    /// Stable persistence id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Flow => "flow",
            Self::Comparison => "comparison",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        [Self::Data, Self::Flow, Self::Comparison]
            .into_iter()
            .find(|kind| kind.id() == value)
    }
}

/// Redact what must not leave the machine from user-typed text: local
/// file paths, credential-looking tokens, and every `extra_terms` entry
/// that appears as a whole word (the host passes the OS user name).
///
/// The scan works on runs of non-whitespace ASCII, so a path or key typed
/// inside CJK prose is cut out without touching the prose around it.
pub fn sanitize_share_text(text: &str, extra_terms: &[&str]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    // A label such as `password:` redacts the value typed after it too,
    // however short that value is.
    let mut value_follows = false;
    let mut flush = |run: &mut String, out: &mut String| {
        if run.is_empty() {
            return;
        }
        if value_follows || redact_token(run) {
            out.push_str(SHARE_REDACTED);
            value_follows = !value_follows && run.ends_with([':', '=']);
        } else {
            out.push_str(&redact_terms(run, extra_terms));
        }
        run.clear();
    };
    for character in text.chars() {
        if character.is_ascii() && !character.is_ascii_whitespace() {
            run.push(character);
        } else {
            flush(&mut run, &mut out);
            out.push(character);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Whether one ASCII run is a local path or a credential.
fn redact_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    // Local paths, in every spelling a user might paste.
    const PATH_MARKERS: [&str; 11] = [
        "file://",
        "/users/",
        "/home/",
        "/private/",
        "/var/",
        "/tmp/",
        "/volumes/",
        "/opt/",
        "/etc/",
        "/mnt/",
        "/root/",
    ];
    if PATH_MARKERS.iter().any(|marker| lower.contains(marker))
        || lower.starts_with("~/")
        || lower.starts_with("~\\")
        || lower.contains(":\\")
        || lower.matches('\\').count() >= 2
        || (lower.starts_with('/') && lower.matches('/').count() >= 3)
    {
        return true;
    }
    // Named credentials: `apiKey=…`, `token:…`. The bare words stay —
    // "design tokens" and "a password manager" are ordinary briefs.
    const SECRET_WORDS: [&str; 8] = [
        "apikey",
        "api_key",
        "api-key",
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
    ];
    if SECRET_WORDS.iter().any(|word| {
        lower
            .match_indices(word)
            .any(|(at, _)| matches!(lower.as_bytes().get(at + word.len()), Some(b'=' | b':')))
    }) {
        return true;
    }
    // Well-known key prefixes.
    const KEY_PREFIXES: [&str; 12] = [
        "sk-",
        "sk_",
        "pk_",
        "rk_",
        "ghp_",
        "gho_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "glpat-",
        "hf_",
        "aiza",
    ];
    if token.len() >= 12 && KEY_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
        return true;
    }
    if token.len() >= 16 && token.starts_with("AKIA") {
        return true;
    }
    // Any long opaque token: letters AND digits, nothing but key chars.
    token.len() >= 32
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_alphabetic())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/' | '='))
}

/// Replace every whole-word, case-insensitive occurrence of a term.
fn redact_terms(run: &str, extra_terms: &[&str]) -> String {
    let mut out = run.to_string();
    for term in extra_terms {
        let term = term.trim();
        if term.chars().count() < 3 || !term.is_ascii() {
            continue;
        }
        out = replace_whole_word(&out, term);
    }
    out
}

fn replace_whole_word(haystack: &str, term: &str) -> String {
    let lower = haystack.to_ascii_lowercase();
    let needle = term.to_ascii_lowercase();
    let bytes = haystack.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut cursor = 0usize;
    let mut search = 0usize;
    while let Some(found) = lower[search..].find(&needle) {
        let start = search + found;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            out.push_str(&haystack[cursor..start]);
            out.push_str(SHARE_REDACTED);
            cursor = end;
        }
        search = end;
    }
    out.push_str(&haystack[cursor..]);
    out
}

/// A Make-one-like-this press staged for Home's next send: the style
/// guide to re-pin once the send has swapped in a fresh document (the
/// swap resets every document-scoped pin).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MakeSameStage {
    pub style_guide: Option<String>,
}

impl EditorUiState {
    /// The recipe this document carries right now: the live workspace's
    /// run when there is one, else the recipe the file was opened with.
    pub fn share_recipe(&self) -> Option<ShareRecipe> {
        let workspace = &self.workspace;
        if workspace.active && !workspace.shared_view && !workspace.brief.trim().is_empty() {
            return Some(ShareRecipe::capture(
                workspace.family,
                &workspace.brief,
                &workspace.options,
                self.pinned_style_guide.as_deref(),
            ));
        }
        self.home.recipe.clone()
    }

    /// Present an opened document that carries a recipe in the Studio
    /// workspace, settled, with the Make-one-like-this banner up. Touch
    /// chrome keeps its own reader. Returns whether the workspace opened.
    pub fn open_workspace_for_shared_recipe(&mut self, now_ms: u64) -> bool {
        if self.touch_chrome() {
            return false;
        }
        let Some(recipe) = self.home.recipe.clone() else {
            return false;
        };
        self.open_workspace_for_generation(
            recipe.family,
            recipe.brief.clone(),
            recipe.options(),
            0,
            now_ms,
            None,
        );
        self.workspace.phase = WorkspacePhase::Done;
        self.workspace.shared_view = true;
        // Nothing has been said about this document yet: the canvas gets
        // the width, and the conversation is one toggle away.
        self.sidebar_open = false;
        true
    }

    /// Make one like this: Home comes back on the recipe's task with its
    /// options, the brief pre-filled as an editable starting point, and
    /// the recipe's style guide pinned — staged so Home's send re-pins it
    /// after swapping in the fresh document.
    pub fn begin_make_same(&mut self, now_ms: u64) -> bool {
        let Some(recipe) = self.share_recipe() else {
            return false;
        };
        let home = &mut self.home;
        home.set_task(recipe.family, now_ms);
        home.set_device(recipe.device);
        home.set_ratio(recipe.ratio);
        home.set_info_kind(recipe.info_kind);
        home.set_draft(recipe.brief.clone());
        let end = home.input.text().len();
        home.set_caret(end, now_ms);
        home.replace_pending = false;
        home.composer_focused = true;
        home.make_same = Some(MakeSameStage {
            style_guide: recipe.style_guide.clone(),
        });
        home.visible = true;
        self.entry_surface = EntrySurface::Home;
        if let Some(guide) = recipe.style_guide {
            self.pinned_style_guide = Some(guide);
        }
        true
    }

    /// Re-pin a staged Make-one-like-this style guide. Hosts call this in
    /// Home's send right after any fresh-document swap; a send with no
    /// staged press changes nothing.
    pub fn restore_make_same_pin(&mut self) {
        if let Some(stage) = self.home.make_same.take() {
            if stage.style_guide.is_some() {
                self.pinned_style_guide = stage.style_guide;
            }
        }
    }
}

#[cfg(test)]
#[path = "share_recipe_tests.rs"]
mod tests;
