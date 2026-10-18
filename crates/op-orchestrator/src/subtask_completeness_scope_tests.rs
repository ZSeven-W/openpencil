use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use serde_json::{json, Value};

const RETAINED_SECTIONS: &str =
    include_str!("../tests/fixtures/subtask_completeness/home_sections_1018.json");
const RETAINED_ADDRESS: &str =
    include_str!("../tests/fixtures/subtask_completeness/home_address_1018.js");

fn retained_sections() -> Vec<Subtask> {
    let mut tasks: Vec<Subtask> = serde_json::from_str(RETAINED_SECTIONS).unwrap();
    for task in &mut tasks {
        task.id_prefix = task.id.clone();
    }
    tasks
}

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"section","label":"Content","elements":elements,
        "region":{"width":375,"height":400}
    }))
    .unwrap()
}

fn item(index: usize) -> Value {
    json!({"type":"frame","id":format!("item-{index}"),"children":[
        {"type":"text","id":format!("text-{index}"),"content":format!("条目 {index}")}
    ]})
}

fn sink(children: Vec<Value>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[
            {"type":"frame","id":"section","layout":"vertical","children":children}
        ]}))
        .unwrap(),
    );
    sink
}

fn outcome() -> SubtaskOutcome {
    SubtaskOutcome {
        id: "section".into(),
        node_count: 1,
        inserted_root_ids: vec!["section".into()],
        error: None,
        headline: None,
        subtask: None,
    }
}

fn grid(missing: bool) -> Vec<Value> {
    (0..3)
        .map(|row| {
            let cells = 3 - usize::from(missing && row == 2);
            json!({"type":"frame","id":format!("row-{row}"),"layout":"horizontal",
                "children":(0..cells).map(|column|item(row*3+column)).collect::<Vec<_>>()})
        })
        .collect()
}

#[test]
fn retained_address_hierarchy_reference_has_no_local_quota() {
    let address = &retained_sections()[0];
    let sink = sink(vec![
        json!({"type":"text","id":"address","content":"天府二街 66 号"}),
    ]);
    assert_eq!(expected_item_count(address), None);
    assert!(incomplete_attempt(&sink, address, &outcome()).is_none());
}

#[test]
fn a_local_address_list_still_requires_both_items_alongside_the_reference() {
    let mut address = retained_sections().remove(0);
    address
        .elements
        .as_mut()
        .unwrap()
        .push_str("；本区显示2条快捷地址");
    assert_eq!(expected_item_count(&address), Some(2));
    assert!(incomplete_attempt(&sink((0..2).map(item).collect()), &address, &outcome()).is_none());
    let failure = incomplete_attempt(&sink(vec![item(0)]), &address, &outcome()).unwrap();
    assert_eq!(failure.expected, 2);
    assert!(failure.delivered < 2);
}

#[test]
fn explicit_foreign_or_excluded_items_do_not_impose_local_quotas() {
    for elements in [
        "主角让位给分类九宫格",
        "主角让位给3x3分类网格",
        "主角让位给9条商品列表",
        "分类九宫格由 home-categories 负责",
        "9条商品列表由其他区块负责",
        "分类九宫格在其他区块展示",
        "其他区块显示9条商品列表",
        "不包含分类九宫格",
        "不要生成9条商品列表",
        "仅供参考：3x3分类网格",
        "Keep secondary to the 3x3 category grid",
        "Another section displays 9 cards and 3 rows",
        "9 cards in another section",
        "list of twelve items belongs to another section",
        "The 3x3 grid is owned by categories",
        "Do not render 9 cards",
        "For context only: 3x3 grid",
    ] {
        assert_eq!(expected_item_count(&task(elements)), None, "{elements}");
    }
}

#[test]
fn local_promises_survive_hierarchy_negation_and_neighbor_references() {
    for (elements, expected) in [
        ("展示2条快捷地址，主角让位给分类九宫格", 2),
        ("主角让位给分类九宫格，但保留2条快捷地址", 2),
        ("分类九宫格由 home-categories 负责；本区显示2条快捷地址", 2),
        ("不包含分类九宫格，但展示2条快捷地址", 2),
        ("ARCHETYPE content-stream — 本区显示九宫格", 9),
        ("显示9项分类九宫格；标题让位给网格", 9),
        ("3x3分类网格，主角让位给另一部分的4x4网格", 9),
        ("不要省略分类九宫格", 9),
        ("分类九宫格由本区负责", 9),
        ("分类九宫格由 section 负责", 9),
        ("参考已有风格生成3x3网格", 9),
        ("show 2 cards and remain secondary to the 3x3 grid", 2),
        ("secondary to the 3x3 grid but show 2 cards", 2),
        (
            "9 cards belong to another section while this section shows 2 cards",
            2,
        ),
        ("Do not render 9 cards but show 2 cards", 2),
        ("Do not omit the 3x3 category grid", 9),
        ("3x3 grid below the address bar", 9),
        ("2 rows below the title", 2),
        ("9 cards owned by this section", 9),
        ("9 cards owned by Content", 9),
        (
            "show 1,000 rows; secondary to the sibling section's 3x3 grid",
            1000,
        ),
    ] {
        assert_eq!(
            expected_item_count(&task(elements)),
            Some(expected),
            "{elements}"
        );
    }
    let mut local_grid = task("Keep secondary to the hero");
    local_grid.label = "分类九宫格".into();
    assert_eq!(expected_item_count(&local_grid), Some(9));
}

