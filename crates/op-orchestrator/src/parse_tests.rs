use super::*;
use op_editor_core::PenNodeExt as _;

#[test]
fn parse_nodes_reads_fenced_pennode_array() {
    // 一个最小的 frame 节点(canonical schema:`type` 标签 +
    // base 字段)。
    let text = r#"Sure, here are the nodes:
```json
[
  { "type": "frame", "id": "hero", "name": "Hero", "x": 0, "y": 0, "width": 1200, "height": 400, "children": [] }
]
```"#;
    let nodes = parse_nodes(text).expect("parse");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "hero");
}

#[test]
fn parse_nodes_rejects_empty_array() {
    assert!(parse_nodes("[]").is_err());
}

#[test]
fn parse_nodes_rejects_no_array() {
    assert!(parse_nodes("the model wrote prose only").is_err());
}

#[test]
fn parse_nodes_defaults_missing_image_src_to_empty_string() {
    let text = r#"[
  { "type": "image", "id": "photo", "name": "Restaurant photo", "x": 0, "y": 0, "width": 240, "height": 160 }
]"#;

    let nodes = parse_nodes(text).expect("missing image src should be tolerated");
    let PenNode::Image(image) = &nodes[0] else {
        panic!("expected image node");
    };
    assert_eq!(image.src, "");
}

#[test]
fn parse_nodes_skips_prose_bracket_before_fenced_array() {
    // 弱模型在真正 JSON 之前写了带 `[` 的推理散文。老逻辑取第一个
    // `[` 会抓到 `[step 1]` 报 "expected ident";新逻辑扫描所有平衡
    // 数组、取节点数最多的有效结果,跳过散文括号。
    let text = "Let me plan this [step 1]: build the card.\n```json\n[\n  { \"type\": \"frame\", \"id\": \"card\", \"name\": \"Card\", \"x\": 0, \"y\": 0, \"width\": 300, \"height\": 200, \"children\": [] }\n]\n```";
    let nodes = parse_nodes(text).expect("should skip prose bracket and parse real array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "card");
}

#[test]
fn parse_nodes_tolerates_one_bad_node_keeps_rest() {
    // 弱模型在节点数组里混入一个 `type:"solid"`(fill 类型当节点)。
    // 老的整组 from_value 会因这一个坏元素丢掉整段;新逻辑逐元素
    // 反序列化、跳过坏的、保留有效节点(实测 Dashboard charts-row)。
    let text = r##"[
          { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
          { "type": "solid", "color": "#06B6D4" },
          { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
        ]"##;
    let nodes = parse_nodes(text).expect("should keep the valid nodes, skip the bad one");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id_str(), "a");
    assert_eq!(nodes[1].id_str(), "b");
}

#[test]
fn parse_nodes_returns_outer_array_not_nested_children() {
    // 嵌套格式:外层只有 1 个 root frame,其 children 有 3 个。只收
    // 顶层数组,返回外层 [root],而不是内层 [a,b,c](Codex review)。
    let text = r#"[
          { "type": "frame", "id": "root", "name": "Root", "x": 0, "y": 0, "width": 300, "height": 200, "children": [
            { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "c", "name": "C", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
          ] }
        ]"#;
    let nodes = parse_nodes(text).expect("parse");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "root");
}

#[test]
fn parse_nodes_reads_jsonl_bare_objects() {
    // DeepSeek 输出 JSONL:```json 围栏里每行一个裸对象,无 [ ] 包裹。
    // 数组路径只看到内层 children:[] 空数组(失败),JSONL 回退按顶层
    // {...} 逐个解析。
    let text = "```json\n{\"type\":\"frame\",\"id\":\"a\",\"name\":\"A\",\"x\":0,\"y\":0,\"width\":100,\"height\":50,\"children\":[]}\n{\"type\":\"frame\",\"id\":\"b\",\"name\":\"B\",\"x\":0,\"y\":0,\"width\":100,\"height\":50,\"children\":[]}\n```";
    let nodes = parse_nodes(text).expect("JSONL fallback should parse bare objects");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id_str(), "a");
    assert_eq!(nodes[1].id_str(), "b");
}

