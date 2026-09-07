use super::*;
use crate::plan::{Region, RootFrameSpec, Subtask};

fn subtask(id: &str, screen: Option<&str>, elements: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: id.into(),
        region: Region {
            width: 390.0,
            height: 200.0,
        },
        bleed_hero: false,
        id_prefix: String::new(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: elements.map(str::to_owned),
        screen: screen.map(str::to_owned),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan(subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Root".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks,
        style_guide_name: None,
    }
}

#[test]
fn marks_exactly_one_hero_subtask_per_screen() {
    let mut plan = plan(vec![
        subtask(
            "hero-one",
            Some("Home"),
            Some("ARCHETYPE: image-led — destination photo"),
        ),
        subtask("hero-two", Some("Home"), Some("full-bleed map route")),
        subtask("profile-map", Some("Profile"), Some("ARCHETYPE: route-map")),
    ]);

    mark_bleed_hero_subtasks(&mut plan);

    assert_eq!(
        plan.subtasks
            .iter()
            .filter(|subtask| subtask.bleed_hero)
            .map(|subtask| subtask.id.as_str())
            .collect::<Vec<_>>(),
        ["hero-one", "profile-map"]
    );
}

#[test]
fn matching_is_case_insensitive_and_leading_whitespace_is_ignored() {
    let mut plan = plan(vec![subtask(
        "hero",
        None,
        Some("  a FULL-BLEED image-led treatment"),
    )]);

    mark_bleed_hero_subtasks(&mut plan);

    assert!(plan.subtasks[0].bleed_hero);
}

#[test]
fn derived_marker_is_not_serialized_back_to_the_model() {
    let mut plan = plan(vec![subtask("hero", None, Some("ARCHETYPE: image-led"))]);
    mark_bleed_hero_subtasks(&mut plan);

    let value = serde_json::to_value(&plan).expect("plan serializes");
    assert_eq!(value["subtasks"][0].get("bleedHero"), None);
}

#[test]
fn parsed_mobile_plan_marks_one_matching_subtask() {
    let text = r#"{
        "rootFrame": {"id":"page","name":"Page","width":390,"height":844},
        "subtasks": [
            {"id":"hero","label":"Hero","region":{"width":390,"height":280},"elements":"ARCHETYPE: image-led"},
            {"id":"details","label":"Details","region":{"width":390,"height":280},"elements":"full-bleed detail card"},
            {"id":"footer","label":"Footer","region":{"width":390,"height":120},"elements":"metadata"}
        ]
    }"#;
    let mut plan = crate::plan::parse_plan(text).expect("plan parses");
    crate::plan_normalize::normalize(&mut plan, &Default::default());

    assert_eq!(
        plan.subtasks
            .iter()
            .filter(|subtask| subtask.bleed_hero)
            .map(|subtask| subtask.id.as_str())
            .collect::<Vec<_>>(),
        ["hero"]
    );
}
