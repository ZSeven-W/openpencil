//! SVG document scanning: root `<svg>` extraction, viewBox-aware
//! root scale + inherited style context, and the recursive element
//! tree walk.

use super::*;

/// Pull the root `<svg …>` open tag from a document. Returns the
/// body between `<svg …>` and `</svg>` plus the parsed root attrs.
/// `None` when the document lacks a balanced `<svg>` element — fed
/// to the regex-equivalent walker the way `parseSvgRegex` does in
/// the TS port.
pub(super) fn extract_svg_root(svg: &str) -> Option<(&str, Vec<(String, String)>)> {
    let lower: String = svg.chars().map(|c| c.to_ascii_lowercase()).collect();
    let open = lower.find("<svg")?;
    let after_open = open + 4;
    let bytes = svg.as_bytes();
    let close_of_open = find_tag_end(bytes, after_open)?;
    let body_start = close_of_open + 1;
    // Closing tag is the last `</svg>` in the document.
    let close_marker = lower.rfind("</svg>")?;
    if close_marker < body_start {
        return None;
    }
    let attrs_str = &svg[after_open..close_of_open].trim_end_matches('/');
    let attrs = parse_attrs(attrs_str);
    Some((&svg[body_start..close_marker], attrs))
}

/// Compute the viewBox-aware scale factor + seed style context. The
/// TS port caps the longer side at `maxDim` (400 px) and scales
/// every coord uniformly so an icon authored in a 24-unit viewBox
/// lands at 400 px instead of 24 px on the canvas.
pub(super) fn compute_root_scale(root_attrs: &[(String, String)]) -> (f64, StyleCtx) {
    let attr = |key: &str| -> Option<&str> {
        root_attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let view_box = attr("viewBox").or_else(|| attr("viewbox"));
    let mut vb_w = 100.0_f64;
    let mut vb_h = 100.0_f64;
    if let Some(vb) = view_box {
        let nums: Vec<f64> = vb
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if nums.len() >= 4 {
            vb_w = nums[2].max(0.001);
            vb_h = nums[3].max(0.001);
        }
    }
    let parse_dim = |raw: &str| -> Option<f64> {
        // Strip a trailing `px` so `width="24px"` round-trips; bail
        // on `%` / `em` / `vh` since they need parent context.
        let trimmed = raw.trim().trim_end_matches("px");
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.chars().any(|c| !c.is_ascii_digit() && c != '.') {
            return None;
        }
        trimmed.parse::<f64>().ok()
    };
    let svg_w = attr("width").and_then(parse_dim).unwrap_or(vb_w);
    let svg_h = attr("height").and_then(parse_dim).unwrap_or(vb_h);
    let mut out_w = svg_w;
    let mut out_h = svg_h;
    if out_w > SVG_MAX_DIM || out_h > SVG_MAX_DIM {
        let s = SVG_MAX_DIM / out_w.max(out_h);
        out_w *= s;
        out_h *= s;
    }
    // Children scale by `out / vb` — matches the TS impl exactly.
    let scale = (out_w / vb_w).min(out_h / vb_h).max(0.001);
    let ctx = merge_style_ctx(&StyleCtx::default(), root_attrs);
    (scale, ctx)
}

/// Recursive tree walker — depth-tracking version of
/// `parse_svg_elements`. Each opening tag pairs with its matching
/// `</tag>` so `<g>` children land under the right parent. Skip
/// tags (`defs` / `style` / …) are filtered out before recursion.
pub(super) fn parse_svg_tree(body: &str) -> Vec<SvgTree> {
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Skip comments.
        if body[i..].starts_with("<!--") {
            match body[i..].find("-->") {
                Some(rel) => i += rel + 3,
                None => break,
            }
            continue;
        }
        // Prolog / DOCTYPE / stray closing tag — advance to `>`.
        if matches!(bytes.get(i + 1), Some(b'/') | Some(b'?') | Some(b'!')) {
            match body[i..].find('>') {
                Some(rel) => i += rel + 1,
                None => break,
            }
            continue;
        }
        let Some(open_end) = find_tag_end(bytes, i + 1) else {
            break;
        };
        let inner = &body[i + 1..open_end];
        let self_closing = inner.trim_end().ends_with('/');
        let (tag_lower, attrs) = match parse_element(inner) {
            Some(el) => (el.tag, el.attrs),
            None => {
                i = open_end + 1;
                continue;
            }
        };
        if SKIP_TAGS.iter().any(|t| *t == tag_lower) {
            // Skip its body too so child shapes inside `<defs>` don't
            // leak into the import.
            i = if self_closing {
                open_end + 1
            } else {
                skip_until_closing(body, open_end + 1, &tag_lower).unwrap_or(body.len())
            };
            continue;
        }
        if self_closing {
            out.push(SvgTree {
                tag: tag_lower,
                attrs,
                children: Vec::new(),
            });
            i = open_end + 1;
            continue;
        }
        let body_start = open_end + 1;
        let body_end = find_matching_close(body, body_start, &tag_lower).unwrap_or(body.len());
        let inner_body = &body[body_start..body_end];
        out.push(SvgTree {
            tag: tag_lower,
            attrs,
            children: parse_svg_tree(inner_body),
        });
        // Advance past the close tag itself.
        i = match body[body_end..].find('>') {
            Some(rel) => body_end + rel + 1,
            None => body.len(),
        };
    }
    out
}

/// Index just past the `</tag>` that closes the given open tag at
/// `from`. Handles nested same-name tags so a `<g>` inside a `<g>`
/// pairs with its own close. Returns `None` when no matching close
/// exists (malformed SVG); the caller treats the rest of the body
/// as the element's content.
fn find_matching_close(body: &str, from: usize, tag: &str) -> Option<usize> {
    let bytes = body.as_bytes();
    let mut depth = 1usize;
    let mut i = from;
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}");
    while i < bytes.len() {
        let rest = &body[i..];
        let lower = rest.to_ascii_lowercase();
        let next_open = lower.find(&open_pat);
        let next_close = lower.find(&close_pat);
        let (idx, is_close) = match (next_open, next_close) {
            (None, None) => return None,
            (Some(o), None) => (o, false),
            (None, Some(c)) => (c, true),
            (Some(o), Some(c)) => {
                if o < c {
                    (o, false)
                } else {
                    (c, true)
                }
            }
        };
        // Confirm the match is followed by `>` or whitespace (so
        // `<rectangle>` doesn't false-match a `<rect>` close).
        let abs = i + idx;
        let after = abs
            + if is_close {
                close_pat.len()
            } else {
                open_pat.len()
            };
        let next_char = body.as_bytes().get(after).copied();
        let valid = matches!(
            next_char,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | Some(b'/')
        );
        if !valid {
            i = abs + 1;
            continue;
        }
        if is_close {
            depth -= 1;
            if depth == 0 {
                return Some(abs);
            }
            i = abs + close_pat.len();
        } else {
            depth += 1;
            i = abs + open_pat.len();
        }
    }
    None
}

/// Skip to just past `</tag>` for a skip-tag's body. Returns the
/// index after the closing tag's `>`.
fn skip_until_closing(body: &str, from: usize, tag: &str) -> Option<usize> {
    let close = find_matching_close(body, from, tag)?;
    body[close..].find('>').map(|rel| close + rel + 1)
}
