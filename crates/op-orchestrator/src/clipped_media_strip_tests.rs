use super::{full_width_media_section, media_lane};
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

fn viewport_with_pages(count: usize, gap: f64) -> Value {
    let images: Vec<_> = (0..count)
        .map(|index| {
            json!({"type":"image","id":format!("page-{index}"),
                "width":375,"height":340,"src":""})
        })
        .collect();
    json!({"type":"frame","id":"viewport","layout":"none",
    "width":375,"height":340,"clipContent":true,"children":[{
        "type":"frame","id":"lane","layout":"horizontal",
        "x":0,"y":0,"width":375.0 * count as f64 + gap * count.saturating_sub(1) as f64,
        "height":340,"clipContent":false,"gap":gap,"children":images
    }]})
}

fn viewport() -> Value {
    viewport_with_pages(5, 0.0)
}

fn assert_rejected(value: &Value, reason: &str) {
    assert!(media_lane(value, 375.0).is_none(), "{reason}: {value}");
}

fn section(value: Value) -> Value {
    json!({"type":"frame","id":"section","layout":"vertical",
        "width":"fill_container","height":"fit_content","children":[value]})
}

fn is_full_width(value: Value, width: f64) -> bool {
    let node: PenNode = serde_json::from_value(value).unwrap();
    full_width_media_section(&node, width)
}

fn decorative_overlay(decoration: Value) -> Value {
    json!({"type":"frame","id":"overlay","x":0,"y":0,
        "width":375,"height":44,"layout":"horizontal","children":[decoration]})
}

fn icon() -> Value {
    json!({"type":"icon_font","id":"overlay-icon","iconFontFamily":"lucide",
        "iconFontName":"chevron-left","width":20,"height":20})
}

fn single_image_viewport() -> Value {
    let mut value = viewport();
    value["children"] = json!([{"type":"image","id":"single-image",
        "x":0,"y":0,"width":375,"height":340,"src":""}]);
    value
}

fn assert_value_and_typed_rejected(value: Value, reason: &str) {
    assert_rejected(&value, reason);
    let node: PenNode = serde_json::from_value(value).unwrap();
    assert!(!full_width_media_section(&node, 375.0), "typed: {reason}");
    assert_rejected(&serde_json::to_value(&node).unwrap(), reason);
}

#[test]
fn unnamed_fixed_media_pages_preserve_the_authored_lane() {
    for count in [2, 5, 7] {
        let value = viewport_with_pages(count, 0.0);
        let before = value.clone();
        let lane = media_lane(&value, 375.0).expect("full-page media strip");
        assert_eq!(lane["id"], "lane");
        assert_eq!(lane["children"].as_array().unwrap().len(), count);
        assert_eq!(lane, &value["children"][0]);
        assert_eq!(value, before, "recognition must not normalize the source");
    }
}

#[test]
fn full_width_and_hug_height_viewport_use_the_supplied_width() {
    let mut value = viewport();
    value["width"] = json!("fill_container");
    value["height"] = json!("fit_content");
    assert!(media_lane(&value, 375.0).is_some());
    for width in [327.0, 400.0, 0.0, -375.0, f64::NAN, f64::INFINITY] {
        assert!(
            media_lane(&value, width).is_none(),
            "page width must agree with finite positive viewport width: {width}"
        );
    }
}

#[test]
fn explicit_static_defaults_and_source_urls_do_not_change_recognition() {
    let mut value = viewport();
    for path in ["", "/children/0", "/children/0/children/0"] {
        let node = value.pointer_mut(path).unwrap();
        node["visible"] = json!(true);
        node["enabled"] = json!(true);
        node["opacity"] = json!(1.0);
    }
    value["padding"] = json!([0, 0, 0, 0]);
    value["children"][0]["padding"] = json!([0, 0]);
    value["children"][0]["children"][0]["src"] = json!("assets/product.webp");
    assert!(media_lane(&value, 375.0).is_some());
}

#[test]
fn nonzero_gap_requires_exact_inter_page_space() {
    let value = viewport_with_pages(5, 12.0);
    assert!(media_lane(&value, 375.0).is_some());
    let mut extra_gap = value.clone();
    extra_gap["children"][0]["width"] = json!(1935);
    assert_rejected(&extra_gap, "five pages have four gaps, not five");
    for gap in [json!(-1), json!("$app.gap")] {
        let mut invalid = value.clone();
        invalid["children"][0]["gap"] = gap;
        assert_rejected(&invalid, "gap must be static and nonnegative");
    }
}

