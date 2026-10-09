//! Deck → Hyperframes composition — the same resolved-scene markup the
//! slideshow export writes, laid out on a TIME axis instead of a key
//! press.
//!
//! # Division of labour with `export_html`
//!
//! [`crate::export_html`] writes a PLAYER: one file a presenter opens
//! and drives, where a slide changes because a human pressed a key.
//! This module writes RENDER STOCK: the same boards, each pinned to a
//! `data-start` / `data-duration` window so a headless renderer can walk
//! the deck frame by frame and encode it as video. Nothing about the
//! slide markup itself differs — both call
//! [`crate::export_html_structured::board_slide_markup`], so a slide
//! that presents correctly renders correctly, and a fix to the emitter
//! reaches both artifacts at once.
//!
//! # Why the timeline is integers
//!
//! The renderer is frame-driven (`frame = floor(time × fps)`), so the
//! only thing that can desynchronise a cut is our own arithmetic.
//! Durations are therefore computed in whole [`TICKS_PER_SECOND`]ths of
//! a second and ACCUMULATED AS INTEGERS: `start[i+1]` is literally
//! `start[i] + duration[i]`, in the attribute text as much as in the
//! maths. Summing floats would eventually emit a start that is a
//! hair short of the previous scene's end, and a one-frame gap shows up
//! on screen as a black flash.
//!
//! # Why every scene animates itself
//!
//! The renderer hides and shows a scene from its `class="clip"` plus
//! its window attributes, and that remains the mechanism. On top of it
//! each scene ALSO carries a CSS animation whose delay is its own start
//! and whose duration is its own window, with
//! `animation-fill-mode:none` so the element falls back to hidden the
//! instant that window closes. Belt and braces on purpose: the file is
//! opened by humans too (a browser, the studio preview) where no
//! runtime is driving anything, and a composition that reads as every
//! slide stacked on the last one outside the renderer is a composition
//! nobody can eyeball before spending a render on it. CSS animations
//! are seekable, which is what keeps the redundancy safe under
//! frame-driven capture — the renderer sets the clock, the animation
//! state follows from it, and the same frame index always produces the
//! same pixels.
//!
//! # Cuts, not transitions
//!
//! Scene changes are hard cuts: the incoming slide's box, background
//! and all, is fully painted on the first frame of its window. The only
//! motion is a 0.3 s fade-in of what sits ON that slide, which is the
//! entrance tween a cut is normally given. Fading the slide box itself
//! is what NOT to do, and it is a mistake with no symptom until you
//! render: the slide's own background goes with it, so every cut
//! becomes a black frame. No shader transitions, no audio track.
//!
//! # Self-containment
//!
//! Identical to the slideshow export: not one URL may be emitted. The
//! markup comes from the same emitter (which embeds anything it cannot
//! express as a `data:` PNG), and the page's stylesheet is inlined by
//! [`markup`]. The renderer opens the file in a sandboxed browser, so a
//! resource it would have to fetch is a frame it would have to capture
//! without.

use std::path::{Path, PathBuf};

use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::EditorState;

use crate::export::ExportError;
use crate::export_html::board_name;
use crate::export_html_structured::{board_slide_markup, css_num};

mod markup;

#[cfg(test)]
#[path = "export_hyperframes/tests.rs"]
mod tests;

/// Timeline resolution. Tenths of a second divide evenly into every
/// common capture rate (3 frames at 30 fps, 6 at 60), so a scene
/// boundary always lands ON a frame rather than inside one.
pub const TICKS_PER_SECOND: u32 = 10;

/// Reading budget, in text units per second. A unit is one CJK
/// character; Latin words are converted at [`LATIN_WORD_UNITS`], which
/// puts English at ~160 words per minute — the usual comfortable
/// silent-reading rate, and the rate the CJK figure was chosen against.
const UNITS_PER_SECOND: f32 = 8.0;

/// One Latin word costs this many text units. Three keeps the two
/// scripts on one scale instead of making a slide of English words race
/// past at CJK character speed.
const LATIN_WORD_UNITS: f32 = 3.0;

/// Time a slide is held before its text is counted at all: the beat a
/// viewer spends registering that the slide CHANGED, before reading
/// starts.
const BASE_SECONDS: f32 = 1.5;

/// Floor on a scene. Below three seconds a viewer who blinked at the
/// cut has no chance to recover, however little text the slide carries
/// — a title slide of two words still needs to be seen.
const MIN_SECONDS: f32 = 3.0;

/// Ceiling on a scene. Past ten seconds a static frame reads as a
/// stall; a slide with that much text is a slide that should be split,
/// and stretching its hold would hide the problem rather than fix it.
const MAX_SECONDS: f32 = 10.0;

/// Entrance tween, in ticks (0.3 s). The upper end of the renderer
/// guideline's 0.1–0.3 s entrance range — long enough to read as
/// motion, short enough that the cut still feels hard.
const FADE_TICKS: u32 = 3;

