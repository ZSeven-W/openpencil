use super::*;
use crate::test_support::VecDocSink;
use op_editor_core::EditorState;
use serde_json::json;

// Exact pd-info elements from the 1018 cross-detail full-pipeline response.
const PRODUCT_INFO: &str =
    "价格数字(动效: 数字滚动), 会员价标签, 商品标题两行, 副标题卖点, 销量/发货地 meta 行";

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"pd-info","label":"价格与商品信息","elements":elements,
        "region":{"width":375,"height":110}
    }))
    .unwrap()
}

fn outcome() -> SubtaskOutcome {
    SubtaskOutcome {
        id: "pd-info".into(),
        node_count: 1,
        inserted_root_ids: vec!["section".into()],
        error: None,
        headline: None,
        subtask: None,
    }
}

#[test]
fn product_info_wrapped_title_does_not_require_two_repeated_items() {
    let mut sink = VecDocSink::new();
    // Preserve the first response's title as one text node. This regression
    // concerns the item promise, not whether the renderer wraps it to two lines.
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[
            {"type":"frame","id":"section","name":"价格与商品信息","children":[
                {"type":"text","id":"price","content":"329.00"},
                {"type":"text","id":"title","content":"北欧原木双人沙发 · 云感羽绒坐垫"},
                {"type":"text","id":"subtitle","content":"白橡木框架 · 可拆洗科技布 · 免安装到户"}
            ]}
        ]}))
        .unwrap(),
    );
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 0);
    assert_eq!(expected_item_count(&task(PRODUCT_INFO)), None);
    assert!(incomplete_attempt(&sink, &task(PRODUCT_INFO), &outcome()).is_none());
}

#[test]
fn title_and_paragraph_line_counts_are_not_item_promises() {
    for elements in [
        "商品标题两行",
        "副标题最多3行",
        "标题显示为两行",
        "正文段落2–3行",
        "描述限制为三行",
        "两行商品标题，三行正文",
        "2–3行段落",
        "product title 2 rows",
        "subtitle wraps to 2 rows",
        "paragraph spans 3 rows",
        "body text across 2–3 rows",
        "2 rows of product title, 3 rows of paragraph text",
        "product title 2 lines; paragraph 3 lines",
        "two-line title and a three-line paragraph",
    ] {
        assert_eq!(expected_item_count(&task(elements)), None, "{elements}");
    }
}

#[test]
fn text_line_context_keeps_real_rows_reviews_and_product_counts() {
    for (elements, count) in [
        ("列表两行，每行商品标题两行", 2),
        ("标题下方3行商品", 3),
        ("两行标题列表", 2),
        ("两行标题卡片", 2),
        ("商品标题两行，评价3条", 3),
        ("段落三行，购物车商品2条", 2),
        ("评价三条，正文最多两行", 3),
        ("商品两条，每条含标题和价格", 2),
        ("title 2 rows; 3 rows of reviews", 3),
        ("list of 3 rows of text", 3),
        ("paragraph spans 3 rows; 2 products", 2),
        ("3 rows below the title", 3),
        ("2 rows of title entries", 2),
        ("2 rows of title cards", 2),
    ] {
        assert_eq!(
            expected_item_count(&task(elements)),
            Some(count),
            "{elements}"
        );
    }
    // A genuine list still fails delivery when the artifact is short, even
    // though the same local spec also asks for a wrapped title on every row.
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[
            {"type":"frame","id":"section","layout":"vertical","children":[
                {"type":"frame","id":"item-a","name":"Review","children":[
                    {"type":"text","id":"label-a","content":"Review A"}
                ]},
                {"type":"frame","id":"item-b","name":"Review","children":[
                    {"type":"text","id":"label-b","content":"Review B"}
                ]}
            ]}
        ]}))
        .unwrap(),
    );
    let failure = incomplete_attempt(&sink, &task("评价3条，标题两行"), &outcome()).unwrap();
    assert_eq!((failure.expected, failure.delivered), (3, 2));
}
