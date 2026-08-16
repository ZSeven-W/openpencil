//! Corpus and assembly guards for `logo-contract`.
//!
//! The production path embeds Markdown, resolves intent, caps each skill, and
//! then fills the phase budget. These tests compare the resolved bytes with the
//! registry bytes, then verify the production design-agent assembly contains
//! each complete body so either truncation mechanism fails loudly.

use crate::loader::get_skill_by_name;
use crate::types::{Phase, SkillCategory, SkillTrigger};
use crate::{design_agent_system_prompt_with_skills, resolve_skills, ResolveOptions};

const LOGO_TAIL: &str = "PASS ONLY WHEN all four fusion tests and all three rendering predicates survive in the exported artifact; otherwise return the pure wordmark.";
const SLIDES_TAIL: &str = "the target `.op` document everywhere in this API.";
const DECK_PATTERNS_TAIL: &str = "a rectangle does not render its children, so the label inside a rectangle placeholder is invisible.";
const DECK_CONTRACT_TAIL: &str =
    "**Formulaic titles** — \"From X to Y\", ad slogans, parallel phrasing on every page.";
const WEB_APP_TAIL: &str = "Apply these silently through node structure.";
const CJK_TAIL: &str = "Detect CJK from the request language — apply these rules (script-specific Noto for headings; the size-banded lineHeight/letterSpacing above everywhere).";

fn assert_resolves_in_full(prompt: &str, names_and_tails: &[(&str, &str)]) {
    let ctx = resolve_skills(Phase::Generation, prompt, &ResolveOptions::default());
    for (name, tail) in names_and_tails {
        let source = get_skill_by_name(name).unwrap_or_else(|| panic!("{name} must be registered"));
        let resolved = ctx
            .skills
            .iter()
            .find(|skill| skill.meta.name == *name)
            .unwrap_or_else(|| {
                panic!(
                    "{prompt:?} dropped {name}; kept {:?} at {}/{} tokens",
                    ctx.skills
                        .iter()
                        .map(|skill| skill.meta.name.as_str())
                        .collect::<Vec<_>>(),
                    ctx.budget_used,
                    ctx.budget_max
                )
            });
        assert!(
            !resolved.truncated,
            "{prompt:?} marked {name} truncated at {}/{} tokens",
            ctx.budget_used, ctx.budget_max
        );
        assert_eq!(
            resolved.content, source.content,
            "{prompt:?} did not assemble the complete registry body for {name}"
        );
        assert!(
            resolved.content.trim_end().ends_with(tail),
            "{prompt:?} lost the tail of {name}: expected {tail:?}"
        );
    }
}

fn assert_assembly_contains_full(prompt: &str, names: &[&str]) {
    let assembled = design_agent_system_prompt_with_skills(prompt);
    for name in names {
        let source = get_skill_by_name(name).unwrap_or_else(|| panic!("{name} must be registered"));
        assert!(
            assembled.contains(source.content.trim()),
            "the real design-agent assembly did not carry the complete {name} body for {prompt:?}"
        );
    }
}

#[test]
fn logo_contract_registers_with_narrow_logo_intent() {
    let skill = get_skill_by_name("logo-contract").expect("logo-contract must be registered");
    assert_eq!(skill.meta.category, SkillCategory::Domain);
    assert!(skill.meta.phase.contains(&Phase::Generation));
    assert_eq!(skill.meta.priority, 21);
    assert_eq!(skill.meta.budget, 1900);
    let SkillTrigger::Keywords(keywords) = &skill.meta.trigger else {
        panic!("logo-contract must be keyword-gated");
    };
    for keyword in [
        "logo design",
        "wordmark",
        "logo critique",
        "logo comparison",
        "标志设计",
        "设计一个logo",
        "字标",
        "品牌板",
    ] {
        assert!(
            keywords.iter().any(|candidate| candidate == keyword),
            "logo-contract must trigger on {keyword:?}"
        );
    }
    assert!(
        !keywords.iter().any(|candidate| candidate == "logo"),
        "a bare logo-placement mention must not spend the logo-contract budget"
    );
}

#[test]
fn logo_contract_carries_operations_gates_tests_and_render_predicates() {
    let body = &get_skill_by_name("logo-contract")
        .expect("logo-contract must be registered")
        .content;

    for operation in [
        "Boundary merge",
        "Path economy",
        "Load-bearing replacement",
        "Counterform encoding",
        "Orientation recast",
        "Letterform reconstruction",
        "Language hinge",
        "Behavior distillation",
        "Mother-form reuse",
        "Bridge-form reduction",
        "Module-and-void tuning",
    ] {
        assert!(
            body.contains(operation),
            "missing structural operation {operation:?}"
        );
    }
    assert_eq!(
        body.matches("**EXIT ").count(),
        4,
        "the applicability gate must expose exactly four exits"
    );
    assert_eq!(
        body.matches("**F").count(),
        4,
        "the fusion proof must expose exactly four tests"
    );
    assert_eq!(
        body.matches("**R").count(),
        3,
        "the render proof must expose monochrome, minimum-size and colour predicates"
    );
    for required in [
        "return a pure wordmark",
        "allow every candidate to be rejected",
        "no rendered artifact, visual QA or similarity screen was completed",
        "Untested cells remain `not run`",
        "render solid black on white and the exact reverse",
        "render at its actual pixel dimensions",
        "Add colour only after R1 and R2 pass",
        "does not promise production-grade curve engineering",
    ] {
        assert!(
            body.contains(required),
            "logo-contract must teach {required:?}"
        );
    }
    assert!(body.trim_end().ends_with(LOGO_TAIL));
}

