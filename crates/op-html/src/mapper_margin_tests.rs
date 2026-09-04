use jian_ops_schema::node::container::Padding;
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use crate::{import_html, HtmlImportOptions, HtmlImportResult};

fn imported(fragment: &str) -> HtmlImportResult {
    import_html(
        &format!("<style>body{{margin:0}}</style>{fragment}"),
        &HtmlImportOptions::default(),
    )
}

fn frame(node: &PenNode) -> &FrameNode {
    let PenNode::Frame(frame) = node else {
        panic!("expected frame, found {node:?}")
    };
    frame
}

fn children(frame: &FrameNode) -> &[PenNode] {
    frame.children.as_deref().unwrap_or_default()
}

fn root(result: &HtmlImportResult) -> &FrameNode {
    frame(&result.nodes[0])
}

fn margin_child(node: &PenNode) -> (&FrameNode, &FrameNode) {
    let margin = frame(node);
    assert_eq!(margin.base.name.as_deref(), Some("Margin"));
    let child = frame(&children(margin)[0]);
    (margin, child)
}

fn edges(padding: Option<&Padding>) -> [f64; 4] {
    match padding {
        Some(Padding::Uniform(value)) => [*value; 4],
        Some(Padding::XY([vertical, horizontal])) => {
            [*vertical, *horizontal, *vertical, *horizontal]
        }
        Some(Padding::LtrB(values)) => *values,
        other => panic!("expected numeric padding, found {other:?}"),
    }
}

fn number(value: Option<&SizingBehavior>) -> f64 {
    let Some(SizingBehavior::Number(value)) = value else {
        panic!("expected numeric size, found {value:?}")
    };
    *value
}

fn named_frame<'a>(node: &'a PenNode, name: &str) -> Option<&'a FrameNode> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    if frame.base.name.as_deref() == Some(name) {
        return Some(frame);
    }
    children(frame)
        .iter()
        .find_map(|child| named_frame(child, name))
}

fn assert_no_private_theme(node: &PenNode) {
    assert!(
        crate::mapper::node_access::node_base(node).theme.is_none(),
        "private carrier leaked from {node:?}"
    );
    if let PenNode::Frame(frame) = node {
        children(frame).iter().for_each(assert_no_private_theme);
    }
}

#[test]
fn body_ua_margin_becomes_the_root_viewport_inset_and_author_zero_wins() {
    let default = import_html("<div>copy</div>", &HtmlImportOptions::default());
    assert_eq!(edges(root(&default).container.padding.as_ref()), [8.0; 4]);

    let reset = import_html(
        "<style>body{margin:0}</style><div>copy</div>",
        &HtmlImportOptions::default(),
    );
    assert_eq!(reset.nodes.len(), 1);
    assert_eq!(root(&reset).container.padding, None);

    let html_only = import_html(
        "<style>html{margin:20px}body{margin:0}</style><div>copy</div>",
        &HtmlImportOptions::default(),
    );
    assert_eq!(
        edges(root(&html_only).container.padding.as_ref()),
        [20.0; 4]
    );

    let html_plus_body = import_html(
        "<style>html{margin:20px}</style><div>copy</div>",
        &HtmlImportOptions::default(),
    );
    assert_eq!(
        edges(root(&html_plus_body).container.padding.as_ref()),
        [28.0; 4]
    );
}

#[test]
fn positive_margins_are_outer_for_visual_and_transparent_boxes() {
    let result = imported(
        "<div style='display:flex'>\
           <div style='width:100px;height:50px;padding:4px;margin:12px 18px;\
                       background:#f00'></div>\
           <div style='width:100px;height:50px;margin:6px 12px'></div>\
         </div>",
    );
    let flex = frame(&children(root(&result))[0]);
    let (visual_margin, visual) = margin_child(&children(flex)[0]);
    assert_eq!(
        edges(visual_margin.container.padding.as_ref()),
        [12.0, 18.0, 12.0, 18.0]
    );
    assert_eq!(number(visual_margin.container.width.as_ref()), 144.0);
    assert_eq!(number(visual_margin.container.height.as_ref()), 82.0);
    assert_eq!(edges(visual.container.padding.as_ref()), [4.0; 4]);
    assert_eq!(number(visual.container.width.as_ref()), 108.0);
    assert_eq!(number(visual.container.height.as_ref()), 58.0);
    assert!(visual.container.fill.is_some());

    let (plain_margin, plain) = margin_child(&children(flex)[1]);
    assert_eq!(
        edges(plain_margin.container.padding.as_ref()),
        [6.0, 12.0, 6.0, 12.0]
    );
    assert_eq!(number(plain_margin.container.width.as_ref()), 124.0);
    assert_eq!(plain.container.padding, None);
    assert_eq!(number(plain.container.width.as_ref()), 100.0);
    assert!(result
        .warnings
        .iter()
        .all(|warning| !warning.contains("visual boxes")));
}

