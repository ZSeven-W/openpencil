//! Lightweight markdown parser for the Design-MD panel.
//!
//! Ported from the TS `design-md-editor.tsx` `MdText` / `renderInline`
//! pair: block-level structure (H3 / H4 headings, bullet lists,
//! paragraphs) plus inline styling (`**bold**`, `` `code` ``, `#HEX`
//! colour chips). The inline styling is the panel's "syntax
//! highlighting" — styled text runs the panel paints in distinct
//! colours / weights.

/// One block of rendered markdown.
pub(super) enum MdBlock {
    /// A `### ` (level 3) or `#### ` (level 4) heading.
    Heading { level: u8, text: String },
    /// One `- ` / `* ` list item — its inline markdown source.
    Bullet(String),
    /// A paragraph — its inline markdown source, newlines flattened.
    Paragraph(String),
}

/// One inline-styled run within a line.
#[derive(Clone)]
pub(super) enum MdRun {
    /// Plain text.
    Plain(String),
    /// `**bold**` text.
    Bold(String),
    /// `` `code` `` span.
    Code(String),
    /// A `#RRGGBB` colour token — paints a swatch + the hex.
    Color(String),
}

impl MdRun {
    /// The run's visible text (for width / wrap accounting).
    fn text(&self) -> &str {
        match self {
            MdRun::Plain(s) | MdRun::Bold(s) | MdRun::Code(s) | MdRun::Color(s) => s,
        }
    }

    /// Whether this run may be split across wrapped lines at spaces.
    fn breakable(&self) -> bool {
        matches!(self, MdRun::Plain(_) | MdRun::Bold(_))
    }
}

/// Parse a markdown content string into renderable blocks. Content
/// longer than `limit` chars is truncated with an ellipsis (mirrors
/// the TS `MdText` `limit` prop).
pub(super) fn parse_blocks(content: &str, limit: usize) -> Vec<MdBlock> {
    let trimmed = if content.chars().count() > limit {
        let kept: String = content.chars().take(limit).collect();
        format!("{kept}...")
    } else {
        content.to_string()
    };

    let mut blocks = Vec::new();
    for raw in split_paragraphs(&trimmed) {
        let block = raw.trim();
        if block.is_empty() {
            continue;
        }
        if let Some(rest) = block.strip_prefix("### ") {
            blocks.push(MdBlock::Heading {
                level: 3,
                text: rest.trim().to_string(),
            });
            continue;
        }
        if let Some(rest) = block.strip_prefix("#### ") {
            blocks.push(MdBlock::Heading {
                level: 4,
                text: rest.trim().to_string(),
            });
            continue;
        }
        // A block whose every non-blank line is a `- ` / `* ` item.
        let lines: Vec<&str> = block.lines().collect();
        let is_list = lines.iter().all(|l| l.trim().is_empty() || is_bullet(l));
        if is_list && lines.iter().any(|l| is_bullet(l)) {
            for line in lines.iter().filter(|l| is_bullet(l)) {
                blocks.push(MdBlock::Bullet(strip_bullet(line)));
            }
            continue;
        }
        // Paragraph — flatten internal newlines to spaces.
        blocks.push(MdBlock::Paragraph(
            block.lines().collect::<Vec<_>>().join(" "),
        ));
    }
    blocks
}

