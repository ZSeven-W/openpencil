use super::*;
use serde_json::json;
use std::collections::HashMap;

pub(super) fn rects(entries: &[(&str, f64, f64, f64, f64)]) -> HashMap<String, Rect> {
    entries
        .iter()
        .map(|(id, x, y, w, h)| {
            (
                (*id).to_string(),
                Rect {
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                },
            )
        })
        .collect()
}

#[test]
fn pill_chip_text_overflow_is_not_converted_to_fill_wrap() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Guest Chip","width":48,"height":40,
        "layout":"horizontal","cornerRadius":9999,"children":[
            {"type":"text","id":"guest-text","name":"Guest Text","content":"2 Guests, 1 Room","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 48.0, 40.0),
        ("guest-text", 0.0, 0.0, 140.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                    if property == "width" && k == "fill_container"
            )
        }),
        "pill chip text must stay single-line instead of fill+wrap: {cmds:?}"
    );
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                    if property == "textGrowth" && k == "fixed-width"
            )
        }),
        "pill chip text must not wrap: {cmds:?}"
    );
    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Bool(true),
                } if node_id.as_str() == "chip" && property == "clipContent"
            )
        }),
        "numeric-width pill with measured text overflow must set clipContent: {cmds:?}"
    );
}

#[test]
fn pill_chip_text_already_fill_wrapped_is_restored_to_single_line() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Guest Chip","width":60,"height":40,
        "layout":"horizontal","cornerRadius":9999,"children":[
            {"type":"text","id":"guest-text","name":"Guest Text","content":"2 Guests, 1 Room","width":"fill_container","textGrowth":"fixed-width"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 60.0, 40.0),
        ("guest-text", 0.0, 0.0, 32.0, 65.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "width" && k == "fit_content"
        )
    }));
    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "textGrowth" && k == "auto"
        )
    }));
}

fn compact_status_header(
    badge_width: serde_json::Value,
    label_width: serde_json::Value,
) -> serde_json::Value {
    json!({
        "type":"frame","id":"header","width":129,"layout":"horizontal",
        "justifyContent":"space_between","alignItems":"center","children":[
            {
                "type":"frame","id":"title","layout":"horizontal","gap":6,
                "alignItems":"center","children":[
                    {
                        "type":"icon_font","id":"icon","iconFontName":"wind",
                        "width":14,"height":14
                    },
                    {"type":"text","id":"title-text","content":"AIR QUALITY","width":"fit_content"}
                ]
            },
            {
                "type":"frame","id":"badge","width":badge_width,
                "layout":"horizontal","gap":4,"padding":[3,8],
                "alignItems":"center",
                "fill":[{"type":"solid","color":"#C4F82A20"}],
                "children":[
                    {"type":"ellipse","id":"dot","width":6,"height":6},
                    {
                        "type":"text","id":"label","content":"Good",
                        "width":label_width,"textGrowth":"fixed-width"
                    }
                ]
            }
        ]
    })
}

#[test]
fn overfull_title_and_status_badge_stacks_without_stretching_badge() {
    let row = compact_status_header(json!("fit_content"), json!("fit_content"));
    let rects = rects(&[
        ("header", 0.0, 0.0, 129.0, 22.0),
        ("title", 0.0, 4.0, 96.0, 14.0),
        ("icon", 0.0, 4.0, 14.0, 14.0),
        ("title-text", 20.0, 4.0, 76.0, 14.0),
        ("badge", 96.0, 0.0, 51.0, 22.0),
        ("dot", 104.0, 8.0, 6.0, 6.0),
        ("label", 114.0, 4.0, 25.0, 14.0),
    ]);

    let mut frame_cmds = Vec::new();
    collect_frame_overflow_fixes(&row, &rects, &mut frame_cmds);
    assert!(
        frame_cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Keyword(value),
                } if node_id.as_str() == "badge"
                    && property == "width"
                    && value == "fill_container"
            )
        }),
        "past-right status badge must stay Hug: {frame_cmds:?}"
    );

    let mut gap_cmds = Vec::new();
    collect_row_gap_fixes(&row, &rects, &mut gap_cmds);
    assert!(gap_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Number(8.0),
            } if node_id.as_str() == "header" && property == "gap"
        )
    }));

    let mut row_cmds = Vec::new();
    collect_row_overfull_fixes(&row, &rects, &mut row_cmds, false);
    assert!(row_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "header"
                && property == "layout"
                && value == "vertical"
        )
    }));
    assert!(row_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "header"
                && property == "height"
                && value == "fit_content"
        )
    }));
    for id in ["title", "badge"] {
        assert!(row_cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Keyword(value),
                } if node_id.as_str() == id
                    && property == "width"
                    && value == "fit_content"
            )
        }));
    }
    assert!(
        row_cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Keyword(value),
                } if node_id.as_str() == "badge"
                    && property == "width"
                    && value == "fill_container"
            )
        }),
        "badge is never stretched: {row_cmds:?}"
    );
}