/// Capture rate written onto the composition root. Thirty is the
/// renderer's own default; declaring it makes the frame grid a property
/// of the artifact rather than of whichever CLI version renders it.
const FPS: u32 = 30;

/// File names written by [`export_deck_hyperframes`].
///
/// The composition is `index.html` because that is what the renderer
/// means by a project: `hyperframes render <dir>` looks for exactly
/// that name, and a second file carrying `data-composition-id` beside
/// it is a lint ERROR (two discoverable entry points). So the directory
/// IS the deliverable, and it holds one composition.
pub const COMPOSITION_FILE: &str = "index.html";
pub const RENDER_NOTES_FILE: &str = "RENDER.md";

/// Fallback composition id / title for an unnamed deck.
const DEFAULT_ID: &str = "deck";

/// One board's slot on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Authored board name — the scene's accessible label.
    pub name: String,
    /// Offset from the start of the composition, in ticks.
    pub start_ticks: u32,
    /// How long the scene holds, in ticks.
    pub duration_ticks: u32,
    /// Board size in doc px.
    pub width: f32,
    pub height: f32,
    /// The slide's inner markup, in board-local coordinates.
    pub body: String,
}

/// A built composition, before it is written anywhere.
#[derive(Debug, Clone, PartialEq)]
pub struct Composition {
    /// The single self-contained HTML file.
    pub html: String,
    /// Companion notes: how to render this file.
    pub render_notes: String,
    /// Video canvas size in px — the first visible board's size.
    pub width: f32,
    pub height: f32,
    /// Scenes emitted, in presentation order.
    pub scenes: usize,
    /// Whole-composition length in ticks.
    pub total_ticks: u32,
    /// Nodes emitted as real elements across the deck.
    pub structured_nodes: usize,
    /// Nodes that had to be embedded as a raster image instead.
    pub raster_fallbacks: usize,
}

impl Composition {
    /// Composition length in seconds. Exact: the tick count is an
    /// integer and the divisor is a power-of-ten constant.
    pub fn total_seconds(&self) -> f32 {
        self.total_ticks as f32 / TICKS_PER_SECOND as f32
    }
}

/// What one export wrote, and where.
#[derive(Debug, Clone, PartialEq)]
pub struct HyperframesExport {
    pub composition_path: PathBuf,
    pub render_notes_path: PathBuf,
    pub composition: Composition,
}

/// Build the active page's deck as a Hyperframes composition.
///
/// Page order comes from [`active_page_boards`] — the same single source
/// the native slideshow and the HTML export present from — so the video
/// runs in exactly the order Preview does, and a board the author hid is
/// skipped in all three.
///
/// A board that fails to render aborts the build rather than being
/// dropped, for the reason the slideshow export gives: the artifact is
/// one file whose timeline would silently close over the hole, and the
/// missing slide would only surface in the finished video.
pub fn deck_composition(state: &EditorState) -> Result<Composition, ExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_export_layout_scene(state);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;

    let mut scenes: Vec<Scene> = Vec::new();
    let mut structured_nodes = 0;
    let mut raster_fallbacks = 0;
    let mut cursor = 0;
    for board_id in active_page_boards(state) {
        let Some(node) = page.find(&board_id) else {
            return Err(ExportError::NodeNotFoundOnPage {
                node_id: board_id,
                page_id: page.id.clone(),
            });
        };
        // Hiding a board is the author saying it is not part of the
        // deck — it costs no time on the timeline either.
        if node.hidden {
            continue;
        }
        let duration_ticks = hold_ticks(text_units(node));
        let markup = board_slide_markup(page, &board_id, board_name(state, &board_id))?;
        structured_nodes += markup.structured_nodes;
        raster_fallbacks += markup.raster_fallbacks();
        scenes.push(Scene {
            name: markup.name,
            start_ticks: cursor,
            duration_ticks,
            width: markup.width,
            height: markup.height,
            body: markup.body,
        });
        cursor += duration_ticks;
    }
    let (first_width, first_height) = match scenes.first() {
        Some(first) => (first.width, first.height),
        None => return Err(ExportError::NothingToExport),
    };

    let title = composition_title(state, &scenes);
    let html =
        markup::render_composition(&title, &slug(&title), first_width, first_height, &scenes);
    let render_notes = markup::render_notes(&title, cursor, scenes.len());
    Ok(Composition {
        html,
        render_notes,
        width: first_width,
        height: first_height,
        scenes: scenes.len(),
        total_ticks: cursor,
        structured_nodes,
        raster_fallbacks,
    })
}

/// Build the composition and write both files into `dir`.
///
/// The notes file ships beside the composition rather than inside it:
/// the HTML is an input to a renderer that would have to be taught to
/// ignore a comment block, and a `.md` next to it is what a human opens.
pub fn export_deck_hyperframes(
    state: &EditorState,
    dir: &Path,
) -> Result<HyperframesExport, ExportError> {
    let composition = deck_composition(state)?;
    std::fs::create_dir_all(dir).map_err(|e| ExportError::Write(e.to_string()))?;
    let composition_path = dir.join(COMPOSITION_FILE);
    let render_notes_path = dir.join(RENDER_NOTES_FILE);
    std::fs::write(&composition_path, &composition.html)
        .map_err(|e| ExportError::Write(e.to_string()))?;
    std::fs::write(&render_notes_path, &composition.render_notes)
        .map_err(|e| ExportError::Write(e.to_string()))?;
    Ok(HyperframesExport {
        composition_path,
        render_notes_path,
        composition,
    })
}