#[test]
fn parse_nodes_bare_object_returns_top_level_not_nested_children() {
    // 单个裸对象(无数组包裹)带真实嵌套 children。合并深度判定下,
    // children:[...] 在 {} 内(深度>0)不被数组路径当节点数组,返回顶层
    // root 而不是内层 [a,b](Codex review)。
    let text = r#"{ "type": "frame", "id": "root", "name": "Root", "x": 0, "y": 0, "width": 300, "height": 200, "children": [
            { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
        ] }"#;
    let nodes = parse_nodes(text).expect("parse");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "root");
}

#[test]
fn parse_nodes_finds_array_after_unmatched_prose_bracket() {
    // 散文里有个不闭合的 `[`(如 "options [1, 2 …"),其后才是真数组。
    // 独立尝试每个 `[` + 不闭合即跳过,后面的真数组不被漏掉(Codex
    // review;修上一轮 combined-depth 引入的回归)。
    let text = "Consider options [a and b, then build:\n```json\n[{ \"type\": \"frame\", \"id\": \"hero\", \"name\": \"Hero\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 100, \"children\": [] }]\n```";
    let nodes =
        parse_nodes(text).expect("should find the real array after an unmatched prose bracket");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "hero");
}

#[test]
fn parse_nodes_reads_labelled_wrapper_object() {
    // 模型把节点数组包进带标签的对象 {"nodes":[...]}。colon-skip 曾把
    // "nodes":[...] 一并跳掉;现在按总节点数择优——wrapper 无 type 解析
    // 失败,数组路径取出 [a,b](Codex review)。
    let text = r#"```json
{ "nodes": [
  { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
  { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
] }
```"#;
    let nodes = parse_nodes(text).expect("labelled wrapper array should be parsed");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id_str(), "a");
    assert_eq!(nodes[1].id_str(), "b");
}

#[test]
fn parse_nodes_reads_ark_metrics_jsonl_regression() {
    // 实采:方舟 ark-code-latest 的 metrics 子任务 —— 31 行合法 JSONL、1 个
    // root(`_parent:null`)、所有 `_parent` 可解析。实测却 `0/0 roots valid`
    // 失败(benchmark 矩阵复现)。固化为回归 fixture。
    let text = include_str!("test_fixtures/ark_metrics.jsonl");
    let nodes = parse_nodes(text).expect("valid 31-node _parent JSONL must parse");
    assert_eq!(nodes.len(), 1, "single metrics-root after re-nest");
    assert_eq!(nodes[0].id_str(), "metrics-root");
    // 31 行里有 1 个重复 id(方舟自身),去重后 30 个 unique 节点。
    assert!(
        total_nodes(&nodes) >= 29,
        "expected ~30 nodes, got {}",
        total_nodes(&nodes)
    );
}

#[test]
fn resolve_numeric_design_token_table() {
    assert_eq!(
        resolve_numeric_design_token("$type-caption-size"),
        Some(12.0)
    );
    assert_eq!(
        resolve_numeric_design_token("$type-display-size"),
        Some(64.0)
    );
    assert_eq!(
        resolve_numeric_design_token("$type-body-line-height"),
        Some(1.5)
    );
    assert_eq!(
        resolve_numeric_design_token("$type-display-letter-spacing"),
        Some(-0.5)
    );
    assert_eq!(resolve_numeric_design_token("$spacing-3"), Some(12.0));
    assert_eq!(resolve_numeric_design_token("$radius-md"), Some(8.0));
    // weight tokens resolve to numbers too (FontWeight accepts a number);
    // unresolved weight tokens would degrade text to the default weight.
    assert_eq!(
        resolve_numeric_design_token("$type-body-weight"),
        Some(400.0)
    );
    assert_eq!(
        resolve_numeric_design_token("$type-display-weight"),
        Some(700.0)
    );
    assert_eq!(resolve_numeric_design_token("$type-h2-weight"), Some(600.0));
    // colors / sizing keywords stay strings (not numeric tokens).
    assert_eq!(resolve_numeric_design_token("$color-accent"), None);
    assert_eq!(resolve_numeric_design_token("fill_container"), None);
}

#[test]
fn normalize_resolves_numeric_token_and_wraps_bare_fill() {
    let mut v = serde_json::json!({
        "type":"text","id":"x",
        "fontSize":"$type-caption-size",
        "fontWeight":"$type-display-weight",
        // token-like strings in NON-numeric fields must stay strings — a
        // design-system showcase may legitimately display "$spacing-3" as text.
        "content":"$spacing-3",
        "name":"$type-caption-size",
        "fill":"$color-surface"
    });
    normalize_generated_node_json(&mut v);
    // Whole numbers serialize as integers so `fontWeight` (FontWeight::Number(u32))
    // accepts them — a float (700.0) would fail to deserialize.
    assert_eq!(v["fontSize"], serde_json::json!(12)); // numeric field → integer
    assert_eq!(v["fontWeight"], serde_json::json!(700)); // weight → integer, not 700.0
    assert!(v["fontWeight"].is_i64()); // not a float
    assert_eq!(v["content"], serde_json::json!("$spacing-3")); // text content preserved
    assert_eq!(v["name"], serde_json::json!("$type-caption-size")); // name preserved
    assert_eq!(
        v["fill"],
        serde_json::json!([{"type":"solid","color":"$color-surface"}])
    );
    // The normalized node must round-trip into the canonical schema — this is the
    // real regression guard: a float fontWeight would fail here.
    serde_json::from_value::<PenNode>(v)
        .expect("normalized node must deserialize into canonical PenNode");
}

#[test]
fn parse_nodes_renests_flat_parent_array() {
    // 扁平 `_parent` 数组(M2.7 形态):root + 一个横向 row + row 的 3 个
    // 子项,全是兄弟、靠 `_parent` 指父。移植 TS parseJsonlToTree 后应重组
    // 成树:root 直接子 = [row],row 子 = [a,b,c],而非 5 个扁平兄弟
    // (扁平 → 横向容器子项跑到 root 下 → 布局竖排破损,这正是回归点)。
    let text = r#"[
          { "type": "frame", "id": "root", "name": "Root", "x": 0, "y": 0, "width": 300, "height": 200, "layout": "vertical", "_parent": null },
          { "type": "frame", "id": "row", "name": "Row", "width": "fill_container", "height": "fit_content", "layout": "horizontal", "_parent": "root" },
          { "type": "frame", "id": "a", "name": "A", "width": 80, "height": 50, "_parent": "row" },
          { "type": "frame", "id": "b", "name": "B", "width": 80, "height": 50, "_parent": "row" },
          { "type": "frame", "id": "c", "name": "C", "width": 80, "height": 50, "_parent": "row" }
        ]"#;
    let nodes = parse_nodes(text).expect("flat _parent array should re-nest into a tree");
    assert_eq!(nodes.len(), 1, "single root after re-nest");
    assert_eq!(nodes[0].id_str(), "root");
    assert_eq!(
        crate::cleanup::count_descendants(&nodes[0]),
        4,
        "root subtree = row + a + b + c"
    );
}

#[test]
fn parse_nodes_renests_flat_parent_jsonl() {
    // DeepSeek 形态:JSONL 裸对象,每个带 `_parent`。重组后 root 含嵌套子树
    // (2 个直接子),而非 3 个扁平兄弟。
    let text = "```json\n{\"type\":\"frame\",\"id\":\"root\",\"name\":\"Root\",\"x\":0,\"y\":0,\"width\":300,\"height\":200,\"_parent\":null}\n{\"type\":\"frame\",\"id\":\"a\",\"name\":\"A\",\"width\":80,\"height\":50,\"_parent\":\"root\"}\n{\"type\":\"frame\",\"id\":\"b\",\"name\":\"B\",\"width\":80,\"height\":50,\"_parent\":\"root\"}\n```";
    let nodes = parse_nodes(text).expect("flat _parent JSONL should re-nest");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "root");
    assert_eq!(crate::cleanup::count_descendants(&nodes[0]), 2);
}

#[test]
fn parse_nodes_renest_orphan_parent_becomes_root() {
    // `_parent` 指向不存在的 id → 该节点当 root(TS parseJsonlToTree:
    // "Parent not found, treat as root")。两个节点都成 root。
    let text = r#"[
          { "type": "frame", "id": "a", "name": "A", "width": 80, "height": 50, "_parent": "ghost" },
          { "type": "frame", "id": "b", "name": "B", "width": 80, "height": 50, "_parent": null }
        ]"#;
    let nodes = parse_nodes(text).expect("orphan parent should fall back to root");
    assert_eq!(nodes.len(), 2);
}

#[test]
fn parse_nodes_renest_cycle_is_lossless() {
    // 病态输入:纯 `_parent` 环(a↔b)。两者互为父 → 没有天然 root。
    // 无丢失保证应把它们装配出来(环被 take 打断),不静默丢节点。
    let text = r#"[
          { "type": "frame", "id": "a", "name": "A", "width": 80, "height": 50, "_parent": "b" },
          { "type": "frame", "id": "b", "name": "B", "width": 80, "height": 50, "_parent": "a" }
        ]"#;
    let nodes = parse_nodes(text).expect("a _parent cycle must not drop all nodes");
    // a 先序、被提升为 root,b 作为 a 的子;总节点数 2(无丢失)。
    assert_eq!(total_nodes(&nodes), 2);
}