#[test]
fn adjacent_block_margins_collapse_to_max_instead_of_sum() {
    let result = imported(
        "<section style='padding:1px'>\
           <div style='height:10px;margin-bottom:20px'></div>\
           <div style='height:10px;margin-top:12px'></div>\
         </section>",
    );
    let section = frame(&children(root(&result))[0]);
    let (first_margin, _) = margin_child(&children(section)[0]);
    let (second_margin, _) = margin_child(&children(section)[1]);
    assert_eq!(edges(first_margin.container.padding.as_ref())[2], 20.0);
    assert_eq!(second_margin.container.padding, None);
    assert_eq!(number(first_margin.container.height.as_ref()), 30.0);
    assert_eq!(number(second_margin.container.height.as_ref()), 10.0);
}

#[test]
fn leading_and_trailing_child_margins_move_to_the_parent_margin_box() {
    let result = imported(
        "<main style='padding:1px'>\
           <section><div style='height:10px;margin:20px 0 30px'></div></section>\
         </main>",
    );
    let main = frame(&children(root(&result))[0]);
    let (section_margin, section) = margin_child(&children(main)[0]);
    assert_eq!(
        edges(section_margin.container.padding.as_ref()),
        [20.0, 0.0, 30.0, 0.0]
    );
    assert_eq!(section.container.padding, None);
    let (child_margin, child) = margin_child(&children(section)[0]);
    assert_eq!(child_margin.container.padding, None);
    assert_eq!(number(child_margin.container.height.as_ref()), 10.0);
    assert_eq!(number(child.container.height.as_ref()), 10.0);
}

#[test]
fn flex_item_margins_remain_independent_and_do_not_collapse() {
    let result = imported(
        "<div style='display:flex;flex-direction:column'>\
           <div style='height:10px;margin-bottom:20px'></div>\
           <div style='height:10px;margin-top:12px'></div>\
         </div>",
    );
    let flex = frame(&children(root(&result))[0]);
    let (first, _) = margin_child(&children(flex)[0]);
    let (second, _) = margin_child(&children(flex)[1]);
    assert_eq!(edges(first.container.padding.as_ref())[2], 20.0);
    assert_eq!(edges(second.container.padding.as_ref())[0], 12.0);
}

#[test]
fn definite_negative_margin_uses_a_flow_preserving_fixed_wrapper() {
    let result = imported(
        "<div style='display:flex'>\
           <div style='width:100px;height:40px;margin-left:-10px;margin-right:5px;\
                       background:#f00'></div>\
         </div>",
    );
    let flex = frame(&children(root(&result))[0]);
    let (margin, visual) = margin_child(&children(flex)[0]);
    assert_eq!(margin.container.padding, None);
    assert_eq!(number(margin.container.width.as_ref()), 95.0);
    assert_eq!(number(margin.container.height.as_ref()), 40.0);
    assert_eq!(visual.base.x, Some(-10.0));
    assert_eq!(visual.base.y, Some(0.0));
    assert_eq!(number(visual.container.width.as_ref()), 100.0);
    assert!(result
        .warnings
        .iter()
        .all(|warning| !warning.contains("negative CSS margins")));
}

#[test]
fn indefinite_negative_margin_keeps_flow_and_reports_the_fallback() {
    let result = imported("<div style='margin-left:-10px'>copy</div>");
    let child = frame(&children(root(&result))[0]);
    assert_ne!(child.base.name.as_deref(), Some("Margin"));
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("negative CSS margins")));
}

