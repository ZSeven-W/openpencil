//! Corpus and assembly guards for the text-only social-card image gate.

use crate::loader::get_skill_by_name;
use crate::{design_agent_system_prompt_with_skills, resolve_skills, Phase, ResolveOptions};

const PROMPT: &str = "用这个文字做一张符合小红书封面的卡片：装了这么多 DSH 插件，到底怎么管？";

#[test]
fn text_only_xhs_prompt_resolves_the_complete_card_contract() {
    let source = get_skill_by_name("cards").expect("cards skill must be registered");
    let ctx = resolve_skills(Phase::Generation, PROMPT, &ResolveOptions::default());
    let resolved = ctx
        .skills
        .iter()
        .find(|skill| skill.meta.name == "cards")
        .expect("the XHS prompt must resolve cards");

    assert!(
        !resolved.truncated,
        "the card contract must not be truncated"
    );
    assert_eq!(resolved.content, source.content);
    assert!(resolved.content.contains("TEXT-ONLY IMAGE GATE"));
    assert!(resolved.content.contains("create NO image node"));
}

#[test]
fn production_prompt_forbids_inventing_a_raster_slot_for_text_only_cards() {
    let prompt = design_agent_system_prompt_with_skills(PROMPT);

    assert!(prompt.contains("every image slot gets a real image"));
    assert!(prompt.contains("never means \"every design must contain an image slot\""));
    assert!(prompt.contains("model-invented image slot in a text-only social card"));
    assert!(prompt.contains("stock-search background"));
}
