use super::*;
use op_editor_core::{BrandKitPayload, EditorState};
use std::collections::BTreeMap;

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn home(available: bool) -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.home.visible = true;
    state.editor_ui.home.brand.available = available;
    state
}

fn stage(state: &mut EditorState) {
    let brand = &mut state.editor_ui.home.brand;
    assert!(brand.press("https://leafline.example", None, 10));
    let (generation, _) = brand.take_request().unwrap();
    let kit = BrandKitPayload {
        label: "Leafline".into(),
        swatches: vec![
            "#0D775A".into(),
            "#FFFFFF".into(),
            "#111111".into(),
            "#F97316".into(),
        ],
        variables: BTreeMap::new(),
        themes: BTreeMap::new(),
        design_md: None,
    };
    assert!(brand.finish(generation, Ok(kit), 20));
}

#[test]
fn no_chip_until_a_brand_is_read() {
    let state = home(true);
    let surface = HomeSurface::for_editor(&state).unwrap();
    assert!(surface.brand_chip_rects(W, H).is_none());
}

#[test]
fn the_staged_chip_sits_in_the_input_box_and_its_close_hits() {
    let mut state = home(true);
    stage(&mut state);
    let surface = HomeSurface::for_editor(&state).unwrap();
    let layout = surface.layout(W, H);
    let (chip, close) = surface.brand_chip_rects(W, H).expect("chip");
    let right = |r: Rect| r.origin.x + r.size.x;
    let bottom = |r: Rect| r.origin.y + r.size.y;
    assert!(chip.origin.x >= layout.input_box.origin.x);
    assert!(right(chip) <= right(layout.input_box));
    assert!(bottom(chip) <= bottom(layout.input_box));
    let at = Point2D::new(
        close.origin.x + close.size.x / 2.0,
        close.origin.y + close.size.y / 2.0,
    );
    assert_eq!(surface.hit_test(W, H, at), Some(HomeHit::BrandClear));
    // The rest of the chip still focuses the composer.
    let label = Point2D::new(chip.origin.x + 4.0, chip.origin.y + chip.size.y / 2.0);
    assert_eq!(surface.hit_test(W, H, label), Some(HomeHit::Sheet));
}

#[test]
fn hints_only_where_the_tool_is_live() {
    let mut state = home(false);
    state.editor_ui.home.hover = Some(HomeHit::ReferenceLink);
    let surface = HomeSurface::for_editor(&state).unwrap();
    assert_eq!(
        link_hint_key(&surface),
        None,
        "disabled hosts keep the soon tooltip"
    );

    let mut state = home(true);
    state.editor_ui.home.hover = Some(HomeHit::ReferenceLink);
    let surface = HomeSurface::for_editor_at(&state, 0).unwrap();
    assert_eq!(link_hint_key(&surface), Some("home.tools.brandHint"));

    let mut state = home(true);
    assert!(!state.editor_ui.home.brand.press("no link", None, 100));
    let surface = HomeSurface::for_editor_at(&state, 200).unwrap();
    assert_eq!(link_hint_key(&surface), Some("home.brand.needSource"));
}
