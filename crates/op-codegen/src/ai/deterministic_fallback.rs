//! Deterministic last-resort code generation for the AI pipeline.
//!
//! When planning/chunks and the one-shot whole-document AI rescue all fail,
//! the existing pure Rust framework generators can still emit a usable file
//! from the selected canonical node forest. Deterministic targets
//! (`Framework::is_deterministic`) take this path directly as their
//! primary, non-degraded output.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use op_editor_core::codegen::Framework;
use serde_json::{json, Value};

use crate::{
    Codegen, Compose, Flutter, Html, React, ReactNative, ReactTailwind, Svelte, SwiftUi, Vue,
};

/// Why the deterministic last-resort generation could not produce code.
/// The `Display` text is recorded verbatim in the pipeline's failure
/// history, so it must stay byte-identical to the strings it replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DeterministicFallbackError {
    /// The selected-nodes payload is not parseable JSON; carries the serde
    /// message (`serde_json::Error` is neither `Clone` nor `Eq`).
    InvalidJson(String),
    /// The payload is a JSON array with no elements.
    EmptyNodeList,
    /// The payload is neither a JSON object nor a JSON array.
    NotObjectOrArray,
    /// The payload parsed but is not a canonical `.op` node forest.
    NotCanonicalNodes(String),
    /// The framework generator ran but emitted only whitespace.
    EmptyGeneratedCode(Framework),
}

impl std::fmt::Display for DeterministicFallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(detail) => {
                write!(formatter, "selected nodes are not valid JSON: {detail}")
            }
            Self::EmptyNodeList => formatter.write_str("selected node list is empty"),
            Self::NotObjectOrArray => {
                formatter.write_str("selected nodes must be a JSON object or array")
            }
            Self::NotCanonicalNodes(detail) => write!(
                formatter,
                "selected nodes are not canonical .op nodes: {detail}"
            ),
            Self::EmptyGeneratedCode(framework) => write!(
                formatter,
                "the deterministic {} generator returned empty code",
                framework.as_wire()
            ),
        }
    }
}

impl std::error::Error for DeterministicFallbackError {}

/// Document-level context that travels next to the selected nodes. Each
/// field is optional JSON; invalid payloads are ignored, never fatal.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct DocumentContext<'a> {
    pub variables_json: Option<&'a str>,
    pub themes_json: Option<&'a str>,
    pub components_json: Option<&'a str>,
}

