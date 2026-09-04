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
    let clip_per_corner = source
        .find("clipRoundRectPerCorner(x, y, w, h, tl, tr, br, bl)")
        .expect("CanvasKit per-corner clip bridge marker exists");
    let per_corner_op = source[clip_per_corner..]
        .find("canvas.clipRRect(rr, CK.ClipOp.Intersect, true)")
        .expect("per-corner clip uses CanvasKit clipRRect")
        + clip_per_corner;

    assert!(
        clip_rect < rect_op
            && rect_op < intersect
            && intersect < clip_round
            && clip_round < round_op,
        "CanvasKit clips must use explicit intersect clip ops, not stale Canvas2D paths"
    );
    assert!(
        clip_per_corner < per_corner_op,
        "CanvasKit per-corner clips must intersect an RRect"
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
        "drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a, seg.emoji)",
        "browserTextMeasure(seg.text, sz, weight, italic)",
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

    // The browser/system-font fallback fast path (which returns before any
    // CanvasKit Paint is allocated) lives in the shared `drawScriptRun`
    // helper that `drawText` — and the uncovered segments of a family-aware
    // run — delegate to.
    let script_run = source
        .find("const drawScriptRun = (t, x, y, sz, weight, italic, r, g, b, a) =>")
        .expect("drawScriptRun helper exists");
    let fallback_fast_path = source[script_run..]
        .find("if (allSegmentsUseBrowserTextFallback(segs))")
        .expect("browser text fast path exists")
        + script_run;
    let fill_paint = source[script_run..]
        .find("const p = fillPaint(r, g, b, a);")
        .expect("CanvasKit paint fallback exists")
        + script_run;
    assert!(
        fallback_fast_path < fill_paint,
        "drawScriptRun must take the browser/system font path before allocating CanvasKit Paint"
    );
}

#[test]
fn canvaskit_text_measurement_uses_draw_text_style() {
    let bridge = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");
    let backend = canvaskit_source();

    for marker in [
        "measureTextStyled(t, sz, weight, italic)",
        "browserTextMeasure(seg.text, sz, weight, italic)",
    ] {
        assert!(
            bridge.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so measured text matches drawn text"
        );
    }

    for marker in [
        "fn measure_text_styled(",
        "weight: u16,",
        "italic: bool,",
        ".measure_text_styled(text, font_size, weight as i32, italic)",
    ] {
        assert!(
            backend.contains(marker),
            "CanvasKit backend must preserve `{marker}` so web layout measures with draw-time text style"
        );
    }
}

#[test]
fn canvaskit_image_transform_uses_inverse_shader_local_matrix() {
    let bridge = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");
    let backend = canvaskit_source();

    for marker in [
        "const figmaImageLocalMatrix =",
        "const invDet = 1 / det;",
        "image.makeShaderOptions(tileMode, tileMode, CK.FilterMode.Linear",
        "paint.setShader(shader);",
    ] {
        assert!(
            bridge.contains(marker),
            "missing CanvasKit image transform marker: {marker}"
        );
    }
    assert!(backend.contains("fn draw_image_with_options_and_transform("));
}

#[test]
fn canvaskit_exposes_font_metric_ascent() {
    let bridge = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "textAscent(family, sz, weight)",
        "font.getMetrics()",
        "-metrics.ascent",
    ] {
        assert!(
            bridge.contains(marker),
            "missing CanvasKit ascent marker: {marker}"
        );
    }
}

