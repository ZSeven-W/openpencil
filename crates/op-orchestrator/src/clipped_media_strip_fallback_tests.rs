use super::{full_width_media_section, full_width_viewport, media_lane};
use jian_ops_schema::node::PenNode;
use op_image_enrich::{image_fallback_policy, is_unchanged_image_fallback, ImageFallbackBranch};
use serde_json::{json, Value};

fn image(id: &str) -> Value {
    json!({"type":"image","id":id,"name":"Authored product photo",
        "width":375,"height":340,"src":"",
        "imageSearchQuery":"stoneware ceramic mug"})
}

fn merge_patch(value: &mut Value, patch: &Value) {
    let Some(fields) = patch.as_object() else {
        *value = patch.clone();
        return;
    };
    if !value.is_object() {
        *value = json!({});
    }
    let target = value.as_object_mut().unwrap();
    for (key, replacement) in fields {
        if replacement.is_null() {
            target.remove(key);
        } else {
            merge_patch(
                target.entry(key.clone()).or_insert(Value::Null),
                replacement,
            );
        }
    }
}

fn fallback(id: &str) -> Value {
    let mut value = image(id);
    let original: PenNode = serde_json::from_value(value.clone()).unwrap();
    let patches = image_fallback_policy(&original, true);
    assert_eq!(
        patches.len(),
        1,
        "exercise the actual post-enrichment policy"
    );
    assert_eq!(patches[0].branch, ImageFallbackBranch::Media);
    let patch: Value = serde_json::from_str(&patches[0].patch_json).unwrap();
    merge_patch(&mut value, &patch);
    assert_eq!(value["id"], id);
    assert_eq!(value["width"], 375);
    assert_eq!(value["height"], 340);
    let node: PenNode = serde_json::from_value(value.clone()).unwrap();
    assert!(is_unchanged_image_fallback(&node));
    value
}

fn strip(pages: Vec<Value>) -> Value {
    let width = pages.len() * 375;
    json!({"type":"frame","id":"viewport","layout":"none",
    "width":375,"height":340,"clipContent":true,"children":[{
        "type":"frame","id":"lane","layout":"horizontal",
        "x":0,"y":0,"width":width,"height":340,"clipContent":false,
        "children":pages
    }]})
}

fn single(page: Value) -> Value {
    json!({"type":"frame","id":"viewport","layout":"none",
        "width":375,"height":340,"clipContent":true,"children":[page]})
}

fn wrapper(viewport: Value) -> Value {
    json!({"type":"frame","id":"section","layout":"vertical",
        "width":"fill_container","height":"fit_content","children":[viewport]})
}

fn assert_parity(value: &Value, expected: bool, has_lane: bool, reason: &str) {
    let before = value.clone();
    assert_eq!(
        media_lane(value, 375.0).is_some(),
        has_lane,
        "raw lane: {reason}"
    );
    assert_eq!(
        full_width_viewport(value, 375.0),
        expected,
        "raw viewport: {reason}"
    );
    let typed: PenNode = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        full_width_media_section(&typed, 375.0),
        expected,
        "typed: {reason}"
    );
    let serialized = serde_json::to_value(&typed).unwrap();
    assert_eq!(
        media_lane(&serialized, 375.0).is_some(),
        has_lane,
        "serialized lane: {reason}"
    );
    assert_eq!(
        full_width_viewport(&serialized, 375.0),
        expected,
        "serialized viewport: {reason}"
    );
    let section: PenNode = serde_json::from_value(wrapper(value.clone())).unwrap();
    assert_eq!(
        full_width_media_section(&section, 375.0),
        expected,
        "wrapped: {reason}"
    );
    assert_eq!(
        value, &before,
        "recognition cannot change policy-owned content"
    );
}

fn assert_rejected_page(page: Value, reason: &str) {
    assert_parity(
        &strip(vec![image("untouched"), page.clone()]),
        false,
        false,
        reason,
    );
    assert_parity(&single(page), false, false, reason);
}

fn edit_metadata(tile: &mut Value, edit: impl FnOnce(&mut Value)) {
    let explain = tile["explain"].as_str().unwrap();
    let split = explain.find('{').expect("policy metadata envelope");
    let prefix = explain[..split].to_owned();
    let mut metadata: Value = serde_json::from_str(&explain[split..]).unwrap();
    edit(&mut metadata);
    tile["explain"] = json!(format!("{prefix}{metadata}"));
}

