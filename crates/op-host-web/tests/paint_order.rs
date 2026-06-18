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
fn canvaskit_clip_ops_use_intersect_clips_without_canvas2d_paths() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    let clip_rect = source
        .find("clipRect(x, y, w, h)")
        .expect("CanvasKit clipRect bridge marker exists");
    let rect_op = source[clip_rect..]
        .find("canvas.clipRect(CK.LTRBRect")
        .expect("clipRect uses CanvasKit clipRect")
        + clip_rect;
    let intersect = source[rect_op..]
        .find("CK.ClipOp.Intersect")
        .expect("clipRect uses intersect clipping")
        + rect_op;
    let clip_round = source
        .find("clipRoundRect(x, y, w, h, rad)")
        .expect("CanvasKit clipRoundRect bridge marker exists");
    let round_op = source[clip_round..]
        .find("canvas.clipRRect(CK.RRectXY")
        .expect("clipRoundRect uses CanvasKit clipRRect")
        + clip_round;

    assert!(
        clip_rect < rect_op
            && rect_op < intersect
            && intersect < clip_round
            && clip_round < round_op,
        "CanvasKit clips must use explicit intersect clip ops, not stale Canvas2D paths"
    );
}

#[test]
fn canvaskit_frame_resets_save_stack_before_flush() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    let begin = source
        .find("beginFrame()")
        .expect("CanvasKit beginFrame marker exists");
    let begin_restore = source[begin..]
        .find("while (canvas.getSaveCount() > 1) canvas.restore();")
        .expect("beginFrame must clear leaked save/clip/matrix state")
        + begin;
    let save = source[begin_restore..]
        .find("canvas.save();")
        .expect("beginFrame saves a clean frame baseline")
        + begin_restore;
    let end = source
        .find("endFrame()")
        .expect("CanvasKit endFrame marker exists");
    let end_restore = source[end..]
        .find("while (canvas.getSaveCount() > 1) canvas.restore();")
        .expect("endFrame restores back to the baseline")
        + end;
    let flush = source[end_restore..]
        .find("surface.flush();")
        .expect("endFrame flushes after restoring canvas state")
        + end_restore;

    assert!(
        begin < begin_restore
            && begin_restore < save
            && save < end
            && end < end_restore
            && end_restore < flush,
        "CanvasKit frames must reset leaked state before painting and restore before flushing"
    );
}

#[test]
fn canvaskit_text_defaults_to_browser_system_font_fallback() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "const hasCjk",
        "const isEmojiCp",
        "const segments = (t)",
        "const shouldUseBrowserTextFallback = (_t, _emojiRun) => Boolean(browserTextCtx);",
        "drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a)",
        "browserTextMeasure(seg.text, sz)",
        "CK.Typeface.GetDefault()",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit text must preserve `{marker}` for browser/system font rendering"
        );
    }
}

#[test]
fn canvaskit_browser_text_fallback_does_not_require_canvas_paint() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "const allSegmentsUseBrowserTextFallback = (segs) =>",
        "const segs = segments(t);",
        "if (allSegmentsUseBrowserTextFallback(segs))",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit drawText must preserve `{marker}` so browser/system text fallback cannot throw from optional PaintStyle setup"
        );
    }

    let draw_text = source
        .find("drawText(t, x, y, sz, weight, italic, r, g, b, a)")
        .expect("drawText bridge exists");
    let fallback_fast_path = source[draw_text..]
        .find("if (allSegmentsUseBrowserTextFallback(segs))")
        .expect("browser text fast path exists")
        + draw_text;
    let fill_paint = source[draw_text..]
        .find("const p = fillPaint(r, g, b, a);")
        .expect("CanvasKit paint fallback exists")
        + draw_text;
    assert!(
        fallback_fast_path < fill_paint,
        "drawText must take the browser/system font path before allocating CanvasKit Paint"
    );
}

#[test]
fn canvaskit_bridge_registers_browser_system_fonts() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "const systemTypefaces",
        "registerSystemFont(family, bytes)",
        "CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(bytes))",
        "systemTypefaceFor(t, emojiRun)",
        "const hasSystemFallbackText",
        "hasSystemFallbackText(t)",
        "Kohinoor Devanagari",
        "Noto Sans Thai",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so web text can use browser system fonts"
        );
    }
}

