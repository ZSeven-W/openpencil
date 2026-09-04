//! Download filename sanitisation.
//!
//! The title comes from the captured page, so it is attacker-controlled and
//! must not be able to steer where `chrome.downloads` writes. Path separators
//! are removed first; then dot runs are collapsed to a single dot and leading
//! dots dropped, so no title can produce a `..` component (which
//! `chrome.downloads` rejects outright, turning a capture into an opaque
//! failure) or a dot-file. The result stays readable — a title only loses
//! characters it could not have carried anyway.
//!
//! The sanitisation chain, step for step:
//!
//! ```text
//! .replace(/[\p{Cc}<>:"/\\|?*%]+/gu, ' ')
//! .replace(/\s+/g, ' ')
//! .replace(/\.{2,}/g, '.')
//! .replace(/^[\s.]+/, '')
//! .trim()
//! .slice(0, 80)
//! .replace(/[\s.]+$/, '')
//! ```
//!
//! The offline download is a ready-to-open `.op` document (OpenPencil's
//! `PenDocument` format), so the sanitised stem carries the `.op` suffix and
//! the file opens by double-click without any CLI step.

use crate::js_text::{is_js_space, js_trim, truncate_utf16};

/// The design-system download has a stable conventional name so it can be
/// imported by any tool that recognises the `design.md` convention.
pub const DESIGN_MD_FILENAME: &str = "design.md";

/// Download name for an extracted design-system guide.
pub fn design_md_filename() -> &'static str {
    DESIGN_MD_FILENAME
}

/// Longest title stem kept, in UTF-16 code units (JS `slice(0, 80)`).
const MAX_STEM_UNITS: usize = 80;

/// Suffix every `.op` download carries.
const SUFFIX: &str = ".op";

/// Stem used when the title sanitises down to nothing.
const FALLBACK_STEM: &str = "page";

/// Characters no host OS accepts in a file name, beyond the `Cc` control
/// range. `%` is included because a percent escape in a name reaching
/// `chrome.downloads` is a needless second decoding layer.
const FORBIDDEN: [char; 10] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '%'];

/// Build the `.op` download file name for a captured page titled `title`.
pub fn op_filename(title: &str) -> String {
    let replaced = replace_forbidden_runs(title);
    let collapsed = collapse_space_runs(&replaced);
    let dotted = collapse_dot_runs(&collapsed);
    let stem = js_trim(dotted.trim_start_matches(|c| is_js_space(c) || c == '.'));
    // The truncation can leave a fresh trailing dot or space, which Windows
    // silently drops from a file name.
    let stem =
        truncate_utf16(stem, MAX_STEM_UNITS).trim_end_matches(|c| is_js_space(c) || c == '.');
    let stem = if stem.is_empty() { FALLBACK_STEM } else { stem };
    format!("{stem}{SUFFIX}")
}

/// `/[\p{Cc}<>:"/\\|?*%]+/gu → ' '`. `char::is_control` is exactly the
/// `Cc` general category the JS class named.
fn replace_forbidden_runs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_run = false;
    for c in s.chars() {
        if c.is_control() || FORBIDDEN.contains(&c) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// `/\s+/g → ' '`.
fn collapse_space_runs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_run = false;
    for c in s.chars() {
        if is_js_space(c) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// `/\.{2,}/g → '.'` — runs of two or more dots only; a single dot survives.
fn collapse_dot_runs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dots = 0usize;
    for c in s.chars() {
        if c == '.' {
            dots += 1;
            continue;
        }
        // A run of any length collapses to exactly one dot; a lone dot is
        // already exactly one.
        if dots > 0 {
            out.push('.');
            dots = 0;
        }
        out.push(c);
    }
    if dots > 0 {
        out.push('.');
    }
    out
}
