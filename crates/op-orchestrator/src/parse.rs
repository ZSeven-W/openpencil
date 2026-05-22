//! 节点解析 —— sub-agent 的 LLM 文本输出 → canonical `PenNode` 树。
//!
//! sub-agent prompt(见 `prompt` 模块)要求模型输出 canonical
//! PenNode JSON 数组。本模块抽出 JSON 数组并 serde 反序列化。

use jian_ops_schema::node::PenNode;

/// 节点解析错误。
#[derive(Debug, Clone)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node parse error: {}", self.0)
    }
}

/// 从 LLM 文本里抽 JSON 数组并反序列化为 canonical `PenNode` 树。
///
/// 容忍 ```json 围栏 + 前后散文 —— 抽第一个平衡的 `[...]`。
/// 空数组视为失败(零节点)。
pub fn parse_nodes(text: &str) -> Result<Vec<PenNode>, ParseError> {
    let json = extract_json_array(text).ok_or_else(|| ParseError("no JSON array found".into()))?;
    let nodes: Vec<PenNode> =
        serde_json::from_str(json).map_err(|e| ParseError(format!("deserialize: {e}")))?;
    if nodes.is_empty() {
        return Err(ParseError("empty node array".into()));
    }
    Ok(nodes)
}

/// 抽第一个平衡的 `[...]`(忽略字符串内的方括号)。
fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::PenNodeExt as _;

    #[test]
    fn parse_nodes_reads_fenced_pennode_array() {
        // 一个最小的 frame 节点(canonical schema:`type` 标签 +
        // base 字段)。
        let text = r#"Sure, here are the nodes:
```json
[
  { "type": "frame", "id": "hero", "name": "Hero", "x": 0, "y": 0, "width": 1200, "height": 400, "children": [] }
]
```"#;
        let nodes = parse_nodes(text).expect("parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "hero");
    }

    #[test]
    fn parse_nodes_rejects_empty_array() {
        assert!(parse_nodes("[]").is_err());
    }

    #[test]
    fn parse_nodes_rejects_no_array() {
        assert!(parse_nodes("the model wrote prose only").is_err());
    }
}
