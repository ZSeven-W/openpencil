use super::spacing::detect_edge_section_padding;
use crate::issue::{FixProperty, IssueCategory, IssueSeverity};
use jian_ops_schema::node::PenNode;
use serde_json::json;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("fixture must deserialize as PenNode")
}

#[test]
fn flags_each_unpadded_content_section_with_default_rail() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "categories", "role": "categories",
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "list", "role": "list",
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].node_id, "categories");
    assert_eq!(issues[1].node_id, "list");
    for issue in issues {
        assert_eq!(issue.category, IssueCategory::EdgeSectionPadding);
        assert_eq!(issue.property, FixProperty::Padding);
        assert_eq!(issue.severity, IssueSeverity::Warning);
        assert_eq!(issue.current_value, json!(null));
        assert_eq!(issue.suggested_value, json!([0, 24, 0, 24]));
    }
}

#[test]
fn padded_sibling_infers_rail_without_suppressing_unpadded_section() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "unpadded", "role": "categories",
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "established", "role": "list",
                "padding": [0, 20, 0, 20],
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "unpadded");
    assert_eq!(issues[0].suggested_value, json!([0, 20, 0, 20]));
}

#[test]
fn rail_inference_prefers_modal_value_then_value_nearest_policy_default() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "unpadded",
                "children": [{"type": "text", "id": "t1", "content": "Title"}]
            },
            {
                "type": "frame", "id": "rail-a", "padding": [0, 22, 0, 22],
                "children": [{"type": "text", "id": "t2", "content": "A"}]
            },
            {
                "type": "frame", "id": "rail-b", "padding": [0, 22, 0, 22],
                "children": [{"type": "text", "id": "t3", "content": "B"}]
            },
            {
                "type": "frame", "id": "rail-c", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "t4", "content": "C"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].suggested_value, json!([0, 22, 0, 22]));
}

#[test]
fn preserves_section_vertical_padding() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "section", "padding": [12, 0, 8, 0],
                "children": [{"type": "text", "id": "t1", "content": "Categories"}]
            },
            {
                "type": "frame", "id": "established", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "t2", "content": "Item"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "section");
    assert_eq!(issues[0].current_value, json!([12, 0, 8, 0]));
    assert_eq!(issues[0].suggested_value, json!([12, 24, 8, 24]));
}

#[test]
fn skips_full_bleed_chrome_and_image_only_sections() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "status", "role": "status-bar",
                "children": [{"type": "text", "id": "time", "content": "9:41"}]
            },
            {
                "type": "frame", "id": "hero", "role": "hero",
                "children": [{"type": "text", "id": "welcome", "content": "Welcome"}]
            },
            {
                "type": "frame", "id": "media", "role": "gallery",
                "children": [{"type": "image", "id": "img", "src": "a.png"}]
            },
            {
                "type": "frame", "id": "nav", "role": "bottom-tab-bar",
                "children": [{"type": "icon_font", "id": "home", "iconFontName": "home"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn skips_surfaced_root_direct_card_without_a_safe_outer_owner() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "card",
                "fill": [{"type":"solid","color":"#111111"}],
                "stroke": {"thickness":1,"fill":[{"type":"solid","color":"#333333"}]},
                "children": [{"type": "text", "id": "card-title", "content": "Hero"}]
            },
            {
                "type": "frame", "id": "established", "padding": [0,24],
                "children": [{"type": "text", "id": "body", "content": "Body"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn skips_non_mobile_root_and_non_content_section() {
    let non_mobile = node(json!({
        "type": "frame", "id": "card",
        "width": 393, "height": 200, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "s1",
                "children": [{"type": "text", "id": "t1", "content": "Title"}]
            },
            {
                "type": "frame", "id": "s2",
                "children": [{"type": "text", "id": "t2", "content": "Body"}]
            }
        ]
    }));
    assert!(detect_edge_section_padding(&non_mobile).is_empty());

    let no_content = node(json!({
        "type": "frame", "id": "root",
        "width": 393, "height": 852, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "s1",
                "children": [{"type": "rectangle", "id": "r1"}]
            },
            {
                "type": "frame", "id": "s2",
                "children": [{"type": "rectangle", "id": "r2"}]
            }
        ]
    }));
    assert!(detect_edge_section_padding(&no_content).is_empty());
}

#[test]
fn scroller_section_targets_transparent_header_and_viewport_leading_edge() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "hourly", "layout": "vertical",
                "children": [
                    {
                        "type": "frame", "id": "hourly-header", "role": "navbar",
                        "layout": "horizontal",
                        "children": [{"type": "text", "id": "title", "content": "Hourly"}]
                    },
                    {
                        "type": "frame", "id": "hourly-scroll",
                        "layout": "horizontal", "clipContent": true,
                        "children": [{
                            "type": "frame", "id": "hour-card",
                            "children": [{"type": "text", "id": "hour", "content": "Now"}]
                        }]
                    }
                ]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);

    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].node_id, "hourly-header");
    assert_eq!(issues[0].suggested_value, json!([0, 24, 0, 24]));
    assert!(issues[0]
        .reason
        .contains("without changing clipped scroll geometry"));
    assert_eq!(issues[1].node_id, "hourly-scroll");
    assert_eq!(issues[1].suggested_value, json!([0, 0, 0, 24]));
    assert!(issues[1].reason.contains("trailing edge flush"));
}

