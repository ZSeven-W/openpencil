//! Pure W3C `CompositionEvent` → `jian_core::gesture::ImeEvent` mapping.
//!
//! Spec §2.4 invariant: `ImeEvent.text` is UTF-8 (Rust `String`) and
//! `ImeEvent.kind == CompositionUpdate { selection }`'s selection
//! Range is in UTF-8 byte offsets. Browsers expose the in-flight
//! preedit as `CompositionEvent.data` (a JS UTF-16 string) and the
//! IME-highlighted segment via `CompositionEvent.getTargetRanges()`
//! (UTF-16 code-unit offsets where supported). This module is the
//! shim that converts both.
//!
//! Pure: no DOM access. Phase C2 reads `CompositionEvent.data` /
//! `getTargetRanges()` and calls into here with already-converted
//! Rust strings + UTF-16 code-unit ranges.

use op_editor_ui::{ImeEvent, ImeKind};
use std::ops::Range;

/// Convert a UTF-16 code-unit selection range to UTF-8 byte offsets
/// within `text`. Walks `text.char_indices()` accumulating
/// `len_utf16()` so we know the UTF-16 prefix length at each UTF-8
/// boundary; sets `start` / `end` as soon as the running prefix
/// reaches `sel.start` / `sel.end`.
///
/// Out-of-range selection bounds clamp to `text.len()` (the byte
/// length of the full UTF-8 string). The caller may treat that as a
/// "select all the way to the end" hint or as a sentinel value.
pub fn utf16_selection_to_utf8(
    text: &str,
    sel_utf16: Option<Range<usize>>,
) -> Option<Range<usize>> {
    let sel = sel_utf16?;
    if sel.end < sel.start {
        // Mis-ordered range — treat as no selection rather than
        // panicking on the caller's behalf.
        return None;
    }
    let mut utf16_pos = 0usize;
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    for (byte_idx, ch) in text.char_indices() {
        if start.is_none() && utf16_pos >= sel.start {
            start = Some(byte_idx);
        }
        if end.is_none() && utf16_pos >= sel.end {
            end = Some(byte_idx);
            break;
        }
        utf16_pos += ch.len_utf16();
    }
    let s = start.unwrap_or(text.len());
    let e = end.unwrap_or(text.len());
    Some(s..e)
}

/// `compositionstart`: text is empty per spec §2.4.
pub fn composition_start() -> ImeEvent {
    ImeEvent {
        kind: ImeKind::CompositionStart,
        text: String::new(),
    }
}

/// `compositionupdate`: `text` is the preedit so far (UTF-8 already
/// converted by the caller). `sel_utf16` is the IME-highlighted
/// segment in UTF-16 code units (whatever `getTargetRanges()` gave
/// us); we remap to UTF-8 byte offsets so widget code never has to
/// know about UTF-16.
pub fn composition_update(text_utf8: String, sel_utf16: Option<Range<usize>>) -> ImeEvent {
    let selection = utf16_selection_to_utf8(&text_utf8, sel_utf16);
    ImeEvent {
        kind: ImeKind::CompositionUpdate { selection },
        text: text_utf8,
    }
}

/// `compositionend`: `text` is the final commit string (UTF-8). No
/// selection — the IME is finished.
pub fn composition_end(text_utf8: String) -> ImeEvent {
    ImeEvent {
        kind: ImeKind::CompositionEnd,
        text: text_utf8,
    }
}
