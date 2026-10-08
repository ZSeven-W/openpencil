//! Extract code envelopes without evaluating Markdown or model commentary.

use super::{ScriptError, MAX_SCRIPT_BYTES};

struct Block<'a> {
    body: &'a str,
    closed: bool,
    end: usize,
}

pub(super) fn extract(text: &str) -> Result<String, ScriptError> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, bool)> = None;
    let mut outside = Vec::new();
    let mut offset = 0;
    let mut saw_fence = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some((start, javascript)) = open {
            if trimmed == "```" {
                if javascript {
                    blocks.push(Block {
                        body: &text[start..offset],
                        closed: true,
                        end: offset + line.len(),
                    });
                }
                open = None;
            }
        } else if let Some(language) = trimmed.strip_prefix("```") {
            saw_fence = true;
            let language = language.trim().to_ascii_lowercase();
            open = Some((
                offset + line.len(),
                matches!(language.as_str(), "" | "js" | "javascript" | "ecmascript"),
            ));
        } else if !trimmed.is_empty() {
            outside.push((offset, trimmed));
        }
        offset += line.len();
    }
    if let Some((start, true)) = open {
        blocks.push(Block {
            body: &text[start..],
            closed: false,
            end: text.len(),
        });
    }
    if !saw_fence {
        return Ok(text.to_string());
    }
    // Count every candidate, including a discarded draft, against the existing
    // source cap. A short correction must not bypass an oversized envelope.
    let bytes: usize = blocks.iter().map(|block| block.body.len()).sum();
    if bytes > MAX_SCRIPT_BYTES {
        return Err(ScriptError::SourceTooLarge {
            bytes,
            max: MAX_SCRIPT_BYTES,
        });
    }
    match blocks.as_slice() {
        [] => Err(ScriptError::EmptySource),
        [block] => Ok(block.body.trim().to_string()),
        [first, last]
            if last.closed
                && outside
                    .iter()
                    .any(|(position, line)| *position >= first.end && chooses_correction(line)) =>
        {
            Ok(last.body.trim().to_string())
        }
        _ if outside.is_empty() => Ok(blocks
            .iter()
            .map(|block| block.body.trim())
            .collect::<Vec<_>>()
            .join("\n")),
        _ => Err(ScriptError::AmbiguousCodeBlocks {
            count: blocks.len(),
        }),
    }
}

fn chooses_correction(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    if [
        "不要", "别用", "不用", "do not", "don't", "ignore", "discard",
    ]
    .iter()
    .any(|negative| line.contains(negative))
    {
        return false;
    }
    [
        "第二段程序",
        "第二段代码",
        "修正版如下",
        "以下是修正版",
        "corrected version",
        "use the second",
        "use the final",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script_runner::{run_modification_script_to_program, run_script_to_program};

    #[test]
    fn an_explicit_complete_correction_runs_once_without_the_draft() {
        let text = "```javascript\nI(null,{type:'frame',name:'draft'});\n```\n以下是修正版：\n```js\nI(null,{type:'frame',name:'final'});\n```\n请使用第二段程序作为最终输出。";
        let program = run_script_to_program(text).unwrap();
        assert_eq!(program.lines().count(), 1);
        assert!(program.contains("final"));
        assert!(!program.contains("draft"));
        assert_eq!(
            run_modification_script_to_program(text)
                .unwrap()
                .lines()
                .count(),
            1
        );
    }

    #[test]
    fn ambiguous_alternatives_are_rejected_instead_of_drawing_one_arbitrarily() {
        let text = "Option A:\n```js\nI(null,{type:'frame',name:'A'});\n```\nOption B:\n```js\nI(null,{type:'frame',name:'B'});\n```";
        assert_eq!(
            run_script_to_program(text),
            Err(ScriptError::AmbiguousCodeBlocks { count: 2 })
        );
    }

    #[test]
    fn a_negated_correction_hint_is_not_a_selection() {
        let text = "```js\nI(null,{type:'frame',name:'first'});\n```\nDo not use the corrected version:\n```js\nI(null,{type:'frame',name:'second'});\n```";
        assert!(matches!(
            run_script_to_program(text),
            Err(ScriptError::AmbiguousCodeBlocks { .. })
        ));
    }

    #[test]
    fn separate_fragments_keep_their_shared_bindings() {
        let text = "```js\nconst root=I(null,{type:'frame'});\n```\n```js\nI(root,{type:'text',content:'keep me'});\n```";
        let program = run_script_to_program(text).unwrap();
        assert_eq!(program.lines().count(), 2);
        assert!(program.contains("keep me"));
    }

    #[test]
    fn an_unclosed_correction_does_not_replace_a_complete_program() {
        let text = "```js\nI(null,{type:'frame'});\n```\nUse the corrected version:\n```js\nI(null,{type:'frame',name:'cut";
        assert!(matches!(
            extract(text),
            Err(ScriptError::AmbiguousCodeBlocks { .. })
        ));
    }

    #[test]
    fn ignored_drafts_still_count_against_the_source_limit() {
        let text = format!(
            "```js\n{}\n```\nUse the corrected version:\n```js\nI(null,{{type:'frame'}});\n```",
            " ".repeat(MAX_SCRIPT_BYTES)
        );
        assert!(matches!(
            extract(&text),
            Err(ScriptError::SourceTooLarge { .. })
        ));
    }

    #[test]
    fn non_javascript_fences_do_not_hide_the_only_program() {
        let text =
            "```json\n{\"explanation\":true}\n```\n```javascript\nI(null,{type:'frame'});\n```";
        assert_eq!(run_script_to_program(text).unwrap().lines().count(), 1);
    }
}
