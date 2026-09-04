//! Responsive image source selection (`<picture>`, `srcset`, `sizes`).
//!
//! A static import has one viewport, so "responsive" collapses to "pick the
//! candidate a browser would pick at the import viewport". Density candidates
//! deliberately resolve to `1x`: the document stores CSS pixels, so a `2x`
//! asset would only inflate the embedded payload.

use crate::css::media_condition_applies;
use crate::dom::{DomElement, DomNode};
use crate::import_warning::ImportWarning;
use crate::length::{parse_length, LengthCtx};
use crate::mapper::MapCtx;
use std::borrow::Cow;

/// One `srcset` entry with its (optional) descriptor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Candidate<'a> {
    pub url: &'a str,
    /// `<n>w` — the candidate's intrinsic width in px.
    pub width: Option<f64>,
    /// `<n>x` — the candidate's pixel density.
    pub density: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedImageSource<'a> {
    pub url: Cow<'a, str>,
    /// Pixel density selected by `srcset`. Encoded pixel dimensions divide by
    /// this value to produce the browser's CSS-pixel natural dimensions.
    pub density: f64,
}

/// The URL an `<img>` should import, honouring an enclosing `<picture>`, its
/// own `srcset` / `sizes`, and finally plain `src`.
pub(crate) fn resolve_image_source(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    element: &DomElement,
) -> String {
    resolve_image_candidate(context, path, element)
        .url
        .into_owned()
}

pub(crate) fn resolve_image_candidate<'a>(
    context: &mut MapCtx<'_>,
    path: &[&'a DomElement],
    element: &'a DomElement,
) -> ResolvedImageSource<'a> {
    let viewport = (context.opts.viewport_width, context.opts.viewport_height());
    let fallback_width = element
        .attr("sizes")
        .and_then(|sizes| resolve_sizes(sizes, viewport, context))
        .unwrap_or(viewport.0);
    // The `<img>` inside a `<picture>` is the guaranteed-decodable fallback, so
    // whether it has one decides how hard the source walk tries.
    let img_fallback = element
        .attr("src")
        .is_some_and(|src| !src.trim().is_empty())
        || element
            .attr("srcset")
            .is_some_and(|srcset| !parse_srcset(srcset).is_empty());
    if let Some(source) = pick_from_picture(context, path, viewport, img_fallback) {
        return source;
    }
    if let Some(candidate) = element
        .attr("srcset")
        .and_then(|srcset| select(&parse_srcset(srcset), fallback_width).copied())
    {
        return resolve_candidate(candidate, fallback_width);
    }
    ResolvedImageSource {
        url: Cow::Borrowed(element.attr("src").unwrap_or_default()),
        density: 1.0,
    }
}

/// Walk the parent `<picture>`'s `<source>` list in document order.
///
/// The first pass honours `type`, which is what a browser does. A second pass
/// that ignores `type` runs ONLY when the `<img>` has no usable fallback:
/// importing an undecodable format still beats importing nothing, but it must
/// never win over a `src` this importer is certain it can read — a browser
/// without AVIF support paints the `<img>`, and so should this.
fn pick_from_picture<'a>(
    context: &mut MapCtx<'_>,
    path: &[&'a DomElement],
    viewport: (f64, f64),
    img_fallback: bool,
) -> Option<ResolvedImageSource<'a>> {
    let picture = path
        .get(path.len().checked_sub(2)?)
        .filter(|parent| parent.tag == "picture")?;
    let passes: &[bool] = if img_fallback {
        &[true]
    } else {
        &[true, false]
    };
    for honour_type in passes.iter().copied() {
        for source in picture.children.iter().filter_map(|child| match child {
            DomNode::Element(element) if element.tag == "source" => Some(element),
            _ => None,
        }) {
            if source
                .attr("media")
                .is_some_and(|media| !media_applies(context, media, viewport))
            {
                continue;
            }
            if honour_type && source.attr("type").is_some_and(|mime| !is_decodable(mime)) {
                continue;
            }
            // `src` is ignored on a `<source>` inside `<picture>`; only
            // `srcset` names candidates there.
            let Some(srcset) = source.attr("srcset") else {
                continue;
            };
            // A `<source>` without `sizes` defaults to `100vw`. It does NOT
            // inherit the `<img>`'s `sizes`, which is a separate element's
            // attribute.
            let target = source
                .attr("sizes")
                .and_then(|sizes| resolve_sizes(sizes, viewport, context))
                .unwrap_or(viewport.0);
            let Some(candidate) = select(&parse_srcset(srcset), target).copied() else {
                continue;
            };
            if !honour_type {
                context.warn_once(ImportWarning::PictureUndecodableTypes);
            }
            return Some(resolve_candidate(candidate, target));
        }
    }
    None
}