#[test]
fn damaged_status_badge_returns_to_hug_text_and_small_gap() {
    let mut row = compact_status_header(json!("fill_container"), json!("fill_container"));
    row.as_object_mut().unwrap().insert("gap".into(), json!(16));
    let rects = rects(&[
        ("header", 0.0, 0.0, 129.0, 46.0),
        ("title", 0.0, 16.0, 96.0, 14.0),
        ("icon", 0.0, 16.0, 14.0, 14.0),
        ("title-text", 20.0, 16.0, 76.0, 14.0),
        ("badge", 112.0, 0.0, 17.0, 46.0),
        ("dot", 120.0, 20.0, 6.0, 6.0),
        ("label", 130.0, 3.0, 1.0, 40.0),
    ]);

    let mut text_cmds = Vec::new();
    collect_text_overflow_fixes(&row, &rects, &mut text_cmds);
    assert!(text_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "label"
                && property == "width"
                && value == "fit_content"
        )
    }));
    assert!(text_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "label"
                && property == "textGrowth"
                && value == "auto"
        )
    }));

    let mut gap_cmds = Vec::new();
    collect_row_gap_fixes(&row, &rects, &mut gap_cmds);
    assert!(gap_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Number(8.0),
            } if node_id.as_str() == "header" && property == "gap"
        )
    }));

    let mut row_cmds = Vec::new();
    collect_row_overfull_fixes(&row, &rects, &mut row_cmds, false);
    assert!(row_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "header"
                && property == "layout"
                && value == "vertical"
        )
    }));
    assert!(row_cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "badge"
                && property == "width"
                && value == "fit_content"
        )
    }));
}

#[test]
fn numeric_six_status_label_returns_to_hug_width() {
    let row = compact_status_header(json!("fit_content"), json!(6));
    let rects = rects(&[
        ("header", 0.0, 0.0, 129.0, 140.0),
        ("title", 0.0, 63.0, 96.0, 14.0),
        ("icon", 0.0, 63.0, 14.0, 14.0),
        ("title-text", 20.0, 63.0, 76.0, 14.0),
        ("badge", 102.0, 0.0, 27.0, 140.0),
        ("dot", 110.0, 67.0, 6.0, 6.0),
        ("label", 120.0, 3.0, 6.0, 134.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&row, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(value),
            } if node_id.as_str() == "label"
                && property == "width"
                && value == "fit_content"
        )
    }));
}

#[test]
fn damaged_compact_status_header_converges_with_readable_geometry() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;

    let mut header = compact_status_header(json!("fill_container"), json!(6));
    let header_obj = header.as_object_mut().unwrap();
    header_obj.insert("height".into(), json!(22));
    header_obj.insert("gap".into(), json!(16));
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","width":129,"height":200,
        "layout":"vertical","children":[header]
    }))
    .expect("valid status-header fixture");
    let mut sink = VecDocSink::new();
    assert!(sink.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    }));

    assert!(geometry_validate_and_fix(&mut sink, "root") > 0);
    let settled = serde_json::to_value(&sink.state().active_children()[0]).unwrap();
    assert_eq!(geometry_validate_and_fix(&mut sink, "root"), 0);
    assert_eq!(
        serde_json::to_value(&sink.state().active_children()[0]).unwrap(),
        settled,
        "a second pass must be byte-structurally idle"
    );

    let rects = resolved_rects(sink.state());
    let row = rects.get("header").unwrap();
    let title = rects.get("title").unwrap();
    let badge = rects.get("badge").unwrap();
    let label = rects.get("label").unwrap();
    assert!(row.h >= 38.0, "fixed 22px header must grow for reflow");
    assert!(
        badge.y >= title.y + title.h + 7.0,
        "title={:?} badge={:?} row={:?}",
        (title.x, title.y, title.w, title.h),
        (badge.x, badge.y, badge.w, badge.h),
        (row.x, row.y, row.w, row.h)
    );
    assert!(badge.x + badge.w <= row.x + row.w + TEXT_OVERFLOW_EPS);
    assert!(label.w >= 20.0, "short status label must remain readable");
    assert_eq!(
        value_by_id(&settled, "header").unwrap()["layout"],
        "vertical"
    );
    assert_eq!(
        value_by_id(&settled, "header").unwrap()["height"],
        "fit_content"
    );
}