#[test]
fn positioned_leading_margins_adjust_left_and_top_without_a_wrapper() {
    let result = imported(
        "<div style='position:relative;width:200px;height:100px'>\
           <div style='position:absolute;left:10px;top:5px;width:20px;height:10px;\
                       margin-left:8px;margin-top:6px;background:#f00'></div>\
         </div>",
    );
    let parent = frame(&children(root(&result))[0]);
    let absolute = frame(&children(parent)[0]);
    assert_eq!(absolute.base.name.as_deref(), Some("div"));
    assert_eq!(absolute.base.x, Some(18.0));
    assert_eq!(absolute.base.y, Some(11.0));
    assert!(result
        .warnings
        .iter()
        .all(|warning| !warning.contains("visual boxes")));
}

#[test]
fn positioned_trailing_margins_adjust_right_and_bottom_anchors() {
    let result = imported(
        "<div style='position:relative;width:200px;height:100px'>\
           <div style='position:absolute;right:10px;bottom:5px;width:20px;height:10px;\
                       margin-right:8px;margin-bottom:6px'></div>\
         </div>",
    );
    let parent = frame(&children(root(&result))[0]);
    let absolute = frame(&children(parent)[0]);
    assert_eq!(absolute.base.x, Some(162.0));
    assert_eq!(absolute.base.y, Some(79.0));
}

#[test]
fn opposing_positioned_insets_subtract_numeric_margins_from_auto_size() {
    let result = imported(
        "<div style='position:relative;width:200px;height:100px'>\
           <div style='position:absolute;left:10px;right:20px;top:5px;bottom:15px;\
                       margin:6px 12px 4px 8px'></div>\
         </div>",
    );
    let parent = frame(&children(root(&result))[0]);
    let absolute = frame(&children(parent)[0]);
    assert_eq!(absolute.base.x, Some(18.0));
    assert_eq!(absolute.base.y, Some(11.0));
    assert_eq!(number(absolute.container.width.as_ref()), 150.0);
    assert_eq!(number(absolute.container.height.as_ref()), 70.0);
}

#[test]
fn unsupported_positioned_margin_expression_keeps_a_diagnostic() {
    let result = imported(
        "<div style='position:relative;width:200px;height:100px'>\
           <div style='position:absolute;left:10px;right:20px;top:0;height:10px;\
                       margin-left:env(safe-area-inset-left)'></div>\
         </div>",
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("visual boxes")));
}

#[test]
fn opposing_insets_distribute_definite_auto_margins() {
    let result = imported(
        "<div style='position:relative;width:200px;height:100px'>\
           <div style='position:absolute;left:10px;right:20px;width:20px;height:10px;\
                       margin-inline:auto'></div>\
         </div>",
    );
    let parent = frame(&children(root(&result))[0]);
    let centered = frame(&children(parent)[0]);
    assert_eq!(centered.base.x, Some(85.0));
}

#[test]
fn intrinsic_replaced_width_participates_in_positioned_auto_margins() {
    use base64::Engine as _;

    let mut png = vec![0; 24];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[8..12].copy_from_slice(&13u32.to_be_bytes());
    png[12..16].copy_from_slice(b"IHDR");
    png[16..20].copy_from_slice(&100u32.to_be_bytes());
    png[20..24].copy_from_slice(&20u32.to_be_bytes());
    let src = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let result = imported(&format!(
        "<div style='position:relative;width:200px;height:100px'>\
           <img src='{src}' style='position:absolute;left:10px;right:20px;\
                    margin-inline:auto'>\
         </div>"
    ));
    let parent = frame(&children(root(&result))[0]);
    let image = &children(parent)[0];
    assert_eq!(crate::mapper::node_access::node_base(image).x, Some(45.0));
    let PenNode::Image(image) = image else {
        panic!("expected image")
    };
    assert_eq!(image.width, Some(SizingBehavior::Number(100.0)));
}

#[test]
fn negative_auto_margin_space_follows_inline_direction() {
    let result = imported(
        "<div style='position:relative;width:100px;height:40px'>\
           <div style='position:absolute;left:0;right:0;width:150px;height:10px;\
                       margin-inline:auto'></div>\
         </div>\
         <div style='position:relative;width:100px;height:40px'>\
           <div dir='rtl' style='position:absolute;left:0;right:0;\
                       width:150px;height:10px;margin-inline:auto'></div>\
         </div>",
    );
    let ltr_parent = frame(&children(root(&result))[0]);
    let rtl_parent = frame(&children(root(&result))[1]);
    assert_eq!(frame(&children(ltr_parent)[0]).base.x, Some(0.0));
    assert_eq!(frame(&children(rtl_parent)[0]).base.x, Some(-50.0));
}

