//! Style-guide MCP parity tests.

use std::collections::BTreeMap;

use super::{get_style_guide_snapshot, get_style_guide_tags_snapshot, McpTool, ToolOutcome};

#[test]
fn get_style_guide_tags_returns_light_and_dark_vocab() {
    match get_style_guide_tags_snapshot().call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert!(out
                .get("tags")
                .is_some_and(|tags| tags.contains("\"light-mode\"")));
            assert!(out
                .get("tags")
                .is_some_and(|tags| tags.contains("\"dark-mode\"")));
            let count: usize = out.get("count").expect("count").parse().expect("count");
            assert!(count > 50);
        }
        other => panic!("expected tags ok, got {other:?}"),
    }
}

#[test]
fn get_style_guide_finds_specific_guide_by_name() {
    let mut args = BTreeMap::new();
    args.insert("name".into(), "saas-clean-light".into());

    match get_style_guide_snapshot().call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("name"), Some(&"saas-clean-light".to_string()));
            assert_eq!(out.get("platform"), Some(&"webapp".to_string()));
            assert!(out
                .get("tags")
                .is_some_and(|tags| tags.contains("\"light-mode\"")));
            assert!(out
                .get("content")
                .is_some_and(|content| content.contains("saas-clean-light")));
        }
        other => panic!("expected guide ok, got {other:?}"),
    }
}
