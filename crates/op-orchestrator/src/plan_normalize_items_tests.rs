//! Tests for the DS-gated item-family bundling step (DS P2-a item ②).
//!
//! The step rewrites `plan.subtasks` — five "法则 01".."法则 05" subtasks
//! become one — only when the model family gate admits the request's model.
//! The screen dimension is the multi-screen fanout's territory: subtasks
//! tagged with different `screen` labels are never merged, whatever the stem.

use super::*;
use crate::plan::{Region, Subtask};

fn req_with_model(model: Option<&str>) -> DesignRequest {
    DesignRequest {
        model: model.map(str::to_string),
        ..Default::default()
    }
}

fn item(id: &str, label: &str, screen: Option<&str>, height: f64) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1080.0,
            height,
        },
        id_prefix: String::new(),
        parent_frame_id: None,
        elements: None,
        screen: screen.map(str::to_string),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

/// Five 法则 items, no screens — the canonical card-board family.
fn five_item_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: (1..=5)
            .map(|n| {
                item(
                    &format!("rule-0{n}"),
                    &format!("法则 0{n}"),
                    None,
                    300.0 * n as f64,
                )
            })
            .collect(),
        style_guide_name: None,
    }
}

#[test]
fn deepseek_model_bundles_five_items_into_one_subtask_in_order() {
    let mut plan = five_item_plan();
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));

    assert_eq!(plan.subtasks.len(), 1, "5 items must merge into 1");
    let merged = &plan.subtasks[0];
    // The description concatenates the member labels in order and notes the
    // member count.
    assert!(
        merged.label.contains("5 items"),
        "the merged label must annotate the member count: {:?}",
        merged.label
    );
    let mut last_pos = 0usize;
    for n in 1..=5 {
        let label = format!("法则 0{n}");
        let pos = merged
            .label
            .find(&label)
            .unwrap_or_else(|| panic!("label {label:?} missing from {:?}", merged.label));
        assert!(
            pos >= last_pos,
            "member labels must stay in order in {:?}",
            merged.label
        );
        last_pos = pos;
    }
    // Every other field is the first member's verbatim.
    assert_eq!(merged.id, "rule-01");
    assert_eq!(merged.region.height, 300.0);
    assert_eq!(merged.screen, None);
    assert_eq!(
        merged.id_prefix, "rule-01",
        "id_prefix assigned before the merge"
    );
    assert_eq!(merged.parent_frame_id.as_deref(), Some("card"));
}

#[test]
fn fewer_than_three_members_never_merge() {
    let mut plan = five_item_plan();
    plan.subtasks.truncate(2);
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    assert_eq!(plan.subtasks.len(), 2, "<3 members must not merge");
}

#[test]
fn glm_model_keeps_the_family_unbundled() {
    let mut plan = five_item_plan();
    normalize(&mut plan, &req_with_model(Some("glm-5.2")));
    assert_eq!(plan.subtasks.len(), 5, "the gate must not fire for glm");
    // Labels untouched.
    assert_eq!(plan.subtasks[0].label, "法则 01");
}

#[test]
fn unknown_model_keeps_the_family_unbundled() {
    let mut plan = five_item_plan();
    normalize(&mut plan, &req_with_model(None));
    assert_eq!(
        plan.subtasks.len(),
        5,
        "default (no model) must not enable the gate"
    );
}

#[test]
fn different_stems_do_not_merge() {
    let mut plan = OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![
            item("rule-01", "法则 01", None, 100.0),
            item("rule-02", "法则 02", None, 100.0),
            item("rule-03", "法则 03", None, 100.0),
            item("case-01", "案例 01", None, 100.0),
            item("case-02", "案例 02", None, 100.0),
        ],
        style_guide_name: None,
    };
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    // 法则 (3) merges; 案例 (2) does not.
    assert_eq!(plan.subtasks.len(), 3, "only the 3-member family merges");
    let labels: Vec<&str> = plan.subtasks.iter().map(|s| s.label.as_str()).collect();
    assert!(labels.iter().any(|l| l.contains("3 items")));
    assert!(labels.contains(&"案例 01") && labels.contains(&"案例 02"));
}