#[test]
fn rtl_logical_margins_map_to_the_physical_positioning_edges() {
    let result = imported(
        "<div style='position:relative;width:200px;height:40px'>\
           <div style='direction:RTL;position:absolute;left:10px;right:20px;width:20px;height:10px;\
                       margin-inline-start:auto;margin-inline-end:0'></div>\
         </div>\
         <div style='direction:rtl;position:relative;width:200px;height:40px'>\
           <div style='direction:nonsense;position:absolute;left:10px;right:20px;\
                       width:20px;height:10px;margin-inline-start:auto;\
                       margin-inline-end:0'></div>\
         </div>\
         <div style='position:relative;width:200px;height:40px'>\
           <div style='direction:rtl;direction:nonsense;position:absolute;left:10px;\
                       right:20px;width:20px;height:10px;margin-inline-start:auto;\
                       margin-inline-end:0'></div>\
         </div>\
         <div style='direction:rtl;position:relative;width:200px;height:40px'>\
           <div style='--d:nonsense;direction:var(--d);position:absolute;left:10px;\
                       right:20px;width:20px;height:10px;margin-inline-start:auto;\
                       margin-inline-end:0'></div>\
         </div>",
    );
    let parent = frame(&children(root(&result))[0]);
    assert_eq!(frame(&children(parent)[0]).base.x, Some(10.0));
    let inherited_parent = frame(&children(root(&result))[1]);
    assert_eq!(frame(&children(inherited_parent)[0]).base.x, Some(10.0));
    let literal_fallback_parent = frame(&children(root(&result))[2]);
    assert_eq!(
        frame(&children(literal_fallback_parent)[0]).base.x,
        Some(10.0)
    );
    let var_fallback_parent = frame(&children(root(&result))[3]);
    assert_eq!(frame(&children(var_fallback_parent)[0]).base.x, Some(10.0));
}

#[test]
fn deferred_logical_margin_uses_the_resolved_direction() {
    let result = imported(
        "<div style='position:relative;width:200px;height:40px'>\
           <div style='--m:auto 0;position:absolute;left:10px;right:20px;width:20px;\
                       height:10px;margin-inline:var(--m)'></div>\
         </div>\
         <div style='position:relative;width:200px;height:40px'>\
           <div style='--d:RTL;--m:auto 0;direction:var(--d);position:absolute;\
                       left:10px;right:20px;width:20px;height:10px;\
                       margin-inline:var(--m)'></div>\
         </div>",
    );
    let ltr_parent = frame(&children(root(&result))[0]);
    let rtl_parent = frame(&children(root(&result))[1]);
    assert_eq!(frame(&children(ltr_parent)[0]).base.x, Some(160.0));
    assert_eq!(frame(&children(rtl_parent)[0]).base.x, Some(10.0));
}

#[test]
fn logical_margin_revert_operates_on_the_final_physical_property() {
    let reverted = imported(
        "<div style='position:relative;width:200px;height:40px'>\
           <div style='position:absolute;left:10px;width:20px;height:10px;\
                       margin-left:20px;margin-inline-start:revert'></div>\
         </div>",
    );
    let parent = frame(&children(root(&reverted))[0]);
    assert_eq!(frame(&children(parent)[0]).base.x, Some(10.0));

    let layered = imported(
        "<style>\
           @layer base {.x{margin-left:5px}}\
           @layer override {.x{margin-left:20px;margin-inline-start:revert-layer}}\
         </style>\
         <div style='position:relative;width:200px;height:40px'>\
           <div class='x' style='position:absolute;left:10px;width:20px;height:10px'></div>\
         </div>",
    );
    let parent = frame(&children(root(&layered))[0]);
    assert_eq!(frame(&children(parent)[0]).base.x, Some(15.0));
}

#[test]
fn fill_item_margin_wrapper_keeps_fill_sizing_on_both_layers() {
    let result = imported(
        "<div style='display:grid;grid-template-columns:1fr;width:300px'>\
           <div style='width:100%;height:20px;margin:0 10px'></div>\
         </div>",
    );
    let grid = frame(&children(root(&result))[0]);
    let (margin, child) = margin_child(&children(grid)[0]);
    assert!(matches!(
        margin.container.width,
        Some(SizingBehavior::Number(300.0))
            | Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ));
    assert_eq!(
        child.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    );
}