fn resolve_candidate(candidate: Candidate<'_>, source_size: f64) -> ResolvedImageSource<'_> {
    let density = candidate
        .width
        .map(|width| width / source_size)
        .or(candidate.density)
        .filter(|density| density.is_finite() && *density > 0.0)
        .unwrap_or(1.0);
    ResolvedImageSource {
        url: Cow::Borrowed(candidate.url),
        density,
    }
}

/// Evaluate a media condition and surface whatever the evaluator could not
/// parse, instead of silently treating an unreadable query as "does not match".
fn media_applies(context: &mut MapCtx<'_>, condition: &str, viewport: (f64, f64)) -> bool {
    let (applies, warnings) = media_condition_applies(condition, viewport);
    for warning in warnings {
        context.warn_once(ImportWarning::MediaQuery(warning));
    }
    applies
}

/// Image MIME types the resource embedder and renderers handle.
fn is_decodable(mime: &str) -> bool {
    let essence = mime.split(';').next().unwrap_or_default().trim();
    essence.len() <= 128
        && essence.is_ascii()
        && [
            "image/png",
            "image/apng",
            "image/jpeg",
            "image/jpg",
            "image/pjpeg",
            "image/gif",
            "image/webp",
            "image/bmp",
            "image/svg+xml",
            "image/x-icon",
            "image/vnd.microsoft.icon",
            "",
        ]
        .into_iter()
        .any(|supported| essence.eq_ignore_ascii_case(supported))
}

/// Parse a `srcset` attribute.
///
/// Follows the HTML "parse a srcset attribute" shape closely enough for real
/// markup: a candidate URL is a run of non-space characters (so a `data:` URL
/// keeps its commas) and its descriptors run to the next top-level comma.
pub(crate) fn parse_srcset(value: &str) -> Vec<Candidate<'_>> {
    const MAX_CANDIDATES: usize = 256;
    let mut candidates = Vec::new();
    let mut at = 0usize;
    while at < value.len() && candidates.len() < MAX_CANDIDATES {
        while let Some(character) = value[at..].chars().next() {
            if !character.is_whitespace() && character != ',' {
                break;
            }
            at += character.len_utf8();
            if at == value.len() {
                break;
            }
        }
        let start = at;
        while at < value.len() {
            let character = value[at..].chars().next().expect("character boundary");
            if character.is_whitespace() {
                break;
            }
            at += character.len_utf8();
        }
        if at == start {
            break;
        }
        let raw = &value[start..at];
        let terminated = raw.ends_with(',');
        let url = raw.trim_end_matches(',');
        let descriptor_start = at;
        if !terminated {
            let mut depth = 0usize;
            while at < value.len() {
                let character = value[at..].chars().next().expect("character boundary");
                match character {
                    '(' => depth += 1,
                    ')' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => {
                        break;
                    }
                    _ => {}
                }
                at += character.len_utf8();
            }
        }
        if url.is_empty() {
            if at < value.len() {
                at += 1;
            }
            continue;
        }
        let (width, density) = parse_descriptors(&value[descriptor_start..at]);
        candidates.push(Candidate {
            url,
            width,
            density,
        });
        if at < value.len() && value.as_bytes()[at] == b',' {
            at += 1;
        }
    }
    candidates
}