#[test]
fn canvaskit_bridge_uses_browser_text_fallback_when_local_font_api_is_unavailable() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "const browserTextCanvas",
        "const shouldUseBrowserTextFallback",
        "const drawBrowserText",
        "CK.MakeImageFromCanvasImageSource",
        "browserTextMeasure(seg.text, sz)",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so non-CJK multilingual text can use browser system fonts without bundled font files"
        );
    }
}

#[test]
fn canvaskit_browser_text_fallback_covers_locale_scripts_and_emoji() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "Apple Color Emoji",
        "Segoe UI Emoji",
        "PingFang SC",
        "Apple SD Gothic Neo",
        "Kohinoor Devanagari",
        "Devanagari Sangam MN",
        "Thonburi",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit browser fallback stack must preserve `{marker}` for locale and emoji text"
        );
    }

    let fallback_line = source
        .lines()
        .find(|line| line.contains("const shouldUseBrowserTextFallback"))
        .expect("browser text fallback predicate exists");
    assert!(
        fallback_line.contains("Boolean(browserTextCtx)"),
        "browser text fallback should route text through browser system fonts by default"
    );
    assert!(
        !fallback_line.contains("!hasCjk(t)"),
        "browser text fallback must not exclude CJK/Hangul locale names"
    );
    assert!(
        !fallback_line.contains("!emojiRun"),
        "browser text fallback must not exclude emoji runs from browser system fonts"
    );
}

#[test]
fn canvaskit_bridge_guards_optional_paint_styles() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    assert!(
        source.contains("const isPaintStyle = (style)"),
        "CanvasKit bridge must validate optional PaintStyle enum values before setStyle"
    );
    assert!(
        source.contains("isPaintStyle(CK.PaintStyle.StrokeAndFill)"),
        "CanvasKit bridge must not pass optional StrokeAndFill directly to Paint.setStyle"
    );
}

#[test]
fn canvaskit_svg_path_nodes_fit_to_destination_rect() {
    let bridge = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");
    for marker in [
        "const fitPathToRect = (path, x, y, w, h)",
        "const bounds = path.getBounds();",
        "const sx = Math.abs(nativeW) > 0.01 ? w / nativeW : 1;",
        "const sy = Math.abs(nativeH) > 0.01 ? h / nativeH : 1;",
        "fillSvgPathInRect(d, x, y, w, h, evenOdd, r, g, b, a)",
        "strokeSvgPathInRect(d, x, y, w, h, r, g, b, a, sw)",
    ] {
        assert!(
            bridge.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so arbitrary .op path nodes fit their own bounds"
        );
    }

    let backend =
        std::fs::read_to_string(format!("{}/src/canvaskit.rs", env!("CARGO_MANIFEST_DIR")))
            .expect("CanvasKit backend source is readable");
    for marker in [
        "js_name = fillSvgPathInRect",
        "js_name = strokeSvgPathInRect",
        "fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color)",
        "fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32)",
    ] {
        assert!(
            backend.contains(marker),
            "CanvasKit backend must override `{marker}` instead of using the icon-size fallback"
        );
    }
}

#[test]
fn canvaskit_svg_path_nodes_preserve_gradient_and_inner_shadow_paths() {
    let bridge = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");
    for marker in [
        "fillSvgPathInRectLinearGradient(",
        "fillSvgPathInRectRadialGradient(",
        "fillInnerShadowSvgPath(",
        "CK.Shader.MakeLinearGradient",
        "CK.Shader.MakeRadialGradient",
        "CK.BlendMode.DstOut",
    ] {
        assert!(
            bridge.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so SVG path effects match desktop rendering"
        );
    }

    let backend =
        std::fs::read_to_string(format!("{}/src/canvaskit.rs", env!("CARGO_MANIFEST_DIR")))
            .expect("CanvasKit backend source is readable");
    for marker in [
        "js_name = fillSvgPathInRectLinearGradient",
        "js_name = fillSvgPathInRectRadialGradient",
        "js_name = fillInnerShadowSvgPath",
        "fn fill_svg_path_in_rect_linear_gradient(",
        "fn fill_svg_path_in_rect_radial_gradient(",
        "fn fill_inner_shadow_svg_path(",
    ] {
        assert!(
            backend.contains(marker),
            "CanvasKit backend must override `{marker}` instead of using default fallback rendering"
        );
    }
}