#[test]
fn canvaskit_paint_style_is_guarded_before_set_style() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    for marker in [
        "const setPaintStyle = (paint, style) =>",
        "setPaintStyle(p, CK.PaintStyle.Fill)",
        "setPaintStyle(p, CK.PaintStyle.Stroke)",
        "setPaintStyle(p, CK.PaintStyle.StrokeAndFill)",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so optional PaintStyle enums cannot throw during repaint"
        );
    }

    assert!(
        !source.contains("p.setStyle(CK.PaintStyle.Fill)")
            && !source.contains("p.setStyle(CK.PaintStyle.Stroke);")
            && !source.contains("p.setStyle(CK.PaintStyle.StrokeAndFill)"),
        "CanvasKit bridge must guard PaintStyle before setStyle to avoid leaving the web host stuck in a borrowed repaint"
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
        "browserTextMeasure(seg.text, sz, weight, italic)",
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
        "Hiragino Kaku Gothic ProN",
        "Yu Gothic",
        "Kohinoor Devanagari",
        "Devanagari Sangam MN",
        "Nirmala UI",
        "Mangal",
        "Thonburi",
        "Leelawadee UI",
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
        "path.computeTightBounds()",
        "path.getBounds()",
        "const sx = Math.abs(nativeW) > 0.01 ? w / nativeW : 1;",
        "const sy = Math.abs(nativeH) > 0.01 ? h / nativeH : 1;",
        "fillSvgPathInRect(d, x, y, w, h, evenOdd, r, g, b, a)",
        "strokeSvgPathInRect(d, x, y, w, h, r, g, b, a, sw)",
    ] {
        assert!(
            bridge.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so arbitrary .op path nodes fit tight bounds with a compatibility fallback"
        );
    }

    let backend = canvaskit_source();
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
fn canvaskit_browser_text_raster_key_drops_color_and_alpha() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    // Text keys omit colour so differently tinted runs reuse one raster.
    assert!(
        source.contains(": [t, sz, weight, italic ? 1 : 0, ss, family].join('\\n');"),
        "text raster cache key must contain no colour/alpha component"
    );
    assert!(
        source.contains(": 'rgba(255, 255, 255, 1)';"),
        "text glyph run must rasterize as a white full-alpha coverage mask"
    );
    // Only emoji retain the legacy RGBA-keyed raster path.
    assert!(
        !source.contains("const key = [t, sz, weight, italic ? 1 : 0, r, g, b, a, ss]"),
        "the always-RGBA-keyed raster cache (the thrash source) must be gone"
    );
    // Colour glyphs ignore fillStyle, so tinting a white mask would flatten them.
    assert!(
        source.contains(
            "const browserTextImage = (t, sz, weight, italic, emoji, r, g, b, a, family = '') =>"
        ),
        "browserTextImage must take an `emoji` selector + colour for the legacy emoji path"
    );
    assert!(
        source.contains("['e', t, sz, weight, italic ? 1 : 0, r, g, b, a, ss, family].join('\\n')"),
        "emoji segments must keep a colour-keyed raster (legacy baked-colour path)"
    );
    assert!(
        source.contains("`rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`"),
        "emoji branch must bake the requested colour into fillStyle (colour glyphs ignore a white mask + tint)"
    );
}

