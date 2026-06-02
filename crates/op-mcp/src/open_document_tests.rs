//! TS-compatible `open_document` parity tests.

use std::collections::BTreeMap;

use super::test_fixtures::sample;
use super::{open_document_snapshot, McpTool, ToolOutcome};

#[test]
fn open_document_reports_live_document_metadata_context_and_prompt() {
    let state = sample();
    let tool = open_document_snapshot(&state);
    let args = BTreeMap::new();

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("filePath"), Some(&"live://canvas".to_string()));
            let document: serde_json::Value =
                serde_json::from_str(out.get("document").expect("document json"))
                    .expect("document json");
            assert_eq!(document["childCount"], 1);
            assert_eq!(document["pageCount"], 1);
            assert_eq!(document["hasVariables"], false);
            assert!(out
                .get("context")
                .is_some_and(|context| context.contains("DOCUMENT SUMMARY")));
            assert!(out
                .get("designPrompt")
                .is_some_and(|prompt| prompt.contains("READ/INSPECT")));
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}