pub(super) fn generate(
    nodes_json: &str,
    context: DocumentContext<'_>,
    framework: Framework,
) -> Result<String, DeterministicFallbackError> {
    let value: Value = serde_json::from_str(nodes_json)
        .map_err(|error| DeterministicFallbackError::InvalidJson(error.to_string()))?;
    let children = match value {
        Value::Array(children) if !children.is_empty() => children,
        Value::Object(_) => vec![value],
        Value::Array(_) => return Err(DeterministicFallbackError::EmptyNodeList),
        _ => return Err(DeterministicFallbackError::NotObjectOrArray),
    };
    let mut document: PenDocument = serde_json::from_value(json!({
        "version": "1.0.0",
        "children": children,
    }))
    .map_err(|error| DeterministicFallbackError::NotCanonicalNodes(error.to_string()))?;
    if let Some(variables_json) = context.variables_json {
        // Variables should enrich the fallback, never make an otherwise valid
        // deterministic generation fail. Invalid or stale variable payloads
        // are therefore ignored while the selected canonical nodes survive.
        if let Ok(variables) = serde_json::from_str(variables_json) {
            document.variables = Some(variables);
        }
    }
    if let Some(themes) = context
        .themes_json
        .and_then(|json| serde_json::from_str(json).ok())
    {
        document.themes = Some(themes);
    }
    // Masters for `ref` instances outside the selection; an invalid
    // payload only costs instance resolution, not the generation.
    let components: Vec<PenNode> = context
        .components_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();

    let code = match framework {
        Framework::React => React.generate(&document),
        Framework::ReactTailwind => ReactTailwind
            .generate_files_with_components(&document, &components)
            .to_single_file(),
        Framework::Vue => Vue.generate(&document),
        Framework::Svelte => Svelte.generate(&document),
        Framework::Html => Html.generate(&document),
        Framework::Flutter => Flutter.generate(&document),
        Framework::SwiftUi => SwiftUi.generate(&document),
        Framework::Compose => Compose.generate(&document),
        Framework::ReactNative => ReactNative.generate(&document),
    };
    if code.trim().is_empty() {
        Err(DeterministicFallbackError::EmptyGeneratedCode(framework))
    } else {
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE: &str = r#"[{"type":"frame","id":"root","name":"Root","x":0,"y":0,"width":320,"height":200,"children":[{"type":"text","id":"title","content":"Hello","x":12,"y":12,"width":100,"height":24}]}]"#;

    #[test]
    fn every_framework_has_a_non_empty_deterministic_fallback() {
        for framework in [
            Framework::React,
            Framework::ReactTailwind,
            Framework::Vue,
            Framework::Svelte,
            Framework::Html,
            Framework::Flutter,
            Framework::SwiftUi,
            Framework::Compose,
            Framework::ReactNative,
        ] {
            let code = generate(NODE, DocumentContext::default(), framework)
                .unwrap_or_else(|error| panic!("{} fallback failed: {error}", framework.as_wire()));
            assert!(!code.trim().is_empty(), "{}", framework.as_wire());
        }
    }

    #[test]
    fn invalid_nodes_return_an_actionable_error() {
        let error = generate("not-json", DocumentContext::default(), Framework::React)
            .expect_err("invalid fixture")
            .to_string();
        assert!(error.contains("not valid JSON"), "{error}");
    }

    #[test]
    fn valid_variables_are_preserved_in_css_capable_fallbacks() {
        let variables = r##"{"brand":{"type":"color","value":"#3366ff"}}"##;
        let code = generate(
            NODE,
            DocumentContext {
                variables_json: Some(variables),
                ..DocumentContext::default()
            },
            Framework::Vue,
        )
        .expect("vue fallback");
        assert!(code.contains("--brand: #3366ff;"), "{code}");
    }

    #[test]
    fn tailwind_target_uses_themes_and_out_of_selection_components() {
        let nodes = r#"[{"type":"frame","id":"row","layout":"horizontal","children":[{"type":"ref","id":"cta","ref":"master","descendants":{"master-label":{"content":"Go"}}}]}]"#;
        let components = r##"[{"type":"frame","id":"master","name":"Outline Button","reusable":true,"children":[{"type":"text","id":"master-label","content":"Button"}]}]"##;
        let variables = r##"{"--card":{"type":"color","value":[{"value":"#ffffff","theme":{"Mode":"Light"}},{"value":"#111111","theme":{"Mode":"Dark"}}]}}"##;
        let code = generate(
            nodes,
            DocumentContext {
                variables_json: Some(variables),
                themes_json: Some(r#"{"Mode":["Light","Dark"]}"#),
                components_json: Some(components),
            },
            Framework::ReactTailwind,
        )
        .expect("tailwind generation");
        assert!(
            code.contains("<Button variant=\"outline\">Go</Button>"),
            "{code}"
        );
        let dark = &code[code.find("// .dark {").expect("dark block")..];
        assert!(
            dark[..dark.find("// }").unwrap()].contains("//   --card: #111111;"),
            "{code}"
        );
    }

    #[test]
    fn invalid_variables_do_not_block_node_fallback() {
        let code = generate(
            NODE,
            DocumentContext {
                variables_json: Some("not-json"),
                ..DocumentContext::default()
            },
            Framework::Html,
        )
        .expect("html fallback");
        assert!(code.contains("<!DOCTYPE html>"), "{code}");
    }
}