#[test]
fn canvaskit_text_rasters_reuse_quantized_effective_scale() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    let scale_fn = source
        .find("const effectiveTextScale = () =>")
        .expect("effective text scale helper exists");
    let image_fn = source
        .find("const browserTextImage = (t, sz, weight, italic, emoji, r, g, b, a, family = '') =>")
        .expect("browserTextImage exists");
    let image_end = source[image_fn..]
        .find("const TINT_FILTER_CACHE_CAP = 64;")
        .expect("browserTextImage ends before the tint-filter cache")
        + image_fn;
    let scale_body = &source[scale_fn..image_fn];
    let image_body = &source[image_fn..image_end];
    for marker in [
        "const m = canvas.getTotalMatrix();",
        "const xScale = Math.hypot(m[0], m[3]);",
        "const yScale = Math.hypot(m[1], m[4]);",
        "return Math.max(1, xScale, yScale);",
    ] {
        assert!(
            scale_body.contains(marker),
            "effective text scale must preserve `{marker}` for rotated/non-uniform transforms"
        );
    }
    assert!(
        source.contains("const TEXT_SCALE_STEPS_PER_OCTAVE = 4;")
            && source.contains("export function opCkQuantizeTextRasterScale(scale)")
            && source.contains("Math.log2(finiteScale) * TEXT_SCALE_STEPS_PER_OCTAVE - 1e-9")
            && scale_body.contains("const browserTextRasterScale = () =>")
            && scale_body.contains("opCkQuantizeTextRasterScale(effectiveTextScale())"),
        "browser text raster scale must round up to reusable quarter-octave buckets"
    );
    assert!(
        image_body.contains("const ss = browserTextRasterScale();"),
        "browserTextImage must supersample at the quantized effective scale"
    );
    for forbidden in ["clearBrowserTextCache()", "browserTextCache.clear()"] {
        assert!(
            !scale_body.contains(forbidden) && !image_body.contains(forbidden),
            "effective-scale changes must partition the LRU by `ss`, not call `{forbidden}` per draw"
        );
    }

    for marker in [
        "['e', t, sz, weight, italic ? 1 : 0, r, g, b, a, ss, family].join('\\n')",
        ": [t, sz, weight, italic ? 1 : 0, ss, family].join('\\n');",
    ] {
        assert!(
            image_body.contains(marker),
            "text raster key must preserve `{marker}`"
        );
    }
    assert!(
        !image_body.contains("[t, sz, weight, italic ? 1 : 0, r, g, b, a, ss]"),
        "ordinary text raster keys must remain colour-independent"
    );

    let clear_fn = source
        .find("const clearBrowserTextCache = () =>")
        .expect("browser text cache cleanup helper exists");
    let text_dpr = source[clear_fn..]
        .find("let textDpr = 1;")
        .expect("cache cleanup helper ends before textDpr state")
        + clear_fn;
    let clear_body = &source[clear_fn..text_dpr];
    let delete = clear_body
        .find("entry.image.delete()")
        .expect("cache cleanup deletes CanvasKit image handles");
    let clear = clear_body[delete..]
        .find("browserTextCache.clear();")
        .expect("cache cleanup clears the map")
        + delete;
    let set_dpr = source.find("setDpr(v)").expect("setDpr bridge exists");
    let set_dpr_end = source[set_dpr..]
        .find("\n    },")
        .expect("setDpr method has a local end boundary")
        + set_dpr;
    let set_dpr_body = &source[set_dpr..set_dpr_end];
    let clear_call = set_dpr_body
        .find("clearBrowserTextCache();")
        .expect("DPR changes clear stale text rasters");
    assert!(delete < clear && clear_call < set_dpr_body.len());
    assert_eq!(
        source.matches("clearBrowserTextCache();").count(),
        1,
        "setDpr must be the only call site that clears all browser text rasters"
    );
    assert_eq!(
        source.matches("browserTextCache.clear();").count(),
        1,
        "only the handle-safe cleanup helper may clear the browser text raster map"
    );

    let eviction = image_body
        .find("if (browserTextCache.size > 512)")
        .expect("browser text raster cache remains bounded");
    assert!(
        image_body[eviction..].contains("old.image.delete()"),
        "LRU eviction must keep deleting the evicted CanvasKit image handle"
    );
}

#[test]
fn canvaskit_browser_text_cache_is_lru() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    // Cache hits reinsert entries so eviction remains LRU rather than FIFO.
    let image_fn = source
        .find("const browserTextImage = (t, sz, weight, italic, emoji, r, g, b, a, family = '') =>")
        .expect("browserTextImage marker exists");
    let hit = source[image_fn..]
        .find("const hit = browserTextCache.get(key);")
        .expect("browserTextImage looks up the cache before rasterizing")
        + image_fn;
    let reinsert_delete = source[hit..]
        .find("browserTextCache.delete(key);")
        .expect("LRU hit re-inserts by deleting first")
        + hit;
    let reinsert_set = source[reinsert_delete..]
        .find("browserTextCache.set(key, hit);")
        .expect("LRU hit re-inserts to move the entry to most-recent")
        + reinsert_delete;
    // Eviction still deletes the wasm image handle of the dropped entry.
    let evict = source[reinsert_set..]
        .find("if (browserTextCache.size > 512)")
        .expect("512-cap eviction still bounds the raster cache")
        + reinsert_set;
    let evict_delete = source[evict..]
        .find("old.image.delete()")
        .expect("evicted raster image handle is deleted")
        + evict;
    assert!(
        hit < reinsert_delete
            && reinsert_delete < reinsert_set
            && reinsert_set < evict
            && evict < evict_delete,
        "browser text raster cache must be LRU (re-insert on hit) and delete evicted image handles"
    );
}

