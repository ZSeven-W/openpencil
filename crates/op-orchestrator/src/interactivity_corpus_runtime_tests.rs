//! Compile the action examples actually sent to the model with Preview's parser.

use jian_core::action::default_registry;
use serde_json::Value;

#[test]
fn interactivity_skill_event_examples_compile_with_the_runtime_registry() {
    let skill = op_ai_skills::get_skill_by_name("interactivity").unwrap();
    let registry = default_registry();
    let mut checked = 0;
    for block in skill.content.split("```json").skip(1) {
        let example = block.split("```").next().unwrap().trim();
        if !example.contains("\"events\"") {
            continue;
        }
        let object = if example.starts_with('"') {
            format!("{{{example}}}")
        } else {
            example.to_owned()
        };
        let value: Value = serde_json::from_str(&object).expect("event example is valid JSON");
        for actions in value["events"].as_object().unwrap().values() {
            registry
                .borrow()
                .parse_list(actions)
                .unwrap_or_else(|error| panic!("skill action example fails: {error}; {example}"));
            checked += 1;
        }
    }
    assert!(checked >= 4, "event examples were not exercised");
}

#[test]
fn interactivity_skill_inline_literal_actions_compile_with_the_runtime_registry() {
    let skill = op_ai_skills::get_skill_by_name("interactivity").unwrap();
    let registry = default_registry();
    let examples = regex::Regex::new(r#"`(\{"(?:set|toast)":[^`]+\})`"#).unwrap();
    let mut checked = 0;
    for example in examples.captures_iter(&skill.content) {
        let value: Value =
            serde_json::from_str(&example[1]).expect("literal example is valid JSON");
        registry
            .borrow()
            .parse_single(&value)
            .unwrap_or_else(|error| {
                panic!("skill literal example fails: {error}; {}", &example[1])
            });
        checked += 1;
    }
    assert!(
        checked >= 2,
        "literal set/toast examples were not exercised"
    );
}