#[test]
fn one_lane_with_independent_overlay_controls_remains_identifiable() {
    let mut value = viewport();
    value["children"].as_array_mut().unwrap().push(json!({
        "type":"frame","id":"controls","x":0,"y":0,"width":375,
        "height":44,"layout":"horizontal","children":[{
            "type":"frame","id":"back","role":"icon-button",
            "width":44,"height":44,"children":[{
                "type":"icon_font","id":"back-icon","name":"navigation glyph",
                "iconFontFamily":"lucide","iconFontName":"chevron-left",
                "width":20,"height":20
            }]
        }]
    }));
    value["children"].as_array_mut().unwrap().push(json!({
        "type":"frame","id":"pagination","x":156,"y":320,"width":63,
        "height":8,"layout":"horizontal","gap":5,"children":[
            {"type":"ellipse","id":"dot-1","width":8,"height":8},
            {"type":"ellipse","id":"dot-2","width":8,"height":8}
        ]
    }));
    assert_eq!(media_lane(&value, 375.0).unwrap()["id"], "lane");
}

#[test]
fn ordinary_copy_and_media_siblings_are_not_decorative_overlays() {
    for sibling in [
        json!({"type":"text","id":"body","x":0,"y":0,
            "width":375,"height":40,"content":"普通商品正文"}),
        json!({"type":"image","id":"other-image","x":0,"y":0,
            "width":375,"height":340,"src":"assets/another-product.webp"}),
        json!({"type":"frame","id":"body-frame","x":0,"y":0,
        "width":375,"height":80,"layout":"vertical","children":[{
            "type":"text","id":"nested-body","content":"商品说明需要正常布局"
        }]}),
    ] {
        let mut value = viewport();
        value["children"].as_array_mut().unwrap().push(sibling);
        assert_rejected(
            &value,
            "copy or independent media is not a controls overlay",
        );
        assert!(!is_full_width(value, 375.0));
    }
}

#[test]
fn two_matching_lanes_are_ambiguous() {
    let mut value = viewport();
    let mut second = value["children"][0].clone();
    second["id"] = json!("second-lane");
    for (index, image) in second["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        image["id"] = json!(format!("second-page-{index}"));
    }
    value["children"].as_array_mut().unwrap().push(second);
    assert_rejected(&value, "exactly one media lane must own the viewport");
}

#[test]
fn ordinary_clipped_rows_and_grids_do_not_gain_media_protection() {
    for (path, field, replacement) in [
        ("", "clipContent", json!(false)),
        ("", "layout", json!("horizontal")),
        ("", "layout", json!("vertical")),
        ("/children/0", "layout", json!("vertical")),
        ("/children/0", "clipContent", json!(true)),
        ("/children/0", "width", json!("fill_container")),
        ("/children/0", "width", json!("fit_content")),
        ("/children/0", "wrap", json!(true)),
        ("/children/0/children/0", "width", json!("fill_container")),
    ] {
        let mut value = viewport();
        value.pointer_mut(path).unwrap()[field] = replacement;
        assert_rejected(&value, &format!("{path}/{field}"));
    }

    let mut thumbnails = viewport();
    thumbnails["children"][0]["width"] = json!(600);
    for image in thumbnails["children"][0]["children"]
        .as_array_mut()
        .unwrap()
    {
        image["width"] = json!(120);
    }
    assert_rejected(
        &thumbnails,
        "overflowing thumbnails are not viewport-sized pages",
    );
    assert_rejected(
        &viewport_with_pages(1, 0.0),
        "one cropped image is not a strip",
    );
    assert_rejected(&viewport_with_pages(0, 0.0), "an empty strip has no pages");
}

#[test]
fn malformed_strip_accounting_and_unequal_pages_are_rejected() {
    for (path, field, replacement) in [
        ("", "height", json!(300)),
        ("", "height", json!("$app.height")),
        ("/children/0", "width", json!(1500)),
        ("/children/0", "width", json!(2000)),
        ("/children/0", "height", json!(360)),
        ("/children/0", "height", json!("fit_content")),
        ("/children/0/children/0", "width", json!(360)),
        ("/children/0/children/0", "height", json!(300)),
        ("/children/0/children/0", "width", json!(0)),
        ("/children/0/children/0", "height", json!(-340)),
        ("/children/0/children/0", "width", json!("$app.pageWidth")),
    ] {
        let mut value = viewport();
        value.pointer_mut(path).unwrap()[field] = replacement;
        assert_rejected(&value, &format!("{path}/{field}"));
    }
}