#[test]
fn canonical_fallback_pages_preserve_mixed_and_all_fallback_strips() {
    for pages in [
        vec![image("photo-1"), fallback("photo-2"), image("photo-3")],
        vec![fallback("photo-1"), fallback("photo-2")],
        (0..5)
            .map(|index| fallback(&format!("photo-{index}")))
            .collect(),
    ] {
        assert_parity(
            &strip(pages),
            true,
            true,
            "actual image fallback policy output",
        );
    }
}

#[test]
fn canonical_fallback_does_not_depend_on_the_display_name() {
    let mut page = fallback("photo");
    page.as_object_mut().unwrap().remove("name");
    assert_parity(
        &strip(vec![image("original"), page.clone()]),
        true,
        true,
        "validated envelope and unchanged tile",
    );
    assert_parity(&single(page), true, false, "single unnamed canonical tile");
}

#[test]
fn ordinary_frames_and_marker_only_forgery_do_not_become_media_pages() {
    let canonical = fallback("photo");
    for (name, explain) in [
        (Value::Null, Value::Null),
        (canonical["name"].clone(), Value::Null),
        (Value::Null, canonical["explain"].clone()),
        (canonical["name"].clone(), canonical["explain"].clone()),
    ] {
        let page = json!({"type":"frame","id":"photo","width":375,"height":340,
            "name":name,"explain":explain,
            "imageSearchQuery":canonical["imageSearchQuery"],"children":[]});
        assert_rejected_page(page, "markers do not prove an unchanged policy tile");
    }
    let mut name_only = canonical;
    name_only.as_object_mut().unwrap().remove("explain");
    assert_rejected_page(
        name_only,
        "even canonical decoration needs valid saved intent",
    );
}

#[test]
fn malformed_or_inconsistent_fallback_metadata_is_rejected() {
    for explain in [
        "image fallback: media {",
        "image fallback: media {}",
        "image fallback: unknown {}",
    ] {
        let mut page = fallback("photo");
        page["explain"] = json!(explain);
        assert_rejected_page(page, "malformed fallback envelope");
    }
    for field in [
        "imageSearchQuery",
        "imagePrompt",
        "imageRequestMode",
        "originalExplain",
    ] {
        let mut missing = fallback("photo");
        edit_metadata(&mut missing, |metadata| {
            metadata.as_object_mut().unwrap().remove(field);
        });
        assert_rejected_page(missing, "required metadata field removed");
        let mut invalid = fallback("photo");
        edit_metadata(&mut invalid, |metadata| {
            metadata[field] = json!(false);
        });
        assert_rejected_page(invalid, "metadata field has an invalid type");
    }
    let mut changed_query = fallback("photo");
    changed_query["imageSearchQuery"] = json!("a different authored subject");
    assert_rejected_page(changed_query, "live query differs from saved intent");
}