#[test]
fn scroller_section_skips_filled_header_but_repairs_viewport_leading_edge() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "hourly", "layout": "vertical",
                "children": [
                    {
                        "type": "frame", "id": "filled-header",
                        "fill": [{"type": "solid", "color": "#111111"}],
                        "children": [{"type": "text", "id": "title", "content": "Hourly"}]
                    },
                    {
                        "type": "frame", "id": "hourly-scroll",
                        "layout": "horizontal", "clipContent": true,
                        "children": [{"type": "text", "id": "hour", "content": "Now"}]
                    }
                ]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "hourly-scroll");
    assert_eq!(issues[0].suggested_value, json!([0, 0, 0, 24]));
}

#[test]
fn ignores_padding_expression_instead_of_clobbering_design_token() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "token-section", "padding": "$space-page",
                "children": [{"type": "text", "id": "title", "content": "Title"}]
            },
            {
                "type": "frame", "id": "forecast", "padding": [0, 24, 0, 24],
                "children": [{"type": "text", "id": "forecast-title", "content": "7-Day"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn root_direct_scroller_gets_leading_only_rail() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "viewport",
                "layout": "horizontal", "clipContent": true,
                "children": [
                    {"type":"image","id":"image-a","width":120,"height":90,"src":"a.png"},
                    {"type":"image","id":"image-b","width":120,"height":90,"src":"b.png"}
                ]
            },
            {
                "type": "frame", "id": "body", "padding": [0,24],
                "children": [{"type": "text", "id": "body-title", "content": "Body"}]
            }
        ]
    }));

    let issues = detect_edge_section_padding(&root);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "viewport");
    assert_eq!(issues[0].suggested_value, json!([0, 0, 0, 24]));
}

#[test]
fn fit_content_mobile_screen_is_checked() {
    let root = node(json!({
        "type": "frame", "id": "root",
        "width": 390, "height": "fit_content", "layout": "vertical",
        "children": [
            {"type":"frame","id":"status","role":"status-bar","height":44},
            {
                "type":"frame","id":"unpadded",
                "children":[{"type":"text","id":"title","content":"Title"}]
            },
            {
                "type":"frame","id":"established","padding":[0,24],
                "children":[{"type":"text","id":"body","content":"Body"}]
            },
            {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
        ]
    }));

    let issues = detect_edge_section_padding(&root);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].node_id, "unpadded");
    assert_eq!(issues[0].suggested_value, json!([0, 24, 0, 24]));
}

#[test]
fn transparent_full_bleed_media_overlay_is_not_inset() {
    let root = node(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {
                "type":"frame","id":"media-overlay","width":"fill_container","layout":"none",
                "children":[
                    {"type":"image","id":"cover","width":"fill_container","height":240,"src":"cover.png"},
                    {"type":"text","id":"overlay","content":"Featured story"}
                ]
            },
            {
                "type":"frame","id":"body","padding":[0,24],
                "children":[{"type":"text","id":"body-title","content":"Body"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn coloured_full_bleed_media_overlay_is_not_inset() {
    let root = node(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {
                "type":"frame","id":"map-section","padding":[0,0],
                "children":[
                    {
                        "type":"frame","id":"map-placeholder",
                        "width":"fill_container",
                        "fill":[{"type":"linear_gradient","stops":[]}]
                    },
                    {
                        "type":"frame","id":"map-inset","padding":[0,24],
                        "children":[{"type":"text","id":"map-title","content":"Map"}]
                    }
                ]
            },
            {
                "type":"frame","id":"body","padding":[0,24],
                "children":[{"type":"text","id":"body-title","content":"Body"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn nested_fit_content_component_without_mobile_chrome_is_not_a_screen() {
    let root = node(json!({
        "type":"frame","id":"desktop-root","width":1200,"height":900,"layout":"vertical",
        "children":[{
            "type":"frame","id":"component","width":375,"height":"fit_content","layout":"vertical",
            "children":[
                {"type":"frame","id":"row-a","children":[{"type":"text","id":"a","content":"A"}]},
                {"type":"frame","id":"row-b","children":[{"type":"text","id":"b","content":"B"}]},
                {"type":"frame","id":"row-c","children":[{"type":"text","id":"c","content":"C"}]},
                {"type":"frame","id":"row-d","padding":[0,24],"children":[{"type":"text","id":"d","content":"D"}]}
            ]
        }]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
}

#[test]
fn a_section_the_hero_bleed_pass_marked_is_not_inset_even_with_a_labelled_map_canvas() {
    // GLM shape from wave-glmflash-0908/B/app-07: the bleed section holds a
    // bordered map canvas whose direct children include text labels, so
    // none of the media predicates match — only the pass's own marker does.
    let root = node(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {
                "type":"frame","id":"map-section","name":"地图占位区 (bleed)","padding":[0,0],
                "children":[{
                    "type":"frame","id":"map-canvas","width":"fill_container","height":260,
                    "layout":"none","fill":[{"type":"solid","color":"#e5e7eb"}],
                    "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#d1d5db"}]},
                    "children":[
                        {"type":"rectangle","id":"road","width":"fill_container","height":6},
                        {"type":"text","id":"label","content":"科技南路","x":120,"y":40}
                    ]
                }]
            },
            {
                "type":"frame","id":"body","padding":[0,24],
                "children":[{"type":"text","id":"body-title","content":"Body"}]
            },
            {
                "type":"frame","id":"body-2","padding":[0,24],
                "children":[{"type":"text","id":"body-title-2","content":"More"}]
            }
        ]
    }));

    assert!(detect_edge_section_padding(&root).is_empty());
    assert!(super::siblings::detect_mixed_sibling_padding(&root).is_empty());
}
