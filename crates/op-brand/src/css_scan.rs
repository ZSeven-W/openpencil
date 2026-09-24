//! A tolerant stylesheet walker that keeps the one piece of context the
//! `op-html` cascade deliberately discards: which colour scheme a rule
//! belongs to. `@media (prefers-color-scheme: dark)` blocks are dropped by
//! a viewport-less cascade, but for a brand kit they ARE the dark theme.
//!
//! The walker only splits rules and at-blocks; declaration parsing,
//! shorthand expansion, and selector parsing reuse `op-html`.

/// Which colour scheme a rule applies under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scheme {
    /// Unconditional (or only width-conditional) — the page as served.
    Base,
    /// Inside `@media (prefers-color-scheme: dark)`.
    Dark,
    /// Inside `@media (prefers-color-scheme: light)`.
    Light,
}

/// One style rule: its raw selector list and raw declaration block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawRule {
    pub scheme: Scheme,
    pub selectors: String,
    pub block: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ScanOutput {
    pub rules: Vec<RawRule>,
    /// `@import` targets as written (unresolved).
    pub imports: Vec<String>,
    /// `@font-face` declaration blocks.
    pub font_faces: Vec<String>,
}

/// Rules beyond this are ignored — a brand lives in the first few
/// thousand rules, and the cap bounds work on hostile input.
const MAX_RULES: usize = 40_000;
const MAX_DEPTH: usize = 12;

/// Walk `css` and append its rules to `out`.
pub(crate) fn scan_stylesheet(css: &str, out: &mut ScanOutput) {
    let cleaned = strip_comments(css);
    walk(&cleaned, Scheme::Base, 0, out);
}

fn walk(css: &str, scheme: Scheme, depth: usize, out: &mut ScanOutput) {
    if depth > MAX_DEPTH {
        return;
    }
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() && out.rules.len() < MAX_RULES {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'}') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let (stop, found) = find_stop(css, i);
        let prelude = css[i..stop].trim();
        match found {
            Stop::Semicolon | Stop::End => {
                if let Some(target) = import_target(prelude) {
                    out.imports.push(target);
                }
                i = stop + 1;
            }
            Stop::Brace => {
                let Some(end) = matching_brace(css, stop) else {
                    break;
                };
                let body = &css[stop + 1..end];
                if let Some(at) = prelude.strip_prefix('@') {
                    let name_end = at
                        .find(|c: char| c.is_whitespace() || c == '(')
                        .unwrap_or(at.len());
                    let name = at[..name_end].to_ascii_lowercase();
                    let condition = &at[name_end..];
                    match name.as_str() {
                        "media" => {
                            if let Some(inner) = media_scheme(condition, scheme) {
                                walk(body, inner, depth + 1, out);
                            }
                        }
                        "supports" | "layer" | "container" | "document" | "scope" => {
                            walk(body, scheme, depth + 1, out)
                        }
                        "font-face" => out.font_faces.push(body.to_string()),
                        _ => {}
                    }
                } else if !prelude.is_empty() {
                    out.rules.push(RawRule {
                        scheme,
                        selectors: prelude.to_string(),
                        block: body.to_string(),
                    });
                }
                i = end + 1;
            }
        }
    }
}

/// The scheme a `@media` block's rules run under, or `None` when the block
/// never applies to a screen (print-only).
fn media_scheme(condition: &str, outer: Scheme) -> Option<Scheme> {
    let squashed: String = condition
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if squashed.contains("print") && !squashed.contains("screen") {
        return None;
    }
    if squashed.contains("prefers-color-scheme:dark") {
        return Some(Scheme::Dark);
    }
    if squashed.contains("prefers-color-scheme:light") {
        return Some(Scheme::Light);
    }
    Some(outer)
}

