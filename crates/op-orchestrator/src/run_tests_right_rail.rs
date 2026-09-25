//! arena-w01 end-to-end (no LLM): a desktop sidebar plan whose last subtask
//! is a right-side task drawer. The drawer must generate into a third shell
//! column BESIDE the kanban, not stack under it and double the page height.

use super::*;

const W01_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "项目管理工作台", "width": 1440, "height": 900,
                 "layout": "horizontal", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "sidebar", "label": "项目侧栏", "region": { "width": 260, "height": 900 },
      "elements": "品牌标识, 新建项目按钮, 项目列表" },
    { "id": "top-tabs", "label": "顶部标签切换", "region": { "width": 1180, "height": 72 },
      "elements": "页面标题, 视图标签栏（看板、列表、日历）" },
    { "id": "kanban-columns", "label": "看板三列各四张任务卡", "region": { "width": 1180, "height": 700 },
      "elements": "三列任务分组（待处理、进行中、已完成）, 任务卡" },
    { "id": "task-detail-drawer", "label": "右侧任务详情抽屉", "region": { "width": 420, "height": 900 },
      "elements": "抽屉顶部关闭按钮, 任务标题, 描述, 评论时间线, 保存按钮" }
  ]
}"##;

/// A section root carrying `items` sibling frames, so the subtask
/// completeness gate sees every promised repeated item delivered.
fn section(name: &str, height: u32, layout: &str, items: &[&str]) -> String {
    let kids: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                r#"{{"type":"frame","name":"{item}","width":"fill_container","height":{height},"layout":"vertical","children":[{{"type":"text","content":"{item}","fontSize":14}}]}}"#
            )
        })
        .collect();
    format!(
        r#"I(null, {{"type":"frame","name":"{name}","width":"fill_container","height":"fit_content","layout":"{layout}","gap":16,"children":[{}]}});"#,
        kids.join(",")
    )
}

fn find_named<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    if v["name"].as_str() == Some(name) {
        return Some(v);
    }
    v["children"]
        .as_array()?
        .iter()
        .find_map(|c| find_named(c, name))
}

#[test]
fn right_drawer_generates_beside_the_board_not_under_it() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(W01_PLAN_JSON.into()),
        // The coverage check may re-plan once; the same plan answers it.
        ScriptResponse::Text(W01_PLAN_JSON.into()),
        ScriptResponse::Text(section(
            "项目侧栏",
            120,
            "vertical",
            &["品牌", "新建项目", "项目列表"],
        )),
        ScriptResponse::Text(section(
            "顶部标签切换",
            56,
            "horizontal",
            &["页面标题", "视图标签栏"],
        )),
        ScriptResponse::Text(section(
            "看板三列各四张任务卡",
            640,
            "horizontal",
            &["待处理", "进行中", "已完成"],
        )),
        ScriptResponse::Text(section(
            "右侧任务详情抽屉",
            120,
            "vertical",
            &["抽屉顶部", "任务标题", "描述", "评论时间线", "保存按钮"],
        )),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_: Progress| {};
    let mut request = req();
    request.prompt = "项目管理 Web 应用主界面（1440×900）：侧栏项目列表、顶部标签切换、看板三列各四张任务卡、右侧任务详情抽屉打开状态。".into();
    request.validation_enabled = false;

    futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("w01 run ok");

    let root = serde_json::to_value(sink.state.active_children().first().expect("root")).unwrap();
    let columns: Vec<&str> = root["children"]
        .as_array()
        .expect("root columns")
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert_eq!(
        columns,
        vec!["Sidebar", "Main Content", "Right Panel"],
        "three-column shell"
    );
    let rail = find_named(&root, "Right Panel").expect("rail");
    assert!(
        find_named(rail, "右侧任务详情抽屉").is_some(),
        "the drawer lives in the right rail: {}",
        serde_json::to_string(rail).unwrap()
    );
    let main = find_named(&root, "Main Content").expect("main");
    assert!(find_named(main, "右侧任务详情抽屉").is_none());
    assert!(find_named(main, "看板三列各四张任务卡").is_some());
    let height = root["height"].as_f64().unwrap_or(0.0);
    assert!(
        height < 1000.0,
        "the drawer no longer stacks under the board, root height {height}"
    );
}
