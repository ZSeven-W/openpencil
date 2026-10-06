use crate::{get_skill_by_name, resolve_skills, DropReason, Phase, ResolveOptions, SkillTrigger};

#[test]
fn turbulence_preset_teaching_resolves_complete_for_every_new_trigger() {
    let source = get_skill_by_name("shader-fill").expect("shader-fill registered");
    let schema = get_skill_by_name("schema").expect("schema registered");

    let SkillTrigger::Keywords(keywords) = &source.meta.trigger else {
        panic!("shader-fill must be keyword-gated");
    };
    for keyword in ["grain", "turbulence", "feTurbulence"] {
        assert!(
            keywords.iter().any(|item| item == keyword),
            "missing `{keyword}` trigger"
        );
    }

    assert!(source
        .content
        .contains(r#"fill: [{ type: "shader", preset: "turbulence","#));
    assert!(!source.content.contains("float hash(float2 p)"));
    assert!(schema
        .content
        .contains(r#"turbulence preset example: `fill: [{ type: "shader", preset: "turbulence""#));

    for prompt in [
        "add fine film grain to the background",
        "use turbulence for the hero texture",
        "use feTurbulence for the texture",
    ] {
        let context = resolve_skills(Phase::Generation, prompt, &ResolveOptions::default());
        let resolved = context
            .skills
            .iter()
            .find(|skill| skill.meta.name == "shader-fill")
            .expect("new trigger must resolve shader-fill");
        assert!(!resolved.truncated, "shader-fill truncated for `{prompt}`");
        assert_eq!(resolved.content, source.content);
        assert!(
            context.skills.iter().all(|skill| !skill.truncated),
            "a new shader trigger must not squeeze another resolved skill"
        );
        assert!(
            context
                .report
                .dropped
                .iter()
                .all(|skill| skill.reason != DropReason::BudgetExhausted),
            "a new shader trigger must not budget-drop another matched skill"
        );
    }
}
