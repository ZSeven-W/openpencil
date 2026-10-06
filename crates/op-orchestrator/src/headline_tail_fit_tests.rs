use super::*;
use crate::test_support::VecDocSink;
use serde_json::json;

fn isolated(test: &str) -> bool {
    const CHILD: &str = "OPENPENCIL_HEADLINE_FONT_TEST";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/shared/fonts");
        jian_skia::register_bundled_fonts(
            ["LibreCaslonText-VF.ttf", "NotoSerifSC-VF.ttf"]
                .iter()
                .map(|name| std::fs::read(root.join(name)).unwrap())
                .collect(),
        );
        return false;
    }
    let full = format!("{}::{test}", module_path!())
        .split_once("::")
        .unwrap()
        .1
        .to_string();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &full, "--nocapture"])
        .env(CHILD, test)
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

fn work(content: &str, width: f64, role: &str) -> VecDocSink {
    let node: jian_ops_schema::node::PenNode=serde_json::from_value(json!({
        "type":"frame","id":"card","name":"图文卡片","width":1080,"height":1440,"layout":"vertical","padding":80,
        "children":[{"type":"text","id":"heading","name":"主标题","role":role,"content":content,"width":width,"height":"fit_content","textGrowth":"fixed-width","fontFamily":"Libre Caslon Text, Noto Serif SC","fontSize":120,"fontWeight":700,"letterSpacing":-1,"lineHeight":1.5},
        {"type":"text","id":"body","content":"其他信息保持不变","fontSize":36}]
    })).unwrap();
    let mut sink = VecDocSink::new();
    assert!(sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None
    }));
    sink.applied.clear();
    sink
}

#[test]
fn actual_serif_heading_repair_is_small_complete_and_idempotent() {
    if isolated("actual_serif_heading_repair_is_small_complete_and_idempotent") {
        return;
    }
    let mut sink = work("第一步 · 准备资料", 880.0, "heading");
    let before = sink.state.doc.clone();
    let original = resolved_rects(&sink.state)["heading"];
    assert_eq!(repair_headline_tails(&mut sink, "card"), 1);
    let fixed = resolved_rects(&sink.state)["heading"];
    assert!(fixed.h < original.h * 0.6);
    let Some(EditorCommand::SetNodeFontSize { font_size, .. }) = sink.applied.last() else {
        panic!("one font-size repair")
    };
    assert!(*font_size >= 106.0 && *font_size < 120.0);
    let mut restored = sink.state.doc.clone();
    let text =
        op_editor_core::walkers::find_node_mut(&mut restored.children, &NodeId::new("heading"))
            .unwrap();
    if let jian_ops_schema::node::PenNode::Text(text) = text {
        text.font_size = Some(120.0);
    }
    assert_eq!(restored, before, "no other property or content changes");
    assert_eq!(repair_headline_tails(&mut sink, "card"), 0);
}

#[test]
fn authored_breaks_narrow_compositions_body_and_imports_are_preserved() {
    if isolated("authored_breaks_narrow_compositions_body_and_imports_are_preserved") {
        return;
    }
    for (content, width, role) in [
        ("第一步 · 准备资\n料", 880.0, "heading"),
        ("第一步 · 准备资料", 420.0, "heading"),
        ("第一步 · 准备资料", 880.0, "body-text"),
    ] {
        let mut sink = work(content, width, role);
        let before = sink.state.doc.clone();
        assert_eq!(repair_headline_tails(&mut sink, "card"), 0);
        assert_eq!(sink.state.doc, before);
    }
    for preserve in [true, false] {
        let mut sink = work("第一步 · 准备资料", 880.0, "heading");
        sink.state.editor_ui.preserve_authored_geometry = preserve;
        if !preserve {
            sink.state.editor_ui.home.imported_from = Some("https://example.test/reference".into());
        }
        assert_eq!(repair_headline_tails(&mut sink, "card"), 0);
    }
}

#[test]
fn two_character_tail_uses_the_same_bounded_fit() {
    if isolated("two_character_tail_uses_the_same_bounded_fit") {
        return;
    }
    let content = "整理文字图片时间地点明确读者检查信息保存稿件";
    let mut sink = work(content, 808.0, "heading");
    sink.state.apply(EditorCommand::SetNodeFontSize {
        node_id: NodeId::new("heading"),
        font_size: 40.0,
    });
    sink.applied.clear();
    let original = resolved_rects(&sink.state)["heading"].h;
    assert_eq!(repair_headline_tails(&mut sink, "card"), 1);
    assert!(resolved_rects(&sink.state)["heading"].h < original * 0.6);
}