#[test]
fn ordinary_card_text_overflow_still_converts_to_fill_wrap() {
    let card = json!({
        "type":"frame","id":"card","name":"Narrow Card","width":48,"height":90,
        "layout":"vertical","cornerRadius":8,"children":[
            {"type":"text","id":"copy","name":"Copy","content":"Long text","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("card", 0.0, 0.0, 48.0, 90.0),
        ("copy", 0.0, 0.0, 140.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&card, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "width" && k == "fill_container"
        )
    }));
    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "textGrowth" && k == "fixed-width"
        )
    }));
}

#[test]
fn overfull_all_pill_chip_row_clips_instead_of_flexifying_chips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":240,"height":48,"layout":"horizontal","gap":8,"children":[
            {"type":"frame","id":"date","name":"Date Chip","width":120,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":100,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"map","name":"Map Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"mt","content":"Map"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 240.0, 48.0),
        ("date", 0.0, 0.0, 120.0, 40.0),
        ("guest", 128.0, 0.0, 100.0, 40.0),
        ("map", 236.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                if node_id.as_str() == "chips" && property == "clipContent"
        )
    }));
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                    if property == "width" && k == "fill_container"
            )
        }),
        "pill chips must not be flexified: {cmds:?}"
    );
}

#[test]
fn overfull_pill_chip_row_with_spacer_still_clips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":240,"height":"fit_content","layout":"horizontal","gap":10,"children":[
            {"type":"frame","id":"date","name":"Date Chip","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":120,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"spacer","name":"Spacer","role":"spacer","width":"fill_container","height":1,"children":[]},
            {"type":"frame","id":"sort","name":"Sort Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"st","content":"Sort"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 240.0, 48.0),
        ("date", 0.0, 0.0, 86.0, 40.0),
        ("guest", 96.0, 0.0, 120.0, 40.0),
        ("spacer", 226.0, 0.0, 1.0, 1.0),
        ("sort", 237.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                    if node_id.as_str() == "chips" && property == "clipContent"
            )
        }),
        "spacer should not prevent chip-row clipping: {cmds:?}"
    );
}

#[test]
fn fitted_pill_chip_rail_with_flexible_spacer_still_clips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":"fill_container","height":"fit_content","layout":"horizontal","gap":10,"children":[
            {"type":"frame","id":"date","name":"Date Chip","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":"fill_container","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"spacer","name":"Spacer","role":"spacer","width":"fill_container","height":1,"children":[]},
            {"type":"frame","id":"sort","name":"Sort Chip","width":"fill_container","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"st","content":"Sort"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 335.0, 40.0),
        ("date", 0.0, 0.0, 149.0, 40.0),
        ("guest", 159.0, 0.0, 60.0, 40.0),
        ("spacer", 229.0, 20.0, 36.0, 1.0),
        ("sort", 275.0, 0.0, 60.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                    if node_id.as_str() == "chips" && property == "clipContent"
            )
        }),
        "all-pill chip rails clip their overflow instead of flexifying chips: {cmds:?}"
    );
}

#[test]
fn mixed_overfull_row_does_not_get_chip_clip_exemption() {
    let row = json!({
        "type":"frame","id":"row","name":"Mixed Row","width":240,"height":48,"layout":"horizontal","gap":8,"children":[
            {"type":"frame","id":"filter","name":"Filter Panel","width":140,"height":40,"cornerRadius":8,"children":[{"type":"text","id":"ft","content":"Filter"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":100,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"map","name":"Map Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"mt","content":"Map"}]}
        ]
    });
    let rects = rects(&[
        ("row", 0.0, 0.0, 240.0, 48.0),
        ("filter", 0.0, 0.0, 140.0, 40.0),
        ("guest", 148.0, 0.0, 100.0, 40.0),
        ("map", 256.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Bool(true), .. }
                    if property == "clipContent"
            )
        }),
        "mixed row must not get pill-row clipping: {cmds:?}"
    );
}

fn category_pill(id: &str, label: &str) -> serde_json::Value {
    json!({
        "type":"frame",
        "id":id,
        "name":format!("Category Pill {label}"),
        "width":"fit_content",
        "height":"fit_content",
        "layout":"horizontal",
        "gap":8,
        "padding":[8,16],
        "cornerRadius":8,
        "children":[
            {
                "type":"rectangle",
                "id":format!("{id}-icon"),
                "name":format!("{label} Icon"),
                "width":16,
                "height":16
            },
            {
                "type":"text",
                "id":format!("{id}-label"),
                "name":format!("{label} Label"),
                "content":label,
                "width":"fit_content",
                "height":"fit_content",
                "textGrowth":"auto",
                "fontSize":14
            }
        ]
    })
}