/// How long a slide carrying `units` of text is held, in ticks.
///
/// Rounding to a whole tick happens HERE, once, so that every later use
/// of the number — the attribute, the animation delay, the running
/// start — is the same integer. Rounding at the formatting step instead
/// would let a scene's printed start disagree with the sum of the
/// printed durations before it.
fn hold_ticks(units: f32) -> u32 {
    let seconds = (BASE_SECONDS + units / UNITS_PER_SECOND).clamp(MIN_SECONDS, MAX_SECONDS);
    (seconds * TICKS_PER_SECOND as f32).round() as u32
}

/// Text units under `node`, counting the visible subtree.
///
/// Hidden nodes are excluded for the same reason hidden boards are: a
/// paragraph the author turned off is not read on screen, so paying for
/// it in hold time would stretch the video against something nobody
/// sees.
fn text_units(node: &op_editor_ui::layout_scene::SceneNode) -> f32 {
    if node.hidden {
        return 0.0;
    }
    let own = node.text.as_deref().map(units_in).unwrap_or(0.0);
    node.children
        .iter()
        .fold(own, |total, child| total + text_units(child))
}

/// Reading cost of one string: CJK characters count one each, runs of
/// Latin/digit characters count as one word each.
fn units_in(text: &str) -> f32 {
    let mut cjk = 0u32;
    let mut words = 0u32;
    let mut inside_word = false;
    for c in text.chars() {
        if is_cjk(c) {
            cjk += 1;
            inside_word = false;
        } else if c.is_alphanumeric() {
            if !inside_word {
                words += 1;
                inside_word = true;
            }
        } else {
            inside_word = false;
        }
    }
    cjk as f32 + words as f32 * LATIN_WORD_UNITS
}

/// Whether `c` is read one character at a time rather than one word at
/// a time. The ranges cover CJK ideographs and their extensions, kana,
/// Hangul, CJK punctuation and the fullwidth forms — everything a deck
/// in an East Asian language is actually set in.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{2E80}'..='\u{9FFF}'
        | '\u{A960}'..='\u{A97F}'
        | '\u{AC00}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FE30}'..='\u{FE4F}'
        | '\u{FF00}'..='\u{FFEF}'
        | '\u{20000}'..='\u{3FFFF}')
}

/// Composition title: the document name, else the first scene's name
/// (which for a generated deck is the deck's own title), else a neutral
/// constant.
fn composition_title(state: &EditorState, scenes: &[Scene]) -> String {
    state
        .doc
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .or_else(|| scenes.first().map(|scene| scene.name.clone()))
        .unwrap_or_else(|| DEFAULT_ID.to_string())
}

/// An ASCII id for `data-composition-id`, derived from the title.
///
/// The attribute is an identifier a renderer may put in a log line or a
/// file name, so it is reduced to lowercase ASCII words joined by
/// hyphens. A title with no ASCII at all (a Chinese deck name, most
/// often) reduces to nothing, and falls back to the constant rather
/// than to an empty attribute.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        DEFAULT_ID.to_string()
    } else {
        trimmed.to_string()
    }
}

/// A tick count as CSS/attribute seconds: `42` → `"4.2"`, `30` → `"3"`.
///
/// Hand-formatted rather than routed through a float: the timeline is
/// integers precisely so that no printed number can round away from the
/// value the arithmetic used, and `{:.1}` on an `f32` would re-introduce
/// exactly that risk. Also guarantees the CSS-legal spelling — a decimal
/// POINT (never a locale comma) and never an exponent.
fn seconds(ticks: u32) -> String {
    let whole = ticks / TICKS_PER_SECOND;
    match ticks % TICKS_PER_SECOND {
        0 => whole.to_string(),
        fraction => format!("{whole}.{fraction}"),
    }
}

/// Scale + offset that fits a `w × h` board into the `cw × ch` video
/// canvas, as a `(scale, left, top)` triple.
///
/// A deck whose boards are all one size — the normal case — gets
/// `(1, 0, 0)` and no transform worth mentioning. A deck that mixes
/// sizes is letterboxed HERE, at export time, rather than by a script at
/// playback: the renderer captures frames, and a fit computed in Rust
/// cannot land differently on the frame the encoder is on.
fn fit(w: f32, h: f32, cw: f32, ch: f32) -> (f32, f32, f32) {
    if w <= 0.0 || h <= 0.0 || cw <= 0.0 || ch <= 0.0 {
        return (1.0, 0.0, 0.0);
    }
    let scale = (cw / w).min(ch / h);
    (scale, (cw - w * scale) / 2.0, (ch - h * scale) / 2.0)
}
