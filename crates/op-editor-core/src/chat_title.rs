pub(crate) const DEFAULT_CHAT_TITLE: &str = "New Chat";

pub(crate) fn suggest_chat_title(prompt: &str) -> Option<String> {
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let first_clause = normalized
        .split([
            '\n', '\r', '，', ',', '。', '.', '！', '!', '？', '?', '；', ';',
        ])
        .next()
        .unwrap_or("")
        .trim();
    let mut title = strip_prompt_prefix(first_clause).trim().to_string();
    title = title.trim_start_matches(['：', ':']).trim().to_string();
    for adjective in ["现代", "精美", "漂亮", "高质量", "简洁", "高端"] {
        if let Some(rest) = title.strip_prefix(&format!("{adjective}的")) {
            title = format!("{adjective}{rest}");
            break;
        }
    }
    (!title.is_empty()).then(|| truncate_title(&title, 19))
}

fn strip_prompt_prefix(input: &str) -> &str {
    const CJK_PREFIXES: &[&str] = &[
        "请帮我设计一个",
        "请帮我设计一款",
        "帮我设计一个",
        "帮我设计一款",
        "帮我生成一个",
        "帮我生成一款",
        "设计一个",
        "设计一款",
        "生成一个",
        "生成一款",
        "创建一个",
        "创建一款",
        "做一个",
        "做一款",
        "实现一个",
        "请帮我",
        "帮我",
        "设计",
        "生成",
        "创建",
        "实现",
        "请",
    ];
    for prefix in CJK_PREFIXES {
        if let Some(rest) = input.strip_prefix(prefix) {
            return rest;
        }
    }

    let lower = input.to_ascii_lowercase();
    for prefix in [
        "please design an ",
        "please design a ",
        "please generate an ",
        "please generate a ",
        "please create an ",
        "please create a ",
        "design an ",
        "design a ",
        "generate an ",
        "generate a ",
        "create an ",
        "create a ",
        "make an ",
        "make a ",
        "design ",
        "generate ",
        "create ",
        "make ",
    ] {
        if lower.starts_with(prefix) {
            return &input[prefix.len()..];
        }
    }
    input
}

fn truncate_title(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let mut out: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::suggest_chat_title;

    #[test]
    fn cjk_prompt_title_uses_first_clause_without_request_prefix() {
        assert_eq!(
            suggest_chat_title("设计一个现代的移动端登录页面，包含邮箱输入框"),
            Some("现代移动端登录页面".into())
        );
    }

    #[test]
    fn english_prompt_title_strips_design_prefix() {
        assert_eq!(
            suggest_chat_title("Design a dashboard for analytics"),
            Some("dashboard for analy...".into())
        );
    }
}
