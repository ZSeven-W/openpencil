use super::*;
use crate::subtask_completeness::named_options_tests::{outcome, pill, section, sink, task};
use serde_json::{json, Value};

#[test]
fn retained_options_do_not_turn_interaction_directions_into_product_choices() {
    let task: Subtask = serde_json::from_str(include_str!(
        "../tests/fixtures/subtask_completeness/product-options-1019-task.json"
    ))
    .unwrap();
    let groups = contracts(&task);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].labels, ["雾白", "燕麦", "深咖"]);
    assert_eq!(groups[1].labels, ["S", "M", "L", "XL"]);
    let raw =
        include_str!("../tests/fixtures/subtask_completeness/product-options-1019-attempt1.js");
    let program = op_mcp::script_runner::run_generation_script_to_program(raw).unwrap();
    let (nodes, _) = crate::program_gen::run_program_to_forest(&program).unwrap();
    let ids = nodes.iter().map(|n| n.id_str().to_owned()).collect();
    let mut result = outcome(&[]);
    result.inserted_root_ids = ids;
    let saved = sink(
        nodes
            .iter()
            .map(|n| serde_json::to_value(n).unwrap())
            .collect(),
    );
    assert_eq!(inspect(&saved, &task, &result).unwrap(), 4);
    let missing = sink(vec![section(vec![
        options("颜色", &["雾白", "燕麦"]),
        options("尺码", &["S", "M", "L", "XL"]),
    ])]);
    let failure = inspect(&missing, &task, &outcome(&["section"])).unwrap_err();
    assert_eq!((failure.expected, failure.delivered), (3, 2));
    assert!(failure.feedback.contains("深咖"));
    assert!(!failure.feedback.contains("onTap"));
}

#[test]
fn quoted_behavior_words_are_still_literal_option_names() {
    let task = task("规格选项（'onTap'/'transition'/'pressed'）");
    let groups = contracts(&task);
    assert_eq!(groups[0].labels, ["onTap", "transition", "pressed"]);
}

const PLAN: &str = include_str!("test_fixtures/product_prose_options_1018_task.json");
const RAW: &str = include_str!("test_fixtures/product_prose_options_1018_salvage.js");

#[test]
fn retained_prose_options_detect_the_actual_blank_salvage() {
    let task: Subtask = serde_json::from_str(PLAN).unwrap();
    let program = op_mcp::script_runner::run_generation_script_to_program(RAW).unwrap();
    let (nodes, _) = crate::program_gen::run_program_to_forest(&program).unwrap();
    let ids = nodes
        .iter()
        .map(|node| node.id_str().to_owned())
        .collect::<Vec<_>>();
    let sink = sink(
        nodes
            .iter()
            .map(|node| serde_json::to_value(node).unwrap())
            .collect(),
    );
    let mut result = outcome(&[]);
    result.inserted_root_ids = ids;
    let failure = inspect(&sink, &task, &result).expect_err("blank named choices are incomplete");
    assert_eq!((failure.expected, failure.delivered), (2, 0));
    assert!(failure.feedback.contains("奶白") && failure.feedback.contains("墨绿"));
}

fn options(heading: &str, labels: &[&str]) -> Value {
    json!({"type":"frame","id":format!("group-{heading}"),"children":[
        {"type":"text","id":format!("heading-{heading}"),"content":heading},
        {"type":"frame","id":format!("row-{heading}"),"children":labels.iter().enumerate()
            .map(|(i,label)| pill(&format!("pill-{heading}-{i}"),label)).collect::<Vec<_>>()}
    ]})
}

#[test]
fn prose_groups_require_their_own_names_under_their_own_heading() {
    let task: Subtask = serde_json::from_str(PLAN).unwrap();
    let mut root = section(vec![]);
    root["children"] = json!([options("颜色", &["奶白", "墨绿"])]);
    let failure = inspect(&sink(vec![root.clone()]), &task, &outcome(&["section"]))
        .expect_err("a complete color group cannot substitute the capacity group");
    assert!(failure.feedback.contains("450ml") && failure.feedback.contains("600ml"));
    root["children"]
        .as_array_mut()
        .unwrap()
        .push(options("容量", &["450ml", "600ml"]));
    assert_eq!(
        inspect(&sink(vec![root]), &task, &outcome(&["section"])).unwrap(),
        2
    );
}

#[test]
fn prose_copy_examples_graphic_only_and_foreign_owners_remain_out_of_scope() {
    for elements in [
        "标题文案「颜色'奶白/墨绿'胶囊选项」",
        "标题文案「'颜色'+'奶白/墨绿'胶囊选项」",
        "其他区域展示'颜色'+'奶白/墨绿'胶囊选项",
        "'颜色'+'奶白/墨绿'胶囊选项，由其他区块负责",
        "'颜色'+'奶白/墨绿'胶囊选项，无文字标签",
        "'颜色'+'例如奶白/墨绿'胶囊选项",
        "'颜色'+'奶白/奶白'胶囊选项",
        "'颜色'+'奶白/墨绿等'胶囊选项",
        "'颜色'+'奶白'胶囊选项",
        "元信息（'颜色'+'奶白/墨绿'胶囊选项）",
    ] {
        assert_eq!(
            inspect(
                &sink(vec![section(vec![])]),
                &task(elements),
                &outcome(&["section"])
            )
            .unwrap(),
            0,
            "{elements}"
        );
    }
}