#[test]
fn numeric_margin_and_one_sided_auto_margin_keep_both_outer_behaviors() {
    let result = imported(
        "<div><div style='width:50px;height:20px;margin-left:auto;margin-right:20px'></div></div>",
    );
    let parent = frame(&children(root(&result))[0]);
    let outer = frame(&children(parent)[0]);
    assert_eq!(outer.base.name.as_deref(), Some("Margin"));
    assert_eq!(
        edges(outer.container.padding.as_ref()),
        [0.0, 20.0, 0.0, 0.0]
    );
    assert_eq!(
        outer.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    );
    let alignment = frame(&children(outer)[0]);
    assert_eq!(alignment.base.name.as_deref(), Some("Auto margin"));
    assert_eq!(
        alignment.container.justify_content,
        Some(jian_ops_schema::node::container::JustifyContent::End)
    );
    assert_eq!(
        number(frame(&children(alignment)[0]).container.width.as_ref()),
        50.0
    );
}

#[test]
fn grid_span_and_relative_offset_carriers_reach_the_outer_margin_wrapper() {
    let result = imported(
        "<div style='display:grid;grid-template-columns:repeat(2,1fr);width:400px'>\
           <div style='grid-column:span 2;position:relative;top:4px;\
                       width:100px;height:20px;margin:0 10px'>x</div>\
           <div>y</div>\
         </div>",
    );
    let margin = named_frame(&result.nodes[0], "Margin").expect("margin wrapper");
    assert_eq!(number(margin.container.width.as_ref()), 400.0);
    let offset = frame(&children(margin)[0]);
    assert_eq!(offset.base.name.as_deref(), Some("Offset"));
    assert_eq!(number(offset.container.width.as_ref()), 100.0);
    assert_eq!(frame(&children(offset)[0]).base.y, Some(4.0));
    result.nodes.iter().for_each(assert_no_private_theme);
}

#[test]
fn caption_margin_does_not_break_caption_side_ordering() {
    let result = imported(
        "<table style='width:200px;caption-side:bottom'><caption style='margin:10px'>C</caption>\
           <tr><td>A</td></tr></table>",
    );
    let table = frame(&children(root(&result))[0]);
    let (margin, caption) = margin_child(children(table).last().expect("caption"));
    assert_eq!(caption.base.name.as_deref(), Some("caption"));
    assert_eq!(edges(margin.container.padding.as_ref()), [10.0; 4]);
    assert!(result
        .warnings
        .iter()
        .all(|warning| !warning.contains("visual boxes")));
}

#[test]
fn boxed_inline_margin_stays_in_its_line_without_leaking_a_collapse_hint() {
    let result = imported(
        "<p style='margin:0'>a<span style='display:inline-block;width:20px;height:10px;\
         margin:0 6px;background:#f00'>b</span>c</p>",
    );
    let paragraph = frame(&children(root(&result))[0]);
    let row = frame(&children(paragraph)[0]);
    assert_eq!(row.base.name.as_deref(), Some("inline-row"));
    let inline_margin = named_frame(&children(row)[1], "Margin").expect("inline margin");
    assert_eq!(
        edges(inline_margin.container.padding.as_ref()),
        [0.0, 6.0, 0.0, 6.0]
    );
    result.nodes.iter().for_each(assert_no_private_theme);
}

#[test]
fn block_pseudo_margin_uses_the_same_outer_box_as_an_element() {
    let result = imported(
        "<div style='padding:1px'></div><style>div::before{content:'x';display:block;\
         width:20px;height:10px;margin:12px;background:#f00}</style>",
    );
    let host = frame(&children(root(&result))[0]);
    let (margin, pseudo) = margin_child(&children(host)[0]);
    assert_eq!(edges(margin.container.padding.as_ref()), [12.0; 4]);
    assert_eq!(pseudo.base.name.as_deref(), Some("::before"));
    assert!(pseudo.container.fill.is_some());
    result.nodes.iter().for_each(assert_no_private_theme);
}