#[test]
fn canvaskit_text_tint_color_filter_cache_is_bounded_and_freed() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    // The bounded white-mask SrcIn cache keys RGB; alpha rides the paint.
    for marker in [
        "const TINT_FILTER_CACHE_CAP = 64;",
        "const tintFilterCache = new Map();",
        "const tintColorFilter = (r, g, b) =>",
        "CK.ColorFilter.MakeBlend(col(r, g, b, 1), CK.BlendMode.SrcIn)",
        "const key = r + '\\n' + g + '\\n' + b;",
    ] {
        assert!(
            source.contains(marker),
            "tint filter cache must preserve `{marker}` (rgb-keyed SrcIn blend, bounded LRU)"
        );
    }

    // LRU re-insert on hit + delete on eviction, mirroring svgPathCache hygiene.
    let cache_fn = source
        .find("const tintColorFilter = (r, g, b) =>")
        .expect("tintColorFilter marker exists");
    let hit = source[cache_fn..]
        .find("const hit = tintFilterCache.get(key);")
        .expect("tintColorFilter looks up the cache")
        + cache_fn;
    let reinsert = source[hit..]
        .find("tintFilterCache.set(key, hit);")
        .expect("tint filter LRU re-inserts on hit")
        + hit;
    let evict = source[reinsert..]
        .find("if (tintFilterCache.size > TINT_FILTER_CACHE_CAP)")
        .expect("tint filter cache is bounded")
        + reinsert;
    let evict_delete = source[evict..]
        .find("old.delete()")
        .expect("evicted ColorFilter wasm handle is deleted")
        + evict;
    assert!(
        hit < reinsert && reinsert < evict && evict < evict_delete,
        "tint ColorFilter cache must be LRU and delete evicted wasm handles"
    );

    // Draw site: dedicated tint paint (not the shared fill paint) applies the
    // filter + opacity, then draws the raster with that paint.
    for marker in [
        "const cachedTintPaint = new CK.Paint();",
        "paint.setColorFilter(tintColorFilter(r, g, b) || null);",
        "paint.setAlphaf(a < 0 ? 0 : a > 1 ? 1 : a);",
        "canvas.drawImage(entry.image, 0, 0, paint);",
        "canvas.drawImage(entry.image, x - 2, y - entry.baseline, paint);",
    ] {
        assert!(
            source.contains(marker),
            "drawBrowserText must preserve `{marker}` (tint-at-draw via dedicated paint)"
        );
    }
    // The tint paint must not alias the shared fill/stroke paints.
    assert!(
        !source.contains("const cachedTintPaint = cachedFillPaint"),
        "tint paint must be its own object, not an alias of the shared fill paint"
    );
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

    let backend = canvaskit_source();
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

/// The CanvasKit host source: the `canvaskit.rs` spine plus every sibling
/// module under `canvaskit/` (the file was split at the 800-line ceiling).
fn canvaskit_source() -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut parts = vec![std::fs::read_to_string(format!("{root}/canvaskit.rs"))
        .expect("canvaskit spine is readable")];
    let mut siblings: Vec<std::path::PathBuf> = std::fs::read_dir(format!("{root}/canvaskit"))
        .expect("canvaskit module directory is readable")
        .map(|entry| entry.expect("canvaskit module entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    siblings.sort();
    for path in siblings {
        parts.push(std::fs::read_to_string(&path).expect("canvaskit module is readable"));
    }
    parts.join("\n")
}

#[test]
fn canvaskit_bridge_routes_complex_scripts_through_the_browser_shaper() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    // Browser shaping handles bidi and joining; one call preserves run order
    // across script boundaries where segmented left-to-right drawing cannot.
    for marker in [
        "drawShapedText(t, family, x, y, sz, weight, italic, r, g, b, a)",
        "measureShapedText(t, family, sz, weight, italic)",
        "shapedTextUnavailable",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit bridge must preserve `{marker}` so complex scripts reach the browser shaper unsegmented"
        );
    }
}

#[test]
fn canvaskit_backend_asks_the_shared_predicate_which_text_path_to_use() {
    // Spans the whole `canvaskit/` module set: the externs live in
    // `bindings.rs` and the routing in `backend.rs`.
    let backend = canvaskit_source();

    // The routing decision lives in `op_editor_core::text_script` so native and
    // web can never disagree about which runs get shaped, and so paint and
    // measure on THIS host can never disagree either.
    for marker in [
        "needs_complex_shaping",
        "draw_shaped_text",
        "measure_shaped_text",
    ] {
        assert!(
            backend.contains(marker),
            "CanvasKit backend must reference `{marker}` to route complex scripts to the shaper"
        );
    }
}