#[test]
fn parse_nodes_ignores_draft_inside_think_block() {
    // 推理模型(MiniMax-M3)在 <think> 里写**未完成的扁平草稿**,`</think>`
    // 之后才是真答案。应解析真答案(嵌套 root),而不是 think 里的扁平草稿
    // (否则得到 M3 实测的扁平 depth-2 残骸)。
    let text = r#"<think>
        Let me draft the cards: [
          {"type":"frame","id":"draft-a","name":"A","width":80,"height":50},
          {"type":"frame","id":"draft-b","name":"B","width":80,"height":50}
        ]
        and similarly for the rest...
        </think>
        [
          {"type":"frame","id":"root","name":"Root","x":0,"y":0,"width":300,"height":200,"layout":"horizontal","_parent":null},
          {"type":"frame","id":"a","name":"A","width":80,"height":50,"_parent":"root"},
          {"type":"frame","id":"b","name":"B","width":80,"height":50,"_parent":"root"}
        ]"#;
    let nodes = parse_nodes(text).expect("should parse the real answer after </think>");
    assert_eq!(
        nodes.len(),
        1,
        "real answer is 1 nested root, not the 2-node draft"
    );
    assert_eq!(nodes[0].id_str(), "root");
    assert_eq!(crate::cleanup::count_descendants(&nodes[0]), 2);
}

