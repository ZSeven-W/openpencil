//! Execute the exact shared teaching corpus against Jian, not a rewritten copy.

use crate::script_gen::parse_script;
use jian_core::{binding::BindingTarget, expression::Expression, Runtime};
use serde_json::Value;

fn recipes() -> Vec<String> {
    let skill = op_ai_skills::get_skill_by_name("interactivity").unwrap();
    assert!(op_ai_skills::budget::estimate_tokens(&skill.content) <= skill.meta.budget);
    skill
        .content
        .split("```js\n")
        .skip(1)
        .map(|tail| tail.split_once("```").unwrap().0.to_owned())
        .collect()
}

struct Recipe {
    nodes: Value,
    runtime: Runtime,
}

impl Recipe {
    fn load(script: &str) -> Self {
        let (nodes, state) =
            parse_script(script).expect("corpus uses the real generation protocol");
        let runtime = Runtime::new();
        for (key, entry) in state {
            runtime.state.app_set(&key, entry.default.unwrap());
        }
        Self {
            nodes: serde_json::to_value(nodes).unwrap(),
            runtime,
        }
    }

    fn node(&self, name: &str) -> &Value {
        fn find<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
            if value["name"] == name {
                return Some(value);
            }
            value
                .as_array()
                .or_else(|| value["children"].as_array())?
                .iter()
                .find_map(|child| find(child, name))
        }
        find(&self.nodes, name).unwrap_or_else(|| panic!("missing {name}"))
    }

    fn tap(&self, name: &str) {
        let chain = self
            .runtime
            .actions
            .borrow()
            .parse_list(&self.node(name)["events"]["onTap"])
            .unwrap();
        let context = self.runtime.make_action_ctx();
        futures::executor::block_on(chain.run_serial(&context)).unwrap();
        assert!(context.take_warnings().is_empty());
    }

    fn bound(&self, name: &str, property: &str) -> Value {
        assert!(BindingTarget::parse(property).is_some());
        let source = self.node(name)["bindings"][property].as_str().unwrap();
        let (value, diagnostics) =
            Expression::compile(source)
                .unwrap()
                .eval(&self.runtime.state, None, None);
        assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
        value.0
    }

    fn money(&self, a: i32, b: i32) {
        for (name, expected) in [
            ("A quantity", a.to_string()),
            ("B quantity", b.to_string()),
            ("A amount", format!("¥{}", 24 * a)),
            ("B amount", format!("¥{}", 36 * b)),
            ("Subtotal", format!("¥{}", 24 * a + 36 * b)),
            ("Total", format!("¥{}", 24 * a + 36 * b)),
        ] {
            assert_eq!(self.bound(name, "content"), expected, "{name}");
        }
    }
}

#[test]
fn shared_recipes_reach_glm_generation_and_each_retry_without_changing_static_intent() {
    let plan: crate::plan::OrchestratorPlan = serde_json::from_value(serde_json::json!({
        "rootFrame":{"id":"shop","name":"Shop","width":375,"height":812},
        "subtasks":[{"id":"options","label":"Specifications","region":{"width":375,"height":120},
            "elements":"Two specification options with onTap selection"}]
    }))
    .unwrap();
    let corpus = op_ai_skills::get_skill_by_name("interactivity").unwrap();
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let request = crate::types::DesignRequest {
            prompt: "电商商品详情 App 两屏可交互：规格胶囊 onTap 切换，购物车两条商品数量加减与金额汇总。".into(),
            model: Some("glm-5.3-flash".into()),
            ..Default::default()
        };
        let (call, report) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &request,
            crate::types::AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(
            report
                .included
                .iter()
                .any(|skill| skill.name == "interactivity" && !skill.truncated),
            "{report:?}"
        );
        assert!(call.system_prompt.contains(&corpus.content));
        assert!(call
            .system_prompt
            .contains("static/mockup-only instructions take"));
    }
}

#[test]
fn shared_selection_recipe_updates_both_chips_and_labels_and_is_idempotent() {
    let examples = recipes();
    assert_eq!(examples.len(), 2);
    let selection = Recipe::load(&examples[0]);
    for selected in ["Cream", "Green", "Green", "Cream"] {
        selection.tap(&format!("Finish {selected}"));
        for label in ["Cream", "Green"] {
            let active = label == selected;
            assert_eq!(
                selection.bound(&format!("Finish {label}"), "fill"),
                if active { "#172A24" } else { "#F1F4F2" }
            );
            assert_eq!(
                selection.bound(&format!("Finish label {label}"), "fill"),
                if active { "#FFFFFF" } else { "#172A24" }
            );
        }
    }
}

#[test]
fn shared_cart_recipe_recomputes_all_amounts_for_each_item_and_minimum() {
    let cart = Recipe::load(&recipes()[1]);
    cart.money(1, 1);
    for (action, a, b) in [
        ("A minus", 1, 1),
        ("A plus", 2, 1),
        ("B plus", 2, 2),
        ("B plus", 2, 3),
        ("A minus", 1, 3),
        ("B minus", 1, 2),
        ("B minus", 1, 1),
        ("B minus", 1, 1),
    ] {
        cart.tap(action);
        cart.money(a, b);
    }
}

#[test]
fn recipe_static_exports_match_initial_bindings_and_actions_preserve_authored_nodes() {
    for script in recipes() {
        let recipe = Recipe::load(&script);
        let before = recipe.nodes.clone();
        fn check(recipe: &Recipe, nodes: &Value) {
            for node in nodes.as_array().unwrap() {
                if let Some(bindings) = node["bindings"].as_object() {
                    let name = node["name"].as_str().unwrap();
                    for property in bindings.keys() {
                        // Canonical fill serializes as a list, unlike bound color strings.
                        let authored = if property == "fill" {
                            node[property][0]["color"].clone()
                        } else {
                            node[property].clone()
                        };
                        assert_eq!(recipe.bound(name, property), authored, "{name}");
                    }
                }
                if node["children"].is_array() {
                    check(recipe, &node["children"]);
                }
            }
        }
        check(&recipe, &recipe.nodes);
        let action = if recipe.node_count() > 10 {
            "A plus"
        } else {
            "Finish Green"
        };
        recipe.tap(action);
        assert_eq!(recipe.nodes, before);
    }
}

impl Recipe {
    fn node_count(&self) -> usize {
        fn count(value: &Value) -> usize {
            value.as_array().map_or(0, |nodes| {
                nodes.iter().map(|node| 1 + count(&node["children"])).sum()
            })
        }
        count(&self.nodes)
    }
}