fn value_by_id<'a>(value: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    if value.get("id").and_then(serde_json::Value::as_str) == Some(id) {
        return Some(value);
    }
    value
        .get("children")
        .and_then(serde_json::Value::as_array)
        .and_then(|children| children.iter().find_map(|child| value_by_id(child, id)))
}

#[test]
fn clipped_horizontal_category_scroller_preserves_hug_pills_across_geometry_passes() {
    use crate::test_support::VecDocSink;
    use crate::types::DocSink;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::PenNodeExt;

    let pills = vec![
        category_pill("stays", "Stays"),
        category_pill("experiences", "Experiences"),
        category_pill("dining", "Dining"),
        category_pill("nature", "Nature"),
        category_pill("hidden-gems", "Hidden Gems"),
    ];
    let root: PenNode = serde_json::from_value(json!({
        "type":"frame",
        "id":"root",
        "name":"Explore",
        "width":375,
        "height":844,
        "layout":"vertical",
        "children":[{
            "type":"frame",
            "id":"viewport",
            "name":"Category Rail Scroll Wrapper",
            "width":"fill_container",
            "height":"fit_content",
            "layout":"horizontal",
            "clipContent":true,
            "children":[{
                "type":"frame",
                "id":"rail",
                "name":"Category Rail Inner",
                "width":"fill_container",
                "height":"fit_content",
                "layout":"horizontal",
                "gap":12,
                "padding":[4,24],
                "justifyContent":"space_between",
                "alignItems":"center",
                "clipContent":false,
                "children":pills
            }]
        }]
    }))
    .expect("valid category scroller fixture");

    let mut sink = VecDocSink::new();
    assert!(sink.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    }));

    // Prove this is a real regression fixture, not merely a scroller-shaped
    // tree: the resolved tail pill lies past the fill-width inner rail and the
    // five rigid pills are collectively overfull. Without inherited scroller
    // context, frame-overflow and row-overfull both have evidence to mutate it.
    let before = resolved_rects(sink.state());
    let rail_rect = before.get("rail").expect("rail resolves");
    let tail_rect = before.get("hidden-gems").expect("tail pill resolves");
    assert!(tail_rect.x + tail_rect.w > rail_rect.x + rail_rect.w + TEXT_OVERFLOW_EPS);
    let pill_widths: f64 = ["stays", "experiences", "dining", "nature", "hidden-gems"]
        .iter()
        .map(|id| before.get(*id).expect("pill resolves").w)
        .sum();
    assert!(pill_widths + 4.0 * 12.0 > rail_rect.w - 48.0 + ROW_OVERFULL_EPS);

    for pass in 1..=3 {
        assert_eq!(
            geometry_validate_and_fix(&mut sink, "root"),
            0,
            "intentional scroller must remain untouched on pass {pass}"
        );
    }

    let root = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root remains present");
    let value = serde_json::to_value(root).expect("root serializes");
    assert_eq!(
        value_by_id(&value, "rail").unwrap()["width"],
        "fill_container"
    );
    assert_eq!(
        value_by_id(&value, "rail").unwrap()["justifyContent"],
        "space_between"
    );
    for id in ["stays", "experiences", "dining", "nature", "hidden-gems"] {
        assert_eq!(
            value_by_id(&value, id).unwrap()["width"],
            "fit_content",
            "{id} must not be flexified"
        );
        let label = value_by_id(&value, &format!("{id}-label")).unwrap();
        assert_eq!(
            label["width"], "fit_content",
            "{id} label must keep hug width"
        );
        assert_eq!(
            label["textGrowth"], "auto",
            "{id} label must stay single-line"
        );
    }
}

#[test]
fn ordinary_vertical_clip_does_not_suppress_text_overflow_repair() {
    let tree = json!({
        "type":"frame","id":"clip","width":100,"height":100,
        "layout":"vertical","clipContent":true,"children":[{
            "type":"frame","id":"card","width":48,"height":90,"layout":"vertical",
            "children":[{
                "type":"text","id":"copy","content":"Long text",
                "width":"fit_content","textGrowth":"auto"
            }]
        }]
    });
    let rects = rects(&[
        ("clip", 0.0, 0.0, 100.0, 100.0),
        ("card", 0.0, 0.0, 48.0, 90.0),
        ("copy", 0.0, 0.0, 140.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&tree, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Keyword(k) }
            if node_id.as_str() == "copy" && property == "width" && k == "fill_container"
    )));
}
