//! Opt-in per-stage trace of the image-search ladder.
//!
//! Shipped image nodes keep only `op-image:<hash>`, so once a wrong photo
//! lands nobody can tell which provider stage produced it. With
//! `OPENPENCIL_IMAGE_SEARCH_TRACE=1` (or [`force_search_trace`] from a live
//! test) every stage prints the provider hits it saw (title + tags), the
//! relevance-fence verdict for each, and the hit finally claimed. Thumbnail
//! URLs are never printed — only catalogue titles and tags.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::net::providers::{fence_rejection, RawHit};

/// Environment switch for the trace (`1`/`true`/`on`).
pub const SEARCH_TRACE_ENV: &str = "OPENPENCIL_IMAGE_SEARCH_TRACE";

static FORCED: AtomicBool = AtomicBool::new(false);

/// Enable the trace programmatically (live diagnostic tests), independent of
/// the environment.
pub fn force_search_trace(enabled: bool) {
    FORCED.store(enabled, Ordering::Relaxed);
}

pub fn search_trace_enabled() -> bool {
    FORCED.load(Ordering::Relaxed)
        || std::env::var(SEARCH_TRACE_ENV).is_ok_and(|value| {
            let value = value.trim();
            value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("on")
        })
}

/// One free-form trace line.
pub(crate) fn trace_line(message: impl std::fmt::Display) {
    if search_trace_enabled() {
        eprintln!("[IMAGE-TRACE] {message}");
    }
}

/// Print one provider answer: every hit with its tags and the fence verdict
/// against the authored `query` + `intent`.
pub(crate) fn trace_hits(stage: &str, provider_query: &str, fence: (&str, &str), hits: &[RawHit]) {
    if !search_trace_enabled() {
        return;
    }
    let (query, intent) = fence;
    eprintln!(
        "[IMAGE-TRACE] {stage} q=\"{provider_query}\": {} hit(s)",
        hits.len()
    );
    for (index, hit) in hits.iter().enumerate() {
        let verdict = fence_rejection(hit, query, intent).unwrap_or("kept");
        eprintln!(
            "[IMAGE-TRACE]   #{} \"{}\" tags=[{}] -> {verdict}",
            index + 1,
            short_title(&hit.title, 90),
            short_title(hit_tags(hit), 110),
        );
    }
}

/// Tags are stored after the title inside `relevance_metadata`.
fn hit_tags(hit: &RawHit) -> &str {
    hit.relevance_metadata
        .strip_prefix(hit.title.as_str())
        .unwrap_or(&hit.relevance_metadata)
        .trim()
}

/// A single-line, length-capped, quote-free rendering of a provider title
/// (captions can carry newlines, quotes and hundreds of characters).
pub fn short_title(value: &str, max_chars: usize) -> String {
    let flat: String = value
        .chars()
        .map(|ch| match ch {
            '"' => '\'',
            ch if ch.is_control() => ' ',
            ch => ch,
        })
        .collect();
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max_chars {
        return flat;
    }
    let mut cut: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::short_title;

    #[test]
    fn short_title_flattens_quotes_newlines_and_caps_length() {
        assert_eq!(short_title("a \"b\"\nc", 20), "a 'b' c");
        assert_eq!(short_title("abcdefghij", 5), "abcd…");
    }
}