fn parse_descriptors(value: &str) -> (Option<f64>, Option<f64>) {
    let mut width = None;
    let mut density = None;
    for token in value.split_whitespace() {
        if let Some(number) = token
            .strip_suffix('w')
            .or_else(|| token.strip_suffix('W'))
            .and_then(|number| number.parse::<f64>().ok())
        {
            if number > 0.0 {
                width = Some(number);
            }
        } else if let Some(number) = token
            .strip_suffix('x')
            .or_else(|| token.strip_suffix('X'))
            .and_then(|number| number.parse::<f64>().ok())
        {
            if number > 0.0 {
                density = Some(number);
            }
        }
    }
    (width, density)
}

/// Choose the candidate a browser would use at `target_width` CSS px.
///
/// Width descriptors win when present: the narrowest candidate that still
/// covers the slot, or the widest available when none does. Otherwise the
/// density closest to `1x` is taken, ties going to the lower density.
pub(crate) fn select<'slice, 'source>(
    candidates: &'slice [Candidate<'source>],
    target_width: f64,
) -> Option<&'slice Candidate<'source>> {
    if candidates.iter().any(|candidate| candidate.width.is_some()) {
        return candidates
            .iter()
            .filter(|candidate| candidate.width.is_some())
            .reduce(|best, candidate| {
                let (best_width, width) = (
                    best.width.unwrap_or(f64::MAX),
                    candidate.width.unwrap_or(f64::MAX),
                );
                let take = match (best_width >= target_width, width >= target_width) {
                    (false, true) => true,
                    (true, true) => width < best_width,
                    (false, false) => width > best_width,
                    (true, false) => false,
                };
                if take {
                    candidate
                } else {
                    best
                }
            });
    }
    candidates.iter().reduce(|best, candidate| {
        let (best_density, density) = (
            best.density.unwrap_or(1.0),
            candidate.density.unwrap_or(1.0),
        );
        let closer = (density - 1.0).abs() < (best_density - 1.0).abs();
        let tie = (density - 1.0).abs() == (best_density - 1.0).abs() && density < best_density;
        if closer || tie {
            candidate
        } else {
            best
        }
    })
}

/// Resolve a `sizes` attribute to the slot width in CSS px at this viewport.
pub(crate) fn resolve_sizes(
    value: &str,
    viewport: (f64, f64),
    context: &mut MapCtx<'_>,
) -> Option<f64> {
    for entry in split_top_level(value) {
        let (condition, size) = split_source_size(entry.trim());
        if condition.is_some_and(|condition| !media_applies(context, condition, viewport)) {
            continue;
        }
        let length = parse_length(
            size,
            &LengthCtx {
                font_size: context.opts.base_font_size,
                root_font_size: context.opts.base_font_size,
                viewport_w: viewport.0,
                viewport_h: viewport.1,
            },
        );
        if let Some(resolved) = length
            .map(|length| length.resolve(viewport.0))
            .filter(|value| value.is_finite() && *value > 0.0)
        {
            return Some(resolved);
        }
    }
    None
}

/// Split one `<source-size>` into its optional media condition and its length.
///
/// A condition is a run of parenthesised groups joined by `and` / `or` / `not`;
/// everything after the last such group is the length. `calc(…)` sizes are
/// therefore not mistaken for a condition, because they never start the entry
/// with a group followed by more content.
fn split_source_size(entry: &str) -> (Option<&str>, &str) {
    let mut at = 0usize;
    let mut condition_end = 0usize;
    loop {
        let rest = entry[at..].trim_start();
        let offset = entry.len() - rest.len();
        if !rest.starts_with('(') {
            break;
        }
        let Some(close) = matching_paren(entry, offset) else {
            break;
        };
        condition_end = close + 1;
        at = close + 1;
        let next = entry[at..].trim_start();
        let keyword = next
            .split(|character: char| !character.is_ascii_alphabetic())
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(keyword.as_str(), "and" | "or" | "not") {
            at += entry[at..].len() - next.len() + keyword.len();
            continue;
        }
        break;
    }
    if condition_end == 0 || entry[condition_end..].trim().is_empty() {
        return (None, entry);
    }
    (
        Some(entry[..condition_end].trim()),
        entry[condition_end..].trim(),
    )
}

fn matching_paren(value: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in value.char_indices().skip_while(|(index, _)| *index < open) {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a comma list, keeping commas nested in `(` … `)` together.
fn split_top_level(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

#[cfg(test)]
#[path = "srcset_tests.rs"]
mod tests;