#[test]
fn items_on_different_screens_never_merge() {
    // Three 法则 items on screen S1, two on S2 — the S1 family merges, the
    // S2 pair does not, and NO subtask ever spans both screens (the
    // multi-screen fanout owns the screen dimension).
    let mut plan = OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![
            item("rule-01", "法则 01", Some("S1"), 100.0),
            item("rule-02", "法则 02", Some("S1"), 100.0),
            item("rule-03", "法则 03", Some("S1"), 100.0),
            item("rule-04", "法则 04", Some("S2"), 100.0),
            item("rule-05", "法则 05", Some("S2"), 100.0),
        ],
        style_guide_name: None,
    };
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    assert_eq!(
        plan.subtasks.len(),
        3,
        "S1 family (3) merges, S2 pair (2) stays"
    );
    let s1 = plan
        .subtasks
        .iter()
        .find(|s| s.screen.as_deref() == Some("S1"))
        .expect("merged S1 subtask present");
    assert!(s1.label.contains("3 items"));
    assert!(s1.label.contains("法则 01") && s1.label.contains("法则 03"));
    assert!(!s1.label.contains("法则 04"), "no cross-screen merge");
    let s2: Vec<&Subtask> = plan
        .subtasks
        .iter()
        .filter(|s| s.screen.as_deref() == Some("S2"))
        .collect();
    assert_eq!(s2.len(), 2, "the S2 pair must stay separate");
}

#[test]
fn per_screen_families_merge_independently_without_crossing_screens() {
    // Three items on S1 AND three items on S2: two merged subtasks, each
    // keeping its own screen — the deck fanout shape stays intact.
    let mut plan = OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![
            item("rule-01", "法则 01", Some("S1"), 100.0),
            item("rule-02", "法则 02", Some("S1"), 100.0),
            item("rule-03", "法则 03", Some("S1"), 100.0),
            item("rule-04", "法则 04", Some("S2"), 100.0),
            item("rule-05", "法则 05", Some("S2"), 100.0),
            item("rule-06", "法则 06", Some("S2"), 100.0),
        ],
        style_guide_name: None,
    };
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    assert_eq!(plan.subtasks.len(), 2, "one merged subtask per screen");
    let screens: Vec<&str> = plan
        .subtasks
        .iter()
        .map(|s| s.screen.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(screens, vec!["S1", "S2"]);
    for st in &plan.subtasks {
        assert!(st.label.contains("3 items"));
    }
    assert!(!plan.subtasks[0].label.contains("法则 04"));
    assert!(!plan.subtasks[1].label.contains("法则 01"));
}

#[test]
fn deck_fallback_screens_survive_bundling() {
    // The deck fallback tags every slide with its OWN screen. Five slides
    // whose labels share a stem must therefore never merge even under a
    // deepseek model — the screen protection is what keeps the multi-root
    // fanout intact.
    let mut plan = OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "deck".into(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: (1..=5)
            .map(|n| {
                item(
                    &format!("slide-{n}"),
                    &format!("法则 0{n}"),
                    Some(&format!("Slide {n}")),
                    1080.0,
                )
            })
            .collect(),
        style_guide_name: None,
    };
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    assert_eq!(
        plan.subtasks.len(),
        5,
        "distinct screens must block the merge (multi-screen fanout territory)"
    );
    let roots: std::collections::BTreeSet<&str> = plan
        .subtasks
        .iter()
        .map(|st| st.parent_frame_id.as_deref().expect("parent assigned"))
        .collect();
    assert_eq!(roots.len(), 5, "each slide keeps its own root");
}

#[test]
fn ordinal_only_labels_fall_back_to_the_id_stem() {
    // Labels like "01".."05" leave no label stem — the id stem ("rule-")
    // carries the family instead.
    let mut plan = OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: (1..=5)
            .map(|n| item(&format!("rule-0{n}"), &format!("0{n}"), None, 100.0))
            .collect(),
        style_guide_name: None,
    };
    normalize(&mut plan, &req_with_model(Some("deepseek-v4-pro")));
    assert_eq!(plan.subtasks.len(), 1, "id-stem family must merge");
    assert!(plan.subtasks[0].label.contains("5 items"));
}