fn import_target(prelude: &str) -> Option<String> {
    let rest = prelude.strip_prefix("@import")?.trim();
    let rest = rest.strip_prefix("url(").map_or(rest, |r| r);
    let quote = rest.chars().next()?;
    let target = if quote == '"' || quote == '\'' {
        let inner = &rest[1..];
        &inner[..inner.find(quote)?]
    } else {
        let end = rest
            .find(|c: char| c == ')' || c.is_whitespace() || c == ';')
            .unwrap_or(rest.len());
        &rest[..end]
    };
    let target = target.trim();
    (!target.is_empty()).then(|| target.to_string())
}

enum Stop {
    Brace,
    Semicolon,
    End,
}

/// First top-level `{` or `;` at or after `from` (strings and parens are
/// skipped so `url("a;b")` or `:is(a, b)` never split a prelude).
fn find_stop(css: &str, from: usize) -> (usize, Stop) {
    let bytes = css.as_bytes();
    let mut i = from;
    let mut parens = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = skip_string(bytes, i),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'{' if parens == 0 => return (i, Stop::Brace),
            b';' if parens == 0 => return (i, Stop::Semicolon),
            _ => {}
        }
        i += 1;
    }
    (bytes.len(), Stop::End)
}

/// Index of the `}` closing the `{` at `open`.
fn matching_brace(css: &str, open: usize) -> Option<usize> {
    let bytes = css.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = skip_string(bytes, i),
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Index of the closing quote of the string starting at `start` (or the
/// last byte when unterminated).
fn skip_string(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b if b == quote => return i,
            _ => {}
        }
        i += 1;
    }
    bytes.len().saturating_sub(1)
}

/// Remove `/* … */` comments outside strings.
fn strip_comments(css: &str) -> String {
    let bytes = css.as_bytes();
    let mut out = String::with_capacity(css.len());
    let mut i = 0;
    let mut copied = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = skip_string(bytes, i) + 1,
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                out.push_str(&css[copied..i]);
                let end = css[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |p| i + 2 + p + 2);
                out.push(' ');
                i = end;
                copied = end;
            }
            _ => i += 1,
        }
    }
    if copied < css.len() {
        out.push_str(&css[copied..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(css: &str) -> ScanOutput {
        let mut out = ScanOutput::default();
        scan_stylesheet(css, &mut out);
        out
    }

    #[test]
    fn splits_rules_and_tracks_dark_media_blocks() {
        let out = scan(
            r#"
            /* brand */ :root { --primary: #6d28d9; }
            @media (prefers-color-scheme: dark) { :root { --primary: #a78bfa } }
            @media print { body { color: red } }
            @media (min-width: 640px) { .btn { border-radius: 6px } }
            "#,
        );
        let schemes: Vec<_> = out
            .rules
            .iter()
            .map(|r| (r.scheme, r.selectors.as_str()))
            .collect();
        assert_eq!(
            schemes,
            vec![
                (Scheme::Base, ":root"),
                (Scheme::Dark, ":root"),
                (Scheme::Base, ".btn"),
            ]
        );
    }

    #[test]
    fn records_imports_and_font_faces() {
        let out = scan(
            "@import url('fonts.css'); @import \"theme.css\" screen;\n\
             @font-face { font-family: 'Brandline'; src: url(a.woff2) }",
        );
        assert_eq!(out.imports, vec!["fonts.css", "theme.css"]);
        assert_eq!(out.font_faces.len(), 1);
    }

    #[test]
    fn strings_and_parens_never_split_a_rule() {
        let out = scan(r#"a[title="x{y"] , :is(.a;.b) { content: "}" ; color: #111 }"#);
        assert_eq!(out.rules.len(), 1);
        assert!(out.rules[0].block.contains("#111"));
    }

    #[test]
    fn unterminated_input_does_not_panic() {
        let out = scan(":root { --a: 1px; .x { color: red");
        assert!(out.rules.is_empty());
        let _ = scan("/* open comment");
        let _ = scan("\"open string");
    }
}
