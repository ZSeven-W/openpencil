use crate::{
    design_agent_system_prompt_with_skills, get_skill_by_name, resolve_skills, DropReason, Phase,
    ResolveOptions, SkillCategory, SkillTrigger,
};
use jian_core::binding::{BindingTarget, InvalidationKind};
use serde_json::Value;
use std::collections::BTreeMap;

const SKILL_NAME: &str = "scroll-orchestration";
const REQUIRED_TRIGGERS: &[&str] = &[
    "滚动",
    "视差",
    "parallax",
    "scroll",
    "sticky",
    "入场动画",
    "交错",
    "stagger",
];
const CONTRACT_FIELDS: &[&str] = &["stickyChildren", "pin", "bindings", "events", "lifecycle"];

fn source() -> &'static crate::loader::SkillEntry {
    get_skill_by_name(SKILL_NAME).expect("scroll-orchestration skill must be registered")
}

fn json_fences(content: &str) -> Vec<&str> {
    let marker = "```json\n";
    let mut rest = content;
    let mut blocks = Vec::new();
    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let end = after
            .find("\n```")
            .expect("every scroll recipe JSON fence must close");
        blocks.push(&after[..end]);
        rest = &after[end + "\n```".len()..];
    }
    blocks
}

fn count_key(value: &Value, key: &str) -> usize {
    match value {
        Value::Object(object) => {
            usize::from(object.contains_key(key))
                + object
                    .values()
                    .map(|child| count_key(child, key))
                    .sum::<usize>()
        }
        Value::Array(items) => items.iter().map(|item| count_key(item, key)).sum(),
        _ => 0,
    }
}

fn contract_field_counts(value: &Value) -> BTreeMap<&'static str, usize> {
    CONTRACT_FIELDS
        .iter()
        .map(|field| (*field, count_key(value, field)))
        .collect()
}

fn validate_action_lists(value: &Value, recipe: usize) {
    match value {
        Value::Object(object) => {
            for container in ["events", "lifecycle"] {
                if let Some(handlers) = object.get(container).and_then(Value::as_object) {
                    for (hook, list) in handlers {
                        if hook.starts_with("on") {
                            jian_core::action::default_registry()
                                .borrow()
                                .parse_list(list)
                                .unwrap_or_else(|error| {
                                    panic!(
                                        "recipe #{recipe} has an invalid {container}.{hook} action list: {error}"
                                    )
                                });
                        }
                    }
                }
            }
            for child in object.values() {
                validate_action_lists(child, recipe);
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_action_lists(item, recipe);
            }
        }
        _ => {}
    }
}

fn validate_scroll_bindings(value: &Value, has_scroll_ancestor: bool, recipe: usize) {
    let Some(object) = value.as_object() else {
        return;
    };
    let owns_scroll = object
        .get("events")
        .and_then(|events| events.get("onScroll"))
        .and_then(Value::as_array)
        .is_some_and(|actions| !actions.is_empty());
    let has_scroll_ancestor = has_scroll_ancestor || owns_scroll;

    if let Some(bindings) = object.get("bindings").and_then(Value::as_object) {
        for (property, expression) in bindings {
            let source = expression.as_str().unwrap_or_else(|| {
                panic!("recipe #{recipe} binding {property:?} must be an expression string")
            });
            jian_core::expression::Expression::compile(source).unwrap_or_else(|error| {
                panic!("recipe #{recipe} binding {property:?} does not compile: {error:?}")
            });
            if source.contains("$scroll") {
                assert!(
                    has_scroll_ancestor,
                    "recipe #{recipe} binding {property:?} uses $scroll without an onScroll ancestor"
                );
                let target = BindingTarget::parse(property).unwrap_or_else(|| {
                    panic!("recipe #{recipe} uses unknown binding target {property:?}")
                });
                assert_eq!(
                    target.invalidation(),
                    InvalidationKind::PaintOnly,
                    "recipe #{recipe} teaches forbidden $scroll binding target {property:?}"
                );
            }
        }
    }

    if let Some(sticky) = object.get("stickyChildren").and_then(Value::as_array) {
        assert!(
            owns_scroll,
            "recipe #{recipe} stickyChildren must live on its onScroll container"
        );
        let child_ids: Vec<&str> = object
            .get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|child| child.get("id").and_then(Value::as_str))
            .collect();
        for id in sticky.iter().filter_map(Value::as_str) {
            assert!(
                child_ids.contains(&id),
                "recipe #{recipe} sticky child {id:?} is not a direct child"
            );
        }
    }

    if let Some(children) = object.get("children").and_then(Value::as_array) {
        for child in children {
            validate_scroll_bindings(child, has_scroll_ancestor, recipe);
        }
    }
}