#[test]
fn cards_references_and_copy_are_not_direct_image_pages() {
    for replacement in [
        json!({"type":"ref","id":"reused-page","ref":"component",
            "width":375,"height":340}),
        json!({"type":"frame","id":"image-card","width":375,"height":340,
            "children":[{"type":"image","id":"nested-image","src":"",
                "width":375,"height":340}]}),
        json!({"type":"text","id":"caption","content":"商品说明",
            "width":375,"height":340}),
    ] {
        let mut value = viewport();
        value["children"][0]["children"][0] = replacement;
        assert_rejected(&value, "only direct Image nodes establish the media strip");
    }
    let mut caption_card = viewport();
    caption_card["children"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "type":"text","id":"caption","content":"商品说明","height":20
        }));
    assert_rejected(
        &caption_card,
        "a clipped caption card must retain normal repair",
    );
}

#[test]
fn hidden_dynamic_or_constrained_geometry_fails_closed_at_each_level() {
    for path in ["", "/children/0", "/children/0/children/0"] {
        for (field, replacement) in [
            ("visible", json!(false)),
            ("enabled", json!(false)),
            ("enabled", json!("$app.show")),
            ("opacity", json!(0)),
            ("opacity", json!("$app.opacity")),
            ("rotation", json!(15)),
            ("constraints", json!({"h":"left_right","v":"top"})),
            ("bindings", json!({"width":"$app.width"})),
        ] {
            let mut value = viewport();
            value.pointer_mut(path).unwrap()[field] = replacement;
            assert_rejected(&value, &format!("{path}/{field}"));
        }
    }
}

#[test]
fn insets_offsets_and_positioned_images_cannot_prove_full_page_geometry() {
    for (path, field, replacement) in [
        ("", "padding", json!(24)),
        ("", "padding", json!([0, 24, 0, 0])),
        ("/children/0", "padding", json!([0, 0, 0, 24])),
        ("/children/0", "padding", json!("$app.padding")),
        ("/children/0", "x", json!(24)),
        ("/children/0", "y", json!(8)),
        ("/children/0/children/0", "x", json!(0)),
        ("/children/0/children/0", "y", json!(0)),
        ("/children/0/children/0", "position", json!("absolute")),
    ] {
        let mut value = viewport();
        value.pointer_mut(path).unwrap()[field] = replacement;
        assert_rejected(&value, &format!("{path}/{field}"));
    }
}

#[test]
fn typed_full_width_detection_matches_value_detection() {
    for value in [viewport(), viewport_with_pages(2, 12.0)] {
        let node: PenNode = serde_json::from_value(value.clone()).unwrap();
        let roundtrip = serde_json::to_value(&node).unwrap();
        assert!(media_lane(&value, 375.0).is_some());
        assert!(media_lane(&roundtrip, 375.0).is_some());
        assert!(full_width_media_section(&node, 375.0));
        assert!(!full_width_media_section(&node, 327.0));
        assert!(is_full_width(section(value), 375.0));
    }
}

#[test]
fn only_one_unpadded_full_width_wrapper_can_own_the_media_viewport() {
    let mut numeric_width = section(viewport());
    numeric_width["width"] = json!(375);
    numeric_width["padding"] = json!(0);
    assert!(is_full_width(numeric_width, 375.0));

    for (field, replacement) in [
        ("width", json!(327)),
        ("width", json!("fit_content")),
        ("padding", json!([0, 24])),
        ("visible", json!(false)),
        ("enabled", json!(false)),
        ("rotation", json!(15)),
        ("bindings", json!({"width":"$app.width"})),
    ] {
        let mut wrapper = section(viewport());
        wrapper[field] = replacement;
        assert!(!is_full_width(wrapper, 375.0), "wrapper {field}");
    }

    assert!(!is_full_width(section(section(viewport())), 375.0));
    let mut ordinary_section = section(viewport());
    ordinary_section["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "type":"text","id":"body","content":"普通正文需要内容边距"
        }));
    assert!(!is_full_width(ordinary_section, 375.0));
}

