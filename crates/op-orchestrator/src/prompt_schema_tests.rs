use super::*;

const BRIEFS: [&str; 4] = [
    "外卖 App 两屏可交互（375×812）：首页(地址栏+分类九宫格+限时横幅+商家列表五条)、商家详情(头图+菜品分类横滑+菜品列表六条+购物车条)。",
    "电商商品详情 App 两屏可交互（375×812）：商品页(大图轮播占位+价格+规格选择+评价三条+底部加购条)、购物车页(商品两条+金额汇总+结算)。",
    "a marketing landing page for a fintech product",
    "帮我做一个 12 页的产品培训课件 PPT",
];

// Measured assembled prompts at 1c57fa351 with the same briefs/model. Replacing
// the conflicting shapes must not quietly grow any planning mode's budget.
const PREVIOUS_SIZES: [[(usize, usize); 3]; 4] = [
    [(34280, 33783), (33071, 32576), (8483, 8461)],
    [(34277, 33780), (33071, 32576), (8484, 8462)],
    [(33779, 33296), (32632, 32151), (7825, 7807)],
    [(31773, 31292), (30631, 30152), (8355, 8327)],
];

#[test]
fn planning_schema_prompt_budget() {
    for (brief_index, brief) in BRIEFS.into_iter().enumerate() {
        for (mode_index, mode) in [
            PlanningMode::Rich,
            PlanningMode::Minimal,
            PlanningMode::Compact,
        ]
        .into_iter()
        .enumerate()
        {
            let request = DesignRequest {
                prompt: brief.into(),
                model: Some("glm-5.3-flash".into()),
                ..Default::default()
            };
            let call = build_orchestrator_prompt(&request, mode, AbortFlag::new()).call_request;
            let size = (call.system_prompt.len(), call.system_prompt.chars().count());
            let previous = PREVIOUS_SIZES[brief_index][mode_index];
            assert!(
                size.0 <= previous.0 && size.1 <= previous.1,
                "{mode:?} grew from {previous:?} to {size:?}"
            );
            println!("{mode:?} bytes/chars {previous:?} -> {size:?}");
            assert_complete_shape(&call.system_prompt);
        }
    }
}

fn assert_complete_shape(prompt: &str) {
    let shapes: Vec<_> = prompt
        .lines()
        .filter(|line| line.starts_with("{\"rootFrame\":"))
        .collect();
    assert_eq!(shapes.len(), 1, "one authoritative full example per prompt");
    let json: serde_json::Value = serde_json::from_str(shapes[0]).unwrap();
    assert!(json["subtasks"].is_array());
    assert!(json["sharedContext"].is_object());
    assert!(json["sharedContext"].get("subtasks").is_none());
    let plan = crate::plan::parse_plan(shapes[0]).unwrap();
    for entity in plan.shared_context.unwrap().example_entities {
        for id in entity.used_by {
            assert!(
                plan.subtasks.iter().any(|task| task.id == id),
                "unresolved example id {id}"
            );
        }
    }
    assert!(prompt.contains("use {} when nothing is shared"));
    assert!(prompt.contains("Each usedBy entry must exactly equal an id"));
    assert!(!prompt.contains("Omit sharedContext"));
}

#[test]
fn standalone_compact_prompt_has_the_same_complete_shape() {
    assert_complete_shape(
        &crate::compact_prompt::build_compact_planning_prompt(BRIEFS[0], None, None).system,
    );
}
