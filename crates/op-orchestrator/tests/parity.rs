//! Frozen compact-planner contracts, originally imported from the retired TS product.
//! Intentional Rust contract changes must update only their affected fixture fields.

use op_orchestrator::build_compact_planning_prompt;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    #[serde(rename = "fn")]
    fn_: String,
    prompt: String,
}

#[derive(serde::Deserialize)]
struct CompactGolden {
    system: String,
    #[serde(rename = "userPrompt")]
    user_prompt: String,
    #[serde(rename = "selectedStyleGuideName")]
    selected_style_guide_name: String,
}

#[test]
fn compact_planning_prompt_matches_frozen_contract() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/planner-golden");
    let cases: Vec<Case> =
        serde_json::from_str(&fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
    let mut checked = 0;
    for c in cases.iter().filter(|c| c.fn_ == "compact") {
        let raw = fs::read_to_string(dir.join(format!("{}.json", c.name))).unwrap_or_else(|_| {
            panic!(
                "golden {}.json missing from tests/planner-golden/ (frozen parity \
                 baseline; no automatic snapshot regeneration)",
                c.name
            )
        });
        let golden: CompactGolden = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("malformed golden {}.json: {e}", c.name));
        let got = build_compact_planning_prompt(&c.prompt, None, None);
        assert_eq!(got.system, golden.system, "case {}: system drift", c.name);
        assert_eq!(
            got.user_prompt, golden.user_prompt,
            "case {}: user_prompt",
            c.name
        );
        assert_eq!(
            got.selected_style_guide_name, golden.selected_style_guide_name,
            "case {}: styleGuideName",
            c.name
        );
        checked += 1;
    }
    assert!(checked > 0, "no compact parity cases ran");
}
