use super::*;
use std::collections::BTreeMap;

fn payload() -> BrandKitPayload {
    BrandKitPayload {
        label: "Verdant Loop".into(),
        swatches: vec!["#0D775A".into()],
        variables: BTreeMap::new(),
        themes: BTreeMap::new(),
        design_md: None,
    }
}

#[test]
fn finds_the_first_link_in_a_brief() {
    assert_eq!(
        first_url("做一个咖啡 app，品牌参考 https://brew.example/about。谢谢").as_deref(),
        Some("https://brew.example/about")
    );
    assert_eq!(
        first_url("see (http://a.example/x), then https://b.example").as_deref(),
        Some("http://a.example/x")
    );
    assert_eq!(first_url("no link here"), None);
    assert_eq!(first_url("https:// nothing"), None);
}

#[test]
fn a_link_wins_over_a_screenshot() {
    let mut brand = HomeBrandState::default();
    assert!(brand.press(
        "brand: https://brew.example",
        Some(("s.png", "image/png", b"x")),
        10
    ));
    let (_, request) = brand.take_request().unwrap();
    assert_eq!(
        request,
        BrandSourceRequest::Url("https://brew.example".into())
    );
    assert!(matches!(
        &brand.status,
        HomeBrandStatus::Extracting { label } if label == "brew.example"
    ));
}

#[test]
fn a_screenshot_is_used_when_the_brief_has_no_link() {
    let mut brand = HomeBrandState::default();
    assert!(brand.press("a bakery app", Some(("shot.png", "image/png", b"png")), 10));
    let (_, request) = brand.take_request().unwrap();
    assert!(matches!(request, BrandSourceRequest::Image { ref name, .. } if name == "shot.png"));
    assert!(
        !format!("{request:?}").contains("112"),
        "bytes are not dumped"
    );
}

#[test]
fn nothing_to_read_shows_a_timed_hint() {
    let mut brand = HomeBrandState::default();
    assert!(!brand.press("a bakery app", None, 100));
    assert!(brand.take_request().is_none());
    assert!(brand.hint_visible(100 + BRAND_HINT_MS - 1));
    assert!(!brand.hint_visible(100 + BRAND_HINT_MS));
}

#[test]
fn results_stage_the_kit_and_stale_ones_are_dropped() {
    let mut brand = HomeBrandState::default();
    brand.press("https://a.example", None, 1);
    let (first, _) = brand.take_request().unwrap();
    assert!(
        !brand.press("https://b.example", None, 2),
        "busy while extracting"
    );
    assert!(brand.finish(first, Ok(payload()), 3));
    assert_eq!(
        brand.staged().map(|k| k.label.as_str()),
        Some("Verdant Loop")
    );
    assert!(brand.chip_visible());

    brand.press("https://b.example", None, 4);
    let (second, _) = brand.take_request().unwrap();
    brand.clear();
    assert!(
        !brand.finish(second, Ok(payload()), 5),
        "removed before it landed"
    );
    assert!(brand.staged().is_none());
}

#[test]
fn failures_surface_as_a_hint() {
    let mut brand = HomeBrandState::default();
    brand.press("https://a.example", None, 1);
    let (generation, _) = brand.take_request().unwrap();
    assert!(brand.finish(generation, Err("could not fetch".into()), 50));
    assert!(brand.hint_visible(60));
    assert!(!brand.chip_visible());
}