#[test]
fn required_triggers_resolve_the_complete_skill_without_budget_damage() {
    let source = source();
    assert_eq!(source.meta.category, SkillCategory::Domain);
    let SkillTrigger::Keywords(keywords) = &source.meta.trigger else {
        panic!("scroll-orchestration must be keyword-gated");
    };
    for trigger in REQUIRED_TRIGGERS {
        assert!(
            keywords.iter().any(|keyword| keyword == trigger),
            "missing required trigger {trigger:?}"
        );
        let context = resolve_skills(Phase::Generation, trigger, &ResolveOptions::default());
        let resolved = context
            .skills
            .iter()
            .find(|skill| skill.meta.name == SKILL_NAME)
            .unwrap_or_else(|| panic!("trigger {trigger:?} must resolve scroll-orchestration"));
        assert!(!resolved.truncated, "skill truncated for {trigger:?}");
        assert_eq!(resolved.content, source.content);
    }

    for stress in [
        "Build a landing page with scroll parallax, sticky chapter navigation, and staggered entrance animation",
        "Build an interactive prototype with scroll progress and sticky navigation",
    ] {
        let context = resolve_skills(Phase::Generation, stress, &ResolveOptions::default());
        assert!(
            context.skills.iter().all(|skill| !skill.truncated),
            "stress prompt {stress:?} truncated skills: {:?}",
            context
                .skills
                .iter()
                .filter(|skill| skill.truncated)
                .map(|skill| skill.meta.name.as_str())
                .collect::<Vec<_>>()
        );
        assert!(
            context
                .report
                .dropped
                .iter()
                .all(|skill| skill.reason != DropReason::BudgetExhausted),
            "stress prompt {stress:?} budget-dropped a matched skill: report={:?}",
            context.report
        );
        let resolved = context
            .skills
            .iter()
            .find(|skill| skill.meta.name == SKILL_NAME)
            .expect("stress prompt resolves scroll-orchestration");
        assert_eq!(resolved.content, source.content);

        let assembled = design_agent_system_prompt_with_skills(stress);
        assert!(assembled.contains(source.content.trim()));
        assert!(assembled.contains("END SCROLL ORCHESTRATION"));
    }
}

#[test]
fn every_recipe_is_valid_schema_and_valid_runtime_vocabulary() {
    let blocks = json_fences(&source().content);
    assert!(
        (5..=8).contains(&blocks.len()),
        "expected 5-8 complete JSON recipes, got {}",
        blocks.len()
    );

    let mut animate_actions = 0;
    let mut scroll_bindings = 0;
    let mut sticky_fields = 0;
    for (index, block) in blocks.iter().enumerate() {
        let recipe = index + 1;
        let node: Value = serde_json::from_str(block)
            .unwrap_or_else(|error| panic!("recipe #{recipe} is invalid JSON: {error}"));
        assert!(node.get("type").and_then(Value::as_str).is_some());
        assert!(node.get("id").and_then(Value::as_str).is_some());

        validate_action_lists(&node, recipe);
        validate_scroll_bindings(&node, false, recipe);
        animate_actions += count_key(&node, "animate");
        scroll_bindings += block.matches("$scroll").count();
        sticky_fields += count_key(&node, "stickyChildren") + count_key(&node, "pin");

        let wrapped = serde_json::json!({
            "version": "1.1",
            "formatVersion": "1.1",
            "id": format!("scroll-recipe-{recipe}"),
            "app": {
                "name": format!("scroll-recipe-{recipe}"),
                "version": "1",
                "id": format!("scroll-recipe-{recipe}")
            },
            "children": [node]
        });
        let loaded = jian_ops_schema::load_str(&wrapped.to_string()).unwrap_or_else(|error| {
            panic!("recipe #{recipe} is not a valid .op document subtree: {error}")
        });
        assert!(
            loaded.warnings.is_empty(),
            "recipe #{recipe} emitted schema warnings: {:?}",
            loaded.warnings
        );
        let round_trip = serde_json::to_value(&loaded.value).expect("serialize loaded recipe");
        assert_eq!(
            contract_field_counts(&wrapped),
            contract_field_counts(&round_trip),
            "recipe #{recipe} lost a runtime contract field during typed round-trip"
        );
    }
    assert!(animate_actions >= 3, "property-anim teaching is missing");
    assert!(scroll_bindings >= 5, "scroll-progress teaching is missing");
    assert!(sticky_fields >= 3, "sticky/pin teaching is missing");
}

#[test]
fn vocabulary_and_scope_boundaries_are_explicit() {
    let content = &source().content;
    for field in ["offset", "maxOffset", "progress", "direction"] {
        assert!(content.contains(field), "missing $scroll field {field}");
    }
    for property in [
        "opacity",
        "x",
        "y",
        "rotation",
        "scaleX",
        "scaleY",
        "fill",
        "stroke",
        "cornerRadius",
        "width",
        "height",
    ] {
        assert!(
            content.contains(property),
            "missing animate property {property}"
        );
    }
    for easing in ["linear", "ease", "ease_in", "ease_out", "ease_in_out"] {
        assert!(content.contains(easing), "missing easing {easing}");
    }
    assert!(content.contains("vertical viewport scroll orchestration"));
    assert!(content.contains("not an ordinary horizontal card rail"));
}

#[test]
fn base_layout_no_longer_contradicts_the_sticky_runtime() {
    let layout = get_skill_by_name("layout").expect("layout skill registered");
    assert!(
        !layout
            .content
            .contains("no `position: fixed` / `position: sticky`"),
        "always-on layout skill still denies the supported sticky contract"
    );
    assert!(layout.content.contains("stickyChildren"));
    assert!(layout.content.contains("pin: true"));

    let interactivity = get_skill_by_name("interactivity").expect("interactivity registered");
    assert_eq!(
        interactivity.meta.trigger,
        SkillTrigger::Keywords(
            [
                "interactive",
                "interactivity",
                "clickable",
                "functional",
                "prototype",
                "stateful",
                "交互",
                "可交互",
                "原型",
                "可点击",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        ),
        "the deep-scroll topic must not regress basic interactivity routing"
    );
}
