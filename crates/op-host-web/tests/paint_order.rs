#[test]
fn canvas_paints_before_property_panel_overlays() {
    let source = std::fs::read_to_string(format!(
        "{}/src/widget_host/paint.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("paint source is readable");

    let canvas = source
        .find("CanvasViewport::from_editor")
        .expect("canvas paint block marker exists");
    let property = source
        .find("PropertyPanel::for_selection")
        .expect("property panel paint block marker exists");

    assert!(
        canvas < property,
        "PropertyPanel must paint after CanvasViewport so popovers extending into the canvas are not covered"
    );
}

#[test]
fn image_fill_popover_paints_above_status_bar() {
    let source = std::fs::read_to_string(format!(
        "{}/src/widget_host/paint.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("paint source is readable");

    let status = source
        .find("StatusBar::for_editor")
        .expect("status paint block marker exists");
    let overlays = source
        .find("PropertyPanel overlays")
        .expect("property overlay paint block marker exists");

    assert!(
        status < overlays,
        "image-fill popover must paint after StatusBar so the zoom pill cannot cover adjustment rows"
    );
}

#[test]
fn emoji_overlay_clip_uses_fresh_canvas_path() {
    let source = std::fs::read_to_string(format!(
        "{}/src/backend/canvas_target.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("canvas target source is readable");

    let begin = source
        .find("context.begin_path();")
        .expect("emoji overlay clip must begin a fresh Canvas2D path");
    let rect = source
        .find("context.rect(")
        .expect("emoji overlay clip rect marker exists");
    let clip = source
        .find("context.clip();")
        .expect("emoji overlay clip marker exists");

    assert!(
        begin < rect && rect < clip,
        "emoji overlay clip must not reuse a stale Canvas2D path across overlays"
    );
}

#[test]
fn web_present_resets_canvas2d_state_for_crisp_output() {
    let source = std::fs::read_to_string(format!(
        "{}/src/backend/canvas_target.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("canvas target source is readable");

    let reset = source
        .find("context.reset_transform()?;")
        .expect("present must reset Canvas2D transform before writing image data");
    let smoothing = source
        .find("context.set_image_smoothing_enabled(false);")
        .expect("present must disable Canvas2D image smoothing");
    let put = source
        .find("context.put_image_data(&image_data, 0.0, 0.0)?;")
        .expect("present put_image_data marker exists");

    assert!(
        reset < put && smoothing < put,
        "present must write the Skia raster into an untransformed, non-smoothed Canvas2D target"
    );
}

#[test]
fn web_text_fonts_enable_crisp_raster_flags() {
    let source = std::fs::read_to_string(format!(
        "{}/src/backend/fonts.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("font backend source is readable");

    for marker in [
        "set_subpixel(true)",
        "set_baseline_snap(true)",
        "set_edging(skia_safe::font::Edging::SubpixelAntiAlias)",
        "set_hinting(skia_safe::FontHinting::Normal)",
    ] {
        assert!(
            source.contains(marker),
            "web text fonts must set `{marker}` before raster drawing"
        );
    }
}