/// Split on runs of 2+ newlines (paragraph breaks).
fn split_paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut blank_run = 0;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run >= 1 && !current.is_empty() {
                // Defer the flush — only a 2nd blank ends the block.
            }
        } else {
            if blank_run >= 1 && !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            blank_run = 0;
        }
        if !line.trim().is_empty() {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Whether `line` is a `- ` / `* ` markdown list item.
fn is_bullet(line: &str) -> bool {
    let t = line.trim_start();
    (t.starts_with("- ") || t.starts_with("* ")) && t.len() > 2
}

/// Strip the `- ` / `* ` marker from a list-item line.
fn strip_bullet(line: &str) -> String {
    line.trim_start()
        .chars()
        .skip(2)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Split one inline-markdown line into styled runs. Recognises
/// `**bold**`, `` `code` `` and `#RRGGBB` colour tokens; everything
/// else is plain text. Mirrors the TS `renderInline`.
pub(super) fn parse_inline(line: &str) -> Vec<MdRun> {
    let mut runs = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        let bold = find_delim(rest, "**", "**");
        let code = find_delim(rest, "`", "`");
        let color = find_hex(rest);
        // Earliest match wins.
        let mut best: Option<(usize, usize, MdRun)> = None;
        let mut consider = |m: Option<(usize, usize, MdRun)>| {
            if let Some((start, end, run)) = m {
                if best.as_ref().is_none_or(|(b, _, _)| start < *b) {
                    best = Some((start, end, run));
                }
            }
        };
        consider(bold);
        consider(code);
        consider(color);

        match best {
            Some((start, end, run)) => {
                if start > 0 {
                    runs.push(MdRun::Plain(rest[..start].to_string()));
                }
                runs.push(run);
                rest = &rest[end..];
            }
            None => {
                runs.push(MdRun::Plain(rest.to_string()));
                break;
            }
        }
    }
    runs
}

/// Find a `open … close`-delimited span; returns `(start, end, run)`
/// in byte offsets where `end` is just past the closing delimiter.
fn find_delim(text: &str, open: &str, close: &str) -> Option<(usize, usize, MdRun)> {
    let start = text.find(open)?;
    let inner_start = start + open.len();
    let inner_end = text[inner_start..].find(close)? + inner_start;
    let inner = &text[inner_start..inner_end];
    if inner.is_empty() {
        return None;
    }
    let run = if open == "**" {
        MdRun::Bold(inner.to_string())
    } else {
        MdRun::Code(inner.to_string())
    };
    Some((start, inner_end + close.len(), run))
}

/// Find a `#RRGGBB` colour token not followed by a 7th hex digit.
fn find_hex(text: &str) -> Option<(usize, usize, MdRun)> {
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'#' {
            continue;
        }
        let Some(hex) = bytes.get(i + 1..i + 7) else {
            continue;
        };
        if hex.iter().all(|c| c.is_ascii_hexdigit())
            && !bytes.get(i + 7).is_some_and(|c| c.is_ascii_hexdigit())
        {
            return Some((i, i + 7, MdRun::Color(text[i..i + 7].to_string())));
        }
    }
    None
}

/// Greedy word-wrap a run sequence to `max_chars` per line. Plain /
/// bold runs break at spaces; code / colour runs stay whole.
pub(super) fn wrap_runs(runs: &[MdRun], max_chars: usize) -> Vec<Vec<MdRun>> {
    let max = max_chars.max(8);
    let mut lines: Vec<Vec<MdRun>> = Vec::new();
    let mut line: Vec<MdRun> = Vec::new();
    let mut col = 0usize;

    for run in runs {
        if run.breakable() {
            let mut had_word = false;
            for word in run.text().split_whitespace() {
                had_word = true;
                let w = word.chars().count();
                if col > 0 && col + 1 + w > max {
                    lines.push(std::mem::take(&mut line));
                    col = 0;
                }
                if col > 0 {
                    // A space joins words within a line.
                    append_styled(&mut line, " ", run);
                    col += 1;
                }
                append_styled(&mut line, word, run);
                col += w;
            }
            // A whitespace-only run (e.g. the gap between two code
            // spans) carries no words — keep one visible space so the
            // tokens around it do not visually collide.
            if !had_word && !run.text().is_empty() && col > 0 && col < max {
                append_styled(&mut line, " ", run);
                col += 1;
            }
        } else {
            let w = run.text().chars().count();
            if col > 0 && col + w > max {
                lines.push(std::mem::take(&mut line));
                col = 0;
            }
            line.push(run.clone());
            col += w;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

/// Append `text` to `line` as the same style as `style`, merging
/// with a trailing run of the same style where possible.
fn append_styled(line: &mut Vec<MdRun>, text: &str, style: &MdRun) {
    match (line.last_mut(), style) {
        (Some(MdRun::Plain(s)), MdRun::Plain(_)) => s.push_str(text),
        (Some(MdRun::Bold(s)), MdRun::Bold(_)) => s.push_str(text),
        (_, MdRun::Bold(_)) => line.push(MdRun::Bold(text.to_string())),
        _ => line.push(MdRun::Plain(text.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_lists_paragraphs() {
        let blocks = parse_blocks("### Title\n\n- one\n- two\n\nA paragraph here.", 999);
        assert!(matches!(&blocks[0], MdBlock::Heading { level: 3, .. }));
        assert!(matches!(&blocks[1], MdBlock::Bullet(b) if b == "one"));
        assert!(matches!(&blocks[2], MdBlock::Bullet(b) if b == "two"));
        assert!(matches!(&blocks[3], MdBlock::Paragraph(_)));
    }

    #[test]
    fn inline_splits_bold_code_color() {
        let runs = parse_inline("use **bold** and `code` and #AABBCC here");
        let kinds: Vec<&str> = runs
            .iter()
            .map(|r| match r {
                MdRun::Plain(_) => "p",
                MdRun::Bold(_) => "b",
                MdRun::Code(_) => "c",
                MdRun::Color(_) => "x",
            })
            .collect();
        assert_eq!(kinds, ["p", "b", "p", "c", "p", "x", "p"]);
    }

    #[test]
    fn wrap_breaks_long_plain_runs() {
        let runs = vec![MdRun::Plain("aa bb cc dd ee ff".to_string())];
        let lines = wrap_runs(&runs, 8);
        assert!(lines.len() > 1);
    }
}