#[test]
fn strip_reasoning_takes_text_after_last_close_tag() {
    assert_eq!(strip_reasoning("<think>draft</think>REAL").trim(), "REAL");
    assert_eq!(strip_reasoning("no tags here"), "no tags here");
    // 在 think 中被截断(无闭合标签)→ 返回开标签之前(此处空),
    // 不把未完成草稿当输出。
    assert_eq!(strip_reasoning("<think>cut off draft"), "");
    assert_eq!(strip_reasoning("preamble <think>cut off"), "preamble ");
    // 闭合块 + 内容 + 末尾被截断的第二个 think 块 → 只保留中间内容
    // (Codex review:截断的第二个 think 块仍被当真节点)。
    assert_eq!(
        strip_reasoning("<think>plan</think>REAL<think>redo [{\"id\"").trim(),
        "REAL"
    );
}

#[test]
fn parse_nodes_rejects_truncated_think_draft() {
    // <think> 打开但被 max_tokens 截断(无 </think>),里面是未完成的草稿
    // JSON。绝不能把草稿当真节点解析(Codex review:truncated think 仍被
    // 当真节点)——strip 到 <think> 之前(空)→ 解析失败、上层重试。
    let text = r#"<think>
        Let me draft the cards: [
          {"type":"frame","id":"draft","name":"D","width":80,"height":50}
        "#;
    assert!(
        parse_nodes(text).is_err(),
        "truncated think draft must not parse as real nodes"
    );
}

#[test]
fn parse_nodes_ignores_draft_in_truncated_second_think() {
    // 闭合 think#1 + 真答案 + 被截断的 think#2(含草稿)。只解析真答案,
    // 不碰 think#2 草稿(Codex review:截断的第二个 think 块仍被当真节点)。
    let text = r#"<think>planning the layout</think>
        [
          {"type":"frame","id":"root","name":"Root","x":0,"y":0,"width":300,"height":200,"layout":"horizontal","_parent":null},
          {"type":"frame","id":"a","name":"A","width":80,"height":50,"_parent":"root"}
        ]
        <think>wait, let me reconsider and draft more cards: [
          {"type":"frame","id":"draft-x","name":"X","width":99,"height":99}
        "#;
    let nodes = parse_nodes(text).expect("real answer between think blocks must parse");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id_str(), "root");
    // 只有 a;draft-x(在被截断的 think#2 里)不计入。
    assert_eq!(crate::cleanup::count_descendants(&nodes[0]), 1);
}