#[test]
fn decoration_bindings_are_rejected_before_and_after_typed_roundtrip() {
    for mut decoration in [
        icon(),
        json!({"type":"ellipse","id":"decoration","width":8,"height":8}),
        json!({"type":"path","id":"decoration","d":"M0 0 L8 8","width":8,"height":8}),
        json!({"type":"line","id":"decoration","x2":8,"y2":8}),
        json!({"type":"polygon","id":"decoration","polygonCount":3,"width":8,"height":8}),
        json!({"type":"rectangle","id":"decoration","width":8,"height":8}),
    ] {
        let mut value = viewport();
        value["children"]
            .as_array_mut()
            .unwrap()
            .push(decorative_overlay(decoration.clone()));
        assert!(is_full_width(value.clone(), 375.0));
        decoration["bindings"] = json!({"opacity":"$app.overlayOpacity"});
        value["children"][1] = decorative_overlay(decoration);
        assert_value_and_typed_rejected(
            value,
            "dynamic decoration must not split typed/value proof",
        );
    }
}

#[test]
fn decoration_size_limits_are_rejected_before_and_after_typed_roundtrip() {
    for mut decoration in [
        icon(),
        json!({"type":"ellipse","id":"decoration","width":8,"height":8}),
        json!({"type":"path","id":"decoration","d":"M0 0 L8 8","width":8,"height":8}),
        json!({"type":"polygon","id":"decoration","polygonCount":3,"width":8,"height":8}),
        json!({"type":"rectangle","id":"decoration","width":8,"height":8}),
    ] {
        decoration["minWidth"] = json!(40);
        let mut value = viewport();
        value["children"]
            .as_array_mut()
            .unwrap()
            .push(decorative_overlay(decoration));
        assert_value_and_typed_rejected(
            value,
            "actual decoration limits must survive typed inspection",
        );
    }
}

#[test]
fn image_paint_in_frame_or_rectangle_overlay_is_not_pure_decoration() {
    let image_fill = json!([{"type":"image","url":"assets/other-product.webp","mode":"fill"}]);
    let mut painted_frame = decorative_overlay(icon());
    painted_frame["fill"] = image_fill.clone();
    let painted_rectangle = decorative_overlay(json!({
        "type":"rectangle","id":"painted-photo","width":375,"height":340,
        "fill":image_fill
    }));
    for overlay in [painted_frame, painted_rectangle] {
        let mut value = viewport();
        value["children"].as_array_mut().unwrap().push(overlay);
        assert_value_and_typed_rejected(
            value,
            "image paint is independent media, not an icon overlay",
        );
    }
}

#[test]
fn shifted_viewports_cannot_claim_full_width_rail_ownership() {
    for value in [viewport(), single_image_viewport()] {
        for axis in ["x", "y"] {
            let mut shifted = value.clone();
            shifted[axis] = json!(24);
            assert!(
                !is_full_width(shifted.clone(), 375.0),
                "direct viewport {axis}"
            );
            assert!(
                !is_full_width(section(shifted), 375.0),
                "wrapped viewport {axis}"
            );
        }
    }
}

#[test]
fn single_full_page_image_with_icon_overlay_owns_rail_but_not_strip_protection() {
    let mut value = single_image_viewport();
    value["children"]
        .as_array_mut()
        .unwrap()
        .push(decorative_overlay(icon()));
    assert!(
        media_lane(&value, 375.0).is_none(),
        "single media is a rail-only exception"
    );
    assert!(is_full_width(value.clone(), 375.0));
    assert!(is_full_width(section(value.clone()), 375.0));
    let node: PenNode = serde_json::from_value(value).unwrap();
    assert!(is_full_width(serde_json::to_value(node).unwrap(), 375.0));
}

#[test]
fn single_image_cards_and_authored_insets_keep_normal_rail_rules() {
    for (path, field, replacement) in [
        ("/children/0", "width", json!(327)),
        ("/children/0", "height", json!(300)),
        ("/children/0", "height", json!(400)),
        ("", "clipContent", json!(false)),
        ("", "padding", json!([0, 24])),
    ] {
        let mut value = single_image_viewport();
        value.pointer_mut(path).unwrap()[field] = replacement;
        assert!(!is_full_width(value, 375.0), "single image {path}/{field}");
    }

    let mut caption_card = single_image_viewport();
    caption_card["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "type":"text","id":"caption","content":"普通商品说明","width":375,"height":24
        }));
    assert!(!is_full_width(caption_card, 375.0));

    let mut multiple_images = single_image_viewport();
    let mut second = multiple_images["children"][0].clone();
    second["id"] = json!("another-image");
    multiple_images["children"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(!is_full_width(multiple_images, 375.0));

    let mut inset_wrapper = section(single_image_viewport());
    inset_wrapper["padding"] = json!([0, 24, 0, 24]);
    assert!(!is_full_width(inset_wrapper, 375.0));
}