#[test]
fn ownership_qualifies_only_its_coordinated_item_phrase() {
    for elements in [
        "本区展示2条快捷地址及由 home-categories 负责的分类九宫格",
        "本区展示2条快捷地址及分类九宫格由 home-categories 负责",
        "由 home-categories 负责的分类九宫格及2条快捷地址",
        "Show 2 cards and a 3x3 grid owned by categories",
        "A 3x3 grid owned by categories and 2 cards",
        "2 cards owned by this section and a 3x3 grid owned by categories",
        "Show 2 cards and 9 cards in another section",
        "9 cards in another section and 2 cards",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(2), "{elements}");
        assert!(incomplete_attempt(&sink((0..2).map(item).collect()), &task, &outcome()).is_none());
        let failure = incomplete_attempt(&sink(vec![item(0)]), &task, &outcome()).unwrap();
        assert_eq!(failure.expected, 2, "{elements}");
        assert!(failure.delivered < 2, "{elements}");
    }
}

#[test]
fn coordinated_words_inside_current_owner_names_do_not_end_ownership() {
    for (label, elements, expected) in [
        ("地址栏与搜索", "2条快捷地址由地址栏与搜索负责", 2),
        ("Pricing and Plans", "9 cards owned by Pricing and Plans", 9),
        (
            "Pricing and Plans",
            "2 cards owned by Pricing and Plans and a 3x3 grid owned by categories",
            2,
        ),
    ] {
        let mut task = task(elements);
        task.label = label.into();
        assert_eq!(expected_item_count(&task), Some(expected), "{elements}");
    }
}

#[test]
fn negated_quantity_comparisons_and_local_layout_keep_real_obligations() {
    for elements in [
        "不要生成少于9条商品",
        "不展示不足9条商品",
        "Do not render fewer than 9 cards",
        "Must not display less than 9 cards",
        "Show 9 cards in separate blocks",
        "9张商品卡各在独立区块中显示",
    ] {
        let task = task(elements);
        assert_eq!(expected_item_count(&task), Some(9), "{elements}");
        assert!(incomplete_attempt(&sink((0..9).map(item).collect()), &task, &outcome()).is_none());
        let failure =
            incomplete_attempt(&sink((0..8).map(item).collect()), &task, &outcome()).unwrap();
        assert_eq!((failure.expected, failure.delivered), (9, 8), "{elements}");
    }
}

#[test]
fn local_grid_shape_keeps_real_nine_cell_checks_with_foreign_references() {
    let categories = retained_sections().remove(1);
    for task in [
        categories,
        task("3x3 category grid; another section owns 12 cards"),
        task("本区分类九宫格；主角让位给另一部分的4x4网格"),
    ] {
        assert_eq!(expected_item_count(&task), Some(9));
        assert!(incomplete_attempt(&sink(grid(false)), &task, &outcome()).is_none());
        let failure = incomplete_attempt(&sink(grid(true)), &task, &outcome()).unwrap();
        assert_eq!((failure.expected, failure.delivered), (9, 8));
    }
}

#[test]
fn foreign_grid_shape_cannot_inflate_delivery_for_an_ordinary_local_list() {
    let task = task("Show 4 cards; secondary to another section's 3x3 grid");
    assert_eq!(expected_item_count(&task), Some(4));
    // Three repeated row frames are not a four-item list. The foreign grid
    // must not let delivery sum their nine nested cells as a local grid.
    let failure = incomplete_attempt(&sink(grid(false)), &task, &outcome()).unwrap();
    assert_eq!((failure.expected, failure.delivered), (4, 3));
}

#[test]
fn literal_copy_and_metadata_still_do_not_own_items() {
    for elements in [
        "标题『九宫格』；本区显示2条快捷地址",
        "标题『list of twelve items』；本区显示2条快捷地址",
        "meta row (12 exercises); show 2 cards",
    ] {
        assert_eq!(expected_item_count(&task(elements)), Some(2));
    }
}

#[test]
fn retained_header_and_real_grid_each_finish_the_retry_ladder_on_first_attempt() {
    let tasks = retained_sections();
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Home".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: tasks.clone(),
        shared_context: None,
        style_guide_name: None,
        palette: None,
    };
    let request = DesignRequest {
        prompt: "地址栏与分类九宫格".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let category_script = format!(
        "I(null,{});",
        json!({
            "type":"frame","name":"分类九宫格","layout":"vertical","children":grid(false)
        })
    );
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(RETAINED_ADDRESS.into()),
        ScriptResponse::Text(category_script),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    for task in &tasks {
        let outcome =
            futures::executor::block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
                task,
                &plan,
                &request,
                &llm,
                &mut sink,
                &AbortFlag::new(),
                crate::model_profile::ModelTier::Full,
                None,
                &GeometryEchoBudget::new(0),
                &mut |event| events.push(event),
                &[],
            ));
        assert!(outcome.error.is_none(), "{outcome:?}");
        assert!(outcome.node_count > 0);
        assert!(incomplete_attempt(&sink, task, &outcome).is_none());
    }
    assert_eq!(llm.user_prompts().len(), 2);
    assert!(!events.iter().any(|event| matches!(
        event,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
}