#[test]
fn empty_block_pseudo_joins_the_following_margin_collapse_set() {
    let result = imported(
        "<section style='padding:1px'><div style='height:10px;margin-top:40px'></div></section>\
         <style>section::before{content:'';display:block;margin:20px 0 30px}</style>",
    );
    let section = frame(&children(root(&result))[0]);
    let (pseudo_margin, pseudo) = margin_child(&children(section)[0]);
    let (child_margin, _) = margin_child(&children(section)[1]);
    assert_eq!(pseudo.base.name.as_deref(), Some("::before"));
    assert_eq!(pseudo_margin.container.padding, None);
    assert_eq!(edges(child_margin.container.padding.as_ref())[0], 40.0);
}

#[test]
fn empty_block_chain_collapses_all_adjoining_edges_as_one_set() {
    let result = imported(
        "<section style='padding:1px'>\
           <div style='height:10px;margin-bottom:10px'></div>\
           <div style='margin-top:20px;margin-bottom:30px'></div>\
           <div style='height:10px;margin-top:40px'></div>\
         </section>",
    );
    let section = frame(&children(root(&result))[0]);
    let (first, _) = margin_child(&children(section)[0]);
    let (empty, _) = margin_child(&children(section)[1]);
    let (last, _) = margin_child(&children(section)[2]);
    assert_eq!(first.container.padding, None);
    assert_eq!(empty.container.padding, None);
    assert_eq!(edges(last.container.padding.as_ref())[0], 40.0);
    assert_eq!(number(first.container.height.as_ref()), 10.0);
    assert_eq!(number(last.container.height.as_ref()), 50.0);
}

#[test]
fn plain_inline_span_preserves_horizontal_margins_without_a_private_hint() {
    let result = imported("<p style='margin:0'>a<span style='margin-inline:9px'>b</span>c</p>");
    let paragraph = frame(&children(root(&result))[0]);
    let row = frame(&children(paragraph)[0]);
    let (margin, span) = margin_child(&children(row)[1]);
    assert_eq!(
        edges(margin.container.padding.as_ref()),
        [0.0, 9.0, 0.0, 9.0]
    );
    assert_eq!(span.base.name.as_deref(), Some("span"));
    assert_eq!(
        span.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );
    result.nodes.iter().for_each(assert_no_private_theme);
}

#[test]
fn visual_plain_inline_ignores_vertical_margin_in_line_layout() {
    let result = imported(
        "<p style='margin:0'>a<span style='display:inline;margin:20px 6px;\
         background:#f00'>b</span>c</p>",
    );
    let paragraph = frame(&children(root(&result))[0]);
    let row = frame(&children(paragraph)[0]);
    let (margin, span) = margin_child(&children(row)[1]);
    assert_eq!(
        edges(margin.container.padding.as_ref()),
        [0.0, 6.0, 0.0, 6.0]
    );
    assert!(span.container.fill.is_some());
}

#[test]
fn inline_block_prevents_child_margin_from_collapsing_through_its_edge() {
    let result = imported(
        "<p style='margin:0'><span style='display:inline-block'>\
           <b style='display:block;height:10px;margin-top:18px'>x</b>\
         </span></p>",
    );
    let paragraph = frame(&children(root(&result))[0]);
    let row = frame(&children(paragraph)[0]);
    let inline_block = frame(&children(row)[0]);
    assert_eq!(inline_block.base.name.as_deref(), Some("span"));
    let (child_margin, child) = margin_child(&children(inline_block)[0]);
    assert_eq!(edges(child_margin.container.padding.as_ref())[0], 18.0);
    assert_eq!(child.base.name.as_deref(), Some("b"));
}

#[test]
fn inline_pseudo_margin_is_boxed_and_reports_possible_wrap_loss() {
    let result = imported(
        "<p style='width:100px;margin:0'>copy</p>\
         <style>p::before{content:'中文长文本';margin-inline:7px}</style>",
    );
    let paragraph = frame(&children(root(&result))[0]);
    let row = frame(&children(paragraph)[0]);
    let (margin, pseudo) = margin_child(&children(row)[0]);
    assert_eq!(
        edges(margin.container.padding.as_ref()),
        [0.0, 7.0, 0.0, 7.0]
    );
    assert_eq!(pseudo.base.name.as_deref(), Some("::before"));
    assert!(result
        .diagnostics
        .iter()
        .any(|warning| warning.code() == "layout.inline_margin_wrapping_approximated"));
    result.nodes.iter().for_each(assert_no_private_theme);
}

