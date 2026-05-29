#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParsedStepStatus {
    Pending,
    Streaming,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedStep {
    pub title: String,
    pub status: Option<ParsedStepStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepExtraction {
    pub steps: Vec<ParsedStep>,
    pub visible_text: String,
}

pub(crate) fn split_design_progress(thinking: &str) -> (Vec<ParsedStep>, String) {
    let mut steps = Vec::new();
    let mut rest = Vec::new();
    for line in thinking.lines() {
        let trimmed = line.trim();
        if let Some(label) = trimmed.strip_prefix('•').map(str::trim) {
            if !label.is_empty() {
                steps.push(ParsedStep {
                    title: label.to_string(),
                    status: None,
                });
            }
        } else if !trimmed.is_empty() {
            rest.push(line.trim_end().to_string());
        }
    }
    (steps, rest.join("\n"))
}

pub(crate) fn extract_step_blocks(text: &str, is_streaming: bool) -> StepExtraction {
    let mut steps = Vec::new();
    let mut visible = String::new();
    let mut cursor = 0usize;
    let mut keep_from = 0usize;

    while let Some(open) = find_ascii_ci(text, "<step", cursor) {
        let Some(tag_end_rel) = text[open..].find('>') else {
            visible.push_str(&text[keep_from..open]);
            if is_streaming {
                steps.push(parsed_step("", "", true));
            }
            return finish_extraction(steps, visible);
        };
        let tag_end = open + tag_end_rel;
        let attrs = &text[open + "<step".len()..tag_end];
        let content_start = tag_end + 1;

        if let Some(close) = find_ascii_ci(text, "</step>", content_start) {
            visible.push_str(&text[keep_from..open]);
            steps.push(parsed_step(attrs, &text[content_start..close], false));
            let close_end = close + "</step>".len();
            cursor = close_end;
            keep_from = close_end;
        } else {
            visible.push_str(&text[keep_from..open]);
            if is_streaming {
                steps.push(parsed_step(attrs, &text[content_start..], true));
            }
            return finish_extraction(steps, visible);
        }
    }

    visible.push_str(&text[keep_from..]);
    finish_extraction(steps, visible)
}

fn finish_extraction(steps: Vec<ParsedStep>, visible: String) -> StepExtraction {
    StepExtraction {
        steps,
        visible_text: visible.trim().to_string(),
    }
}

fn parsed_step(attrs: &str, _content: &str, partial: bool) -> ParsedStep {
    let default_title = if partial { "Design" } else { "Processing" };
    ParsedStep {
        title: attr_value(attrs, "title")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_title.to_string()),
        status: attr_value(attrs, "status").and_then(|s| parse_status(s.trim())),
    }
}

fn parse_status(value: &str) -> Option<ParsedStepStatus> {
    match value.to_ascii_lowercase().as_str() {
        "pending" => Some(ParsedStepStatus::Pending),
        "streaming" => Some(ParsedStepStatus::Streaming),
        "done" => Some(ParsedStepStatus::Done),
        "error" => Some(ParsedStepStatus::Error),
        _ => None,
    }
}

fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut cursor = 0usize;
    while let Some(found_rel) = find_ascii_ci(attrs, key, cursor) {
        let found = found_rel;
        let before_ok = found == 0
            || attrs[..found]
                .chars()
                .next_back()
                .map_or(true, |c| c.is_ascii_whitespace());
        let after_key = found + key.len();
        let after_ok = attrs[after_key..]
            .chars()
            .next()
            .map_or(true, |c| c.is_ascii_whitespace() || c == '=');
        if !before_ok || !after_ok {
            cursor = after_key;
            continue;
        }

        let mut rest = attrs[after_key..].trim_start();
        if !rest.starts_with('=') {
            cursor = after_key;
            continue;
        }
        rest = rest[1..].trim_start();
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let value_start = quote.len_utf8();
            let value = &rest[value_start..];
            return value.find(quote).map(|end| value[..end].to_string());
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

fn find_ascii_ci(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let hay = haystack.get(start..)?;
    let hay = hay.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    hay.find(&needle).map(|idx| start + idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_closed_step_blocks_and_strips_visible_text() {
        let extracted = extract_step_blocks(
            r#"before <step title="Check" status="done">ok</step> after"#,
            false,
        );

        assert_eq!(extracted.visible_text, "before  after");
        assert_eq!(
            extracted.steps,
            vec![ParsedStep {
                title: "Check".into(),
                status: Some(ParsedStepStatus::Done),
            }]
        );
    }

    #[test]
    fn extracts_partial_streaming_step() {
        let extracted =
            extract_step_blocks(r#"<step title="Sketch" status="streaming">drawing"#, true);

        assert!(extracted.visible_text.is_empty());
        assert_eq!(extracted.steps[0].title, "Sketch");
        assert_eq!(extracted.steps[0].status, Some(ParsedStepStatus::Streaming));
    }
}
