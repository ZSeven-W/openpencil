//! P0 acceptance for the card system's three break points
//! (`card-system-0808.md` §8.2): design type → tags → style-guide shelf.
//!
//! The three are tested together on purpose. Each one alone looks fine and
//! ships nothing: the type without the platform leaves the guides unreachable,
//! the platform without the type points at an empty shelf, and the tags
//! without either score against the wrong candidates. The user-visible
//! promise is "ask for 小红书 cards, get one of the four shipped styles at
//! 1080x1440", and only the whole chain delivers it.

use crate::compact_prompt::build_compact_planning_prompt;
use crate::design_type::{detect_design_type, DesignType};
use crate::style_guide_context::infer_tags_from_prompt;

const CARD_GUIDES: [&str; 4] = [
    "mingsha-mineral-dark",
    "leadprint-vermilion-light",
    "arcade-neon-dark",
    "highlighter-notebook-light",
];

const CARD_PROMPT: &str = "帮我做一套小红书卡片：如何早起";

#[test]
fn a_card_prompt_reaches_a_card_style_guide() {
    let preset = detect_design_type(CARD_PROMPT);
    assert_eq!(preset.type_, DesignType::Card);

    let tags = infer_tags_from_prompt(CARD_PROMPT);
    for expected in [
        "social-card",
        "card-series",
        "vertical-portrait",
        "cjk-type",
    ] {
        assert!(tags.iter().any(|t| t == expected), "{expected} in {tags:?}");
    }

    let built = build_compact_planning_prompt(CARD_PROMPT, None, None);
    assert!(
        CARD_GUIDES.contains(&built.selected_style_guide_name.as_str()),
        "selected {:?}, expected one of the shipped card guides",
        built.selected_style_guide_name
    );
    assert!(
        built.system.contains("width=1080") && built.system.contains("height=1440"),
        "the planning prompt states the 3:4 board size: {}",
        built.system
    );
}

#[test]
fn all_four_shipped_guides_sit_on_the_card_shelf_and_are_selectable_by_name() {
    // Deliberately NOT "this prompt picks that theme": `card-system-0808.md`
    // §8.3 rules out automatic theme recommendation — picking a theme from
    // content words is exactly the model-lottery the spec exists to remove.
    // What the wiring owes is that all four are ON the shelf and each can be
    // asked for by name, which is the sanctioned selection path.
    use op_ai_skills::style_guide::{
        select_style_guide, style_guide_registry, Platform, SelectOptions,
    };
    let registry = style_guide_registry();
    for name in CARD_GUIDES {
        let guide = registry
            .iter()
            .find(|g| g.name == name)
            .unwrap_or_else(|| panic!("{name} is registered"));
        assert_eq!(
            guide.platform,
            Platform::Card,
            "{name} must declare the card platform or it is unreachable"
        );
        let selected = select_style_guide(
            registry,
            &SelectOptions {
                tags: Vec::new(),
                name: Some(name.to_string()),
                platform: Some(Platform::Card),
            },
        );
        assert_eq!(selected.map(|g| g.name.as_str()), Some(name));
    }
}

#[test]
fn a_component_request_never_lands_on_the_card_shelf() {
    for prompt in [
        "卡片组件",
        "a profile card",
        "a card component for the design system",
    ] {
        let built = build_compact_planning_prompt(prompt, None, None);
        assert!(
            !CARD_GUIDES.contains(&built.selected_style_guide_name.as_str()),
            "{prompt} reached the card shelf: {:?}",
            built.selected_style_guide_name
        );
        assert!(
            !infer_tags_from_prompt(prompt)
                .iter()
                .any(|t| t == "social-card"),
            "{prompt} got card tags"
        );
    }
}

#[test]
fn an_ordinary_web_request_is_unaffected() {
    // The platform filter is hard in both directions: the four card guides
    // must be invisible to every non-card request.
    for prompt in [
        "a coffee brand landing page",
        "an analytics dashboard",
        "a mobile login screen",
    ] {
        let built = build_compact_planning_prompt(prompt, None, None);
        assert!(
            !CARD_GUIDES.contains(&built.selected_style_guide_name.as_str()),
            "{prompt} -> {:?}",
            built.selected_style_guide_name
        );
    }
}

#[test]
fn the_card_board_survives_plan_normalization() {
    // A model that plans 1080x0 must still get a 3:4 board — the deck's
    // measured failure, ported to the card contract.
    let mut plan = crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width: 1080.0,
            height: 0.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![crate::plan::Subtask {
            id: "cover".into(),
            label: "封面".into(),
            region: crate::plan::Region {
                width: 1080.0,
                height: 1440.0,
            },
            id_prefix: String::new(),
            parent_frame_id: None,
            insert_after_sibling_id: None,
            elements: None,
            screen: Some("封面".into()),
            generated_root_id: None,
            existing_section_labels: None,
            retry_feedback: None,
        }],
        style_guide_name: None,
    };
    let req = crate::types::DesignRequest {
        prompt: CARD_PROMPT.to_string(),
        ..Default::default()
    };
    let info = crate::plan_normalize::normalize(&mut plan, &req);
    assert_eq!(plan.root_frame.width, 1080.0);
    assert_eq!(plan.root_frame.height, 1440.0);
    assert!(
        info.preserve_requested_root_height,
        "cleanup must not grow a card board to its content"
    );
    assert!(!info.is_mobile, "1080 wide is not a phone screen");
}