#[test]
fn wrapping_warning_only_fires_when_inline_content_can_wrap() {
    let wrapping = imported(
        "<p style='width:100px;margin:0'><span style='margin-inline:4px'>two words</span></p>",
    );
    assert!(wrapping
        .diagnostics
        .iter()
        .any(|warning| warning.code() == "layout.inline_margin_wrapping_approximated"));

    let one_word =
        imported("<p style='width:100px;margin:0'><span style='margin-inline:4px'>word</span></p>");
    assert!(one_word
        .diagnostics
        .iter()
        .all(|warning| warning.code() != "layout.inline_margin_wrapping_approximated"));

    for content in [
        "中文长文本",
        "soft\u{00ad}hyphen",
        "zero\u{200b}width",
        "a<wbr>b",
    ] {
        let result = imported(&format!(
            "<p style='width:100px;margin:0'><span style='margin-inline:4px'>{content}</span></p>"
        ));
        assert!(
            result
                .diagnostics
                .iter()
                .any(|warning| warning.code() == "layout.inline_margin_wrapping_approximated"),
            "{content:?} should report possible wrap loss"
        );
    }
    for wrapping_rule in [
        "overflow-wrap:anywhere",
        "overflow-wrap:break-word",
        "word-break:break-all",
    ] {
        let result = imported(&format!(
            "<p style='width:100px;margin:0'><span style='margin-inline:4px;{wrapping_rule}'>longtoken</span></p>"
        ));
        assert!(result
            .diagnostics
            .iter()
            .any(|warning| warning.code() == "layout.inline_margin_wrapping_approximated"));
    }
}

#[test]
fn block_table_margin_collapses_with_a_block_sibling() {
    let result = imported(
        "<section style='padding:1px'>\
           <div style='height:10px;margin-bottom:20px'></div>\
           <table style='width:100px;margin-top:30px'><tr><td>x</td></tr></table>\
         </section>",
    );
    let section = frame(&children(root(&result))[0]);
    let (first, _) = margin_child(&children(section)[0]);
    let (table_margin, table) = margin_child(&children(section)[1]);
    assert_eq!(first.container.padding, None);
    assert_eq!(edges(table_margin.container.padding.as_ref())[0], 30.0);
    assert_eq!(table.base.name.as_deref(), Some("table"));
}

#[test]
fn table_margin_applicability_uses_computed_display_not_tag() {
    let internal = imported("<div style='display:table-row;margin:12px;height:10px'></div>");
    assert_eq!(
        frame(&children(root(&internal))[0]).base.name.as_deref(),
        Some("div")
    );

    let restyled = imported(
        "<table style='width:100px'><tr style='display:block;margin:12px'>\
           <td>x</td></tr></table>",
    );
    let table = frame(&children(root(&restyled))[0]);
    let (margin, row) = margin_child(&children(table)[0]);
    assert_eq!(edges(margin.container.padding.as_ref()), [12.0; 4]);
    assert_eq!(row.base.name.as_deref(), Some("tr"));
}

#[test]
fn block_siblings_inside_table_cell_still_collapse() {
    let result = imported(
        "<table style='width:100px'><tr><td>\
           <div style='height:10px;margin-bottom:20px'></div>\
           <div style='height:10px;margin-top:30px'></div>\
         </td></tr></table>",
    );
    let table = frame(&children(root(&result))[0]);
    let row = frame(&children(table)[0]);
    let cell = frame(&children(row)[0]);
    let (first, _) = margin_child(&children(cell)[0]);
    let (second, _) = margin_child(&children(cell)[1]);
    assert_eq!(first.container.padding, None);
    assert_eq!(edges(second.container.padding.as_ref())[0], 30.0);
}

#[test]
fn display_contents_keeps_the_outer_flex_as_the_layout_parent() {
    let result = imported(
        "<div style='display:flex;flex-direction:column'>\
           <span style='display:contents'>\
             <section><div style='height:10px;margin-top:20px'></div></section>\
           </span>\
         </div>",
    );
    let flex = frame(&children(root(&result))[0]);
    let section = frame(&children(flex)[0]);
    assert_eq!(section.base.name.as_deref(), Some("section"));
    let (child_margin, _) = margin_child(&children(section)[0]);
    assert_eq!(edges(child_margin.container.padding.as_ref())[0], 20.0);
}