#[test]
fn edited_policy_caption_is_authored_content_in_both_media_paths() {
    let mut page = fallback("photo");
    let caption = page["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|child| child["type"] == "text")
        .expect("policy-owned caption");
    caption["content"] = json!("New business copy authored after image enrichment");
    assert_rejected_page(
        page,
        "changed caption cannot be treated as untouched fallback media",
    );
}

#[test]
fn additional_business_content_is_not_ignored_inside_a_fallback_tile() {
    let mut page = fallback("photo");
    page["children"].as_array_mut().unwrap().push(json!({
        "type":"text","id":"business-copy","content":"Product details and price"
    }));
    assert_rejected_page(page, "additional text child is authored content");
}

#[test]
fn modified_fallback_decoration_is_rejected_after_typed_roundtrip() {
    for index in [0, 1] {
        for (field, replacement) in [
            ("visible", json!(false)),
            ("bindings", json!({"opacity":"$app.opacity"})),
            ("minWidth", json!(10)),
            ("x", json!(4)),
        ] {
            let mut page = fallback("photo");
            page["children"][index][field] = replacement;
            assert_rejected_page(page, &format!("edited policy child {index}/{field}"));
        }
    }
}

#[test]
fn canonical_fallback_still_requires_static_matching_page_geometry() {
    for (field, replacement) in [
        ("visible", json!(false)),
        ("enabled", json!(false)),
        ("enabled", json!("$app.enabled")),
        ("opacity", json!(0)),
        ("bindings", json!({"width":"$app.width"})),
        ("minWidth", json!(300)),
        ("maxHeight", json!(400)),
        ("constraints", json!({"h":"left","v":"top"})),
        ("rotation", json!(15)),
        ("x", json!(12)),
        ("y", json!(12)),
        ("width", json!(327)),
        ("height", json!(300)),
        ("width", json!("fill_container")),
        ("height", json!("fit_content")),
        ("padding", json!("$app.pad")),
        ("padding", json!([0, 24])),
        ("gap", json!("$app.gap")),
        ("gap", json!(-1)),
    ] {
        let mut page = fallback("photo");
        page[field] = replacement;
        assert_rejected_page(page, &format!("fallback page {field}"));
    }
}

#[test]
fn positioned_fallback_pages_do_not_pass_the_lane_proof() {
    for field in ["x", "y"] {
        let mut page = fallback("photo");
        page[field] = json!(0);
        assert_parity(
            &strip(vec![image("original"), page]),
            false,
            false,
            "lane pages must be unpositioned even when the offset is zero",
        );
    }
    // `position` is an untyped input field; typed nodes express placement via x/y.
    let mut page = fallback("photo");
    page["position"] = json!("absolute");
    let viewport = strip(vec![image("original"), page.clone()]);
    assert!(media_lane(&viewport, 375.0).is_none());
    assert!(!full_width_viewport(&single(page), 375.0));
}

#[test]
fn canonical_single_fallback_owns_the_rail_with_decorative_controls() {
    let mut page = fallback("photo");
    page["x"] = json!(0);
    page["y"] = json!(0);
    let mut viewport = single(page);
    viewport["children"].as_array_mut().unwrap().push(json!({
        "type":"frame","id":"controls","x":0,"y":0,"width":375,
        "height":44,"layout":"horizontal","children":[{
            "type":"icon_font","id":"back-icon","iconFontFamily":"lucide",
            "iconFontName":"chevron-left","width":20,"height":20
        }]
    }));
    assert_parity(
        &viewport,
        true,
        false,
        "canonical single-image fallback is rail-only",
    );
}

#[test]
fn canonical_fallback_can_retain_static_nonnegative_decoration_gap() {
    let mut page = fallback("photo");
    page["gap"] = json!(12);
    assert_parity(
        &strip(vec![image("original"), page.clone()]),
        true,
        true,
        "static fallback decoration gap does not change page geometry",
    );
    assert_parity(
        &single(page),
        true,
        false,
        "static gap on a single fallback",
    );
}

#[test]
fn canonical_single_fallback_does_not_bypass_the_viewport_boundary() {
    for (field, replacement) in [
        ("clipContent", json!(false)),
        ("width", json!(327)),
        ("height", json!(300)),
        ("padding", json!([0, 24])),
        ("x", json!(24)),
        ("y", json!(24)),
    ] {
        let mut viewport = single(fallback("photo"));
        viewport[field] = replacement;
        assert_parity(
            &viewport,
            false,
            false,
            &format!("single fallback viewport {field}"),
        );
    }
    let mut padded_wrapper = wrapper(single(fallback("photo")));
    padded_wrapper["padding"] = json!([0, 24]);
    let typed: PenNode = serde_json::from_value(padded_wrapper).unwrap();
    assert!(
        !full_width_media_section(&typed, 375.0),
        "authored wrapper padding owns its rail"
    );
}

#[test]
fn canonical_single_fallback_does_not_absorb_caption_or_a_second_page() {
    for sibling in [
        json!({"type":"text","id":"caption","content":"Product description"}),
        image("another-photo"),
        fallback("another-fallback"),
    ] {
        let mut viewport = single(fallback("photo"));
        viewport["children"].as_array_mut().unwrap().push(sibling);
        assert_parity(
            &viewport,
            false,
            false,
            "single-media viewport has additional content",
        );
    }
}