#[test]
fn logo_prompt_resolves_and_reaches_the_design_agent_without_tail_loss() {
    let prompt = "Critique this wordmark logo design and prepare a brand concept sheet";
    assert_resolves_in_full(prompt, &[("logo-contract", LOGO_TAIL)]);
    assert_resolves_in_full("帮我设计一个logo", &[("logo-contract", LOGO_TAIL)]);

    let assembled = design_agent_system_prompt_with_skills(prompt);
    assert!(
        assembled.contains(LOGO_TAIL),
        "the real design-agent assembly must carry the logo-contract tail"
    );
    assert_assembly_contains_full(prompt, &["logo-contract"]);
}

#[test]
fn existing_deck_contracts_still_resolve_byte_complete() {
    let prompt =
        "Create a 10-slide SaaS pitch deck with analytics tables and a landing-page comparison";
    let expected = [
        ("deck-patterns", DECK_PATTERNS_TAIL),
        ("deck-contract", DECK_CONTRACT_TAIL),
        ("slides", SLIDES_TAIL),
    ];
    assert_resolves_in_full(prompt, &expected);
    assert_assembly_contains_full(prompt, &["deck-patterns", "deck-contract", "slides"]);

    let assembled = design_agent_system_prompt_with_skills(prompt);
    for (_, tail) in expected {
        assert!(
            assembled.contains(tail),
            "the real design-agent assembly lost deck tail {tail:?}"
        );
    }
}

#[test]
fn logo_and_deck_contracts_survive_the_same_production_assembly() {
    let prompt = "Compare logo wordmark directions, then present the brand concept and QA evidence in a 10-slide pitch deck";
    let expected = [
        ("logo-contract", LOGO_TAIL),
        ("deck-patterns", DECK_PATTERNS_TAIL),
        ("deck-contract", DECK_CONTRACT_TAIL),
        ("slides", SLIDES_TAIL),
    ];
    assert_resolves_in_full(prompt, &expected);
    assert_assembly_contains_full(
        prompt,
        &["logo-contract", "deck-patterns", "deck-contract", "slides"],
    );

    let assembled = design_agent_system_prompt_with_skills(prompt);
    for (_, tail) in expected {
        assert!(
            assembled.contains(tail),
            "the mixed logo/deck assembly lost contract tail {tail:?}"
        );
    }

    let cjk_prompt = "请评审这个字标 logo，并做一份 10 页营销官网路演 PPT";
    let cjk_expected = [
        ("logo-contract", LOGO_TAIL),
        ("cjk-typography", CJK_TAIL),
        ("deck-patterns", DECK_PATTERNS_TAIL),
        ("deck-contract", DECK_CONTRACT_TAIL),
        ("slides", SLIDES_TAIL),
    ];
    assert_resolves_in_full(cjk_prompt, &cjk_expected);
    assert_assembly_contains_full(
        cjk_prompt,
        &[
            "logo-contract",
            "cjk-typography",
            "deck-patterns",
            "deck-contract",
            "slides",
        ],
    );
    let cjk_assembled = design_agent_system_prompt_with_skills(cjk_prompt);
    for (_, tail) in cjk_expected {
        assert!(
            cjk_assembled.contains(tail),
            "the mixed CJK logo/deck assembly lost contract tail {tail:?}"
        );
    }
}

#[test]
fn existing_web_app_contract_still_resolves_byte_complete() {
    let prompt = "Design a SaaS web app admin workspace with a populated client table";
    assert_resolves_in_full(prompt, &[("web-app", WEB_APP_TAIL)]);
    assert_assembly_contains_full(prompt, &["web-app"]);

    let assembled = design_agent_system_prompt_with_skills(prompt);
    assert!(
        assembled.contains(WEB_APP_TAIL),
        "the real design-agent assembly lost the web-app contract tail"
    );
}

#[test]
fn logo_and_web_app_contracts_survive_the_same_production_assembly() {
    let prompt = "Compare logo wordmark candidates and apply the selected brand lockup to a SaaS web app admin workspace";
    let expected = [("logo-contract", LOGO_TAIL), ("web-app", WEB_APP_TAIL)];
    assert_resolves_in_full(prompt, &expected);
    assert_assembly_contains_full(prompt, &["logo-contract", "web-app"]);

    let assembled = design_agent_system_prompt_with_skills(prompt);
    for (_, tail) in expected {
        assert!(
            assembled.contains(tail),
            "the mixed logo/web-app assembly lost contract tail {tail:?}"
        );
    }
}
