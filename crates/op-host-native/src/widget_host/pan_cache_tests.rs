//! Pan bitmap cache: during a live pan gesture, a pure-pan frame must
//! blit the cached offscreen canvas layer instead of re-walking and
//! re-submitting the whole scene (the 38k-node zoomed-out pan burned
//! ~230 ms/frame in GPU op submission).

use super::WidgetHostNative;
use crate::backend::{NativeBackend, NativeFrameBackend};

const W: i32 = 800;
const H: i32 = 600;

fn seed_red_rect(host: &mut WidgetHostNative) {
    let json = r##"{"version":"1.0.0","children":[{"type":"rectangle","id":"r1","x":200,"y":200,"width":120,"height":120,"fill":[{"type":"solid","color":"#ff0000"}]}]}"##;
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    let mut state = op_editor_core::EditorState::from_document(doc);
    state.chat.minimize();
    *host.editor_state_mut() = state;
    host.mark_paint_dirty_for_test();
}

/// Paint one full frame into a fresh raster surface; return the pixels.
fn paint_frame(host: &mut WidgetHostNative, backend: &mut NativeBackend) -> Vec<u8> {
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((W, H)).expect("raster surface allocated");
    surface.canvas().clear(skia_safe::Color::WHITE);
    {
        let mut frame = NativeFrameBackend::new(backend, surface.canvas());
        host.paint(&mut frame, W as f32, H as f32);
    }
    let stride = (W * 4) as usize;
    let mut pixels = vec![0u8; stride * H as usize];
    let info = skia_safe::ImageInfo::new(
        (W, H),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    assert!(surface.read_pixels(&info, &mut pixels, stride, (0, 0)));
    pixels
}

/// Leftmost x on scanline `y` whose pixel is saturated red.
fn leftmost_red_x(pixels: &[u8], y: i32) -> Option<i32> {
    let stride = (W * 4) as usize;
    (0..W).find(|&x| {
        let o = y as usize * stride + x as usize * 4;
        pixels[o] > 200 && pixels[o + 1] < 60 && pixels[o + 2] < 60
    })
}

/// Cache-lifecycle assertions race against sibling tests bumping the
/// GLOBAL `jian_skia` font generation (a bump correctly rebuilds the
/// scene and drops the pan cache). When churn is detected the run is
/// inconclusive — skip rather than flake; isolated runs always assert.
fn fonts_stable_since(generation: u64) -> bool {
    let stable = jian_skia::font_generation() == generation;
    if !stable {
        eprintln!("pan_cache test inconclusive: font generation churned mid-test");
    }
    stable
}

#[test]
fn pan_gesture_frames_blit_the_cached_canvas_layer() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    // Frame 1: cold (no gesture) — normal paint, no cache involved.
    host.set_now_ms(1_000);
    let cold = paint_frame(&mut host, &mut backend);
    let y_probe = 300;
    let x0 = leftmost_red_x(&cold, y_probe).expect("red rect visible on cold frame");
    assert_eq!(host.pan_cache_blits_for_test(), 0);

    // Frame 2: first hot pan frame — renders the cache layer and blits
    // it at zero offset. Content must land exactly where a normal
    // paint would put it (shifted by the pan delta).
    let pan_before = host.editor_state().viewport.pan_x;
    assert!(host.apply_pan_gesture(350.0, 150.0, 50.0, 0.0, W as f32, H as f32));
    assert_eq!(
        host.editor_state().viewport.pan_x - pan_before,
        50.0,
        "gesture must route to the canvas pan branch"
    );
    assert!(host.fast_interaction_active());
    let hot1 = paint_frame(&mut host, &mut backend);
    let x1 = leftmost_red_x(&hot1, y_probe).expect("red rect visible on first hot frame");
    assert_eq!(x1 - x0, 50);

    // Frame 3: second hot pan frame with no document change — must be
    // served from the cache (blit count increments) and land shifted
    // by the accumulated pan.
    assert!(host.apply_pan_gesture(350.0, 150.0, 30.0, 0.0, W as f32, H as f32));
    let hot2 = paint_frame(&mut host, &mut backend);
    let x2 = leftmost_red_x(&hot2, y_probe).expect("red rect visible on second hot frame");
    assert_eq!(x2 - x0, 80);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(
        host.pan_cache_blits_for_test() >= 1,
        "pure-pan frame must be served from the cached layer"
    );

    // Gesture cools: the full-quality path paints the identical
    // geometry, and the layer STAYS resident so the next gesture
    // reuses it without paying the expanded rebuild again.
    host.set_now_ms(10_000);
    let cooled = paint_frame(&mut host, &mut backend);
    let x3 = leftmost_red_x(&cooled, y_probe).expect("red rect visible after gesture ends");
    assert_eq!(x3, x2);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());

    // A NEW pan gesture serves straight from the retained layer:
    // blit count rises, build count does not.
    let builds = host.pan_cache_builds_for_test();
    let blits = host.pan_cache_blits_for_test();
    assert!(host.apply_pan_gesture(350.0, 150.0, 5.0, 0.0, W as f32, H as f32));
    let reused = paint_frame(&mut host, &mut backend);
    let x4 = leftmost_red_x(&reused, y_probe).expect("red rect visible on reused-cache frame");
    assert_eq!(x4, x3 + 5);
    assert_eq!(host.pan_cache_builds_for_test(), builds);
    assert_eq!(host.pan_cache_blits_for_test(), blits + 1);
}

#[test]
fn zoom_gesture_frames_never_build_the_pan_cache() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);

    // Each zoom tick invalidates any layer the frame could build, so
    // building during zoom (2× a plain frame) is pure loss — zoom
    // frames must paint direct in degrade mode.
    assert!(host.apply_wheel(350.0, 150.0, -40.0, W as f32, H as f32));
    assert!(host.fast_interaction_active());
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert_eq!(host.pan_cache_builds_for_test(), 0);
    assert!(!host.pan_cache_resident_for_test());

    // A pan tick right after the zoom builds the layer once.
    assert!(host.apply_pan_gesture(350.0, 150.0, 5.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert_eq!(host.pan_cache_builds_for_test(), 1);
    assert!(host.pan_cache_resident_for_test());
}

#[test]
fn long_pan_scroll_refresh_matches_a_direct_paint_exactly() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);

    // Build the cache on the first hot frame.
    assert!(host.apply_pan_gesture(350.0, 150.0, -10.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());
    assert_eq!(host.pan_cache_scrolls_for_test(), 0);

    // Pan far past the scroll threshold (0.75 × margin = 192) in both
    // directions: each frame must scroll the layer in place and
    // repaint only the exposed strip — never a full rebuild.
    assert!(host.apply_pan_gesture(350.0, 150.0, -220.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    assert!(host.apply_pan_gesture(350.0, 150.0, 220.0, 0.0, W as f32, H as f32));
    let scrolled = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert_eq!(host.pan_cache_scrolls_for_test(), 2);
    assert!(host.pan_cache_resident_for_test());

    // The scrolled frame must be pixel-identical to a direct
    // full-quality paint at the same viewport (the fixture has no
    // effects / tiny leaves, so degrade changes nothing).
    host.set_now_ms(60_000);
    let reference = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    // The layer is retained across gestures for instant reuse.
    assert!(host.pan_cache_resident_for_test());
    assert_eq!(
        scrolled, reference,
        "scroll-refreshed frame diverged from a direct paint"
    );
}

#[test]
fn zoom_ticks_serve_scaled_blits_from_the_retained_layer() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);

    // A pan builds the layer once.
    assert!(host.apply_pan_gesture(350.0, 150.0, -10.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert_eq!(host.pan_cache_builds_for_test(), 1);
    let blits = host.pan_cache_blits_for_test();

    // A zoom tick then serves an approximate SCALED blit from that
    // layer instead of re-rendering the scene — and never rebuilds.
    assert!(host.apply_wheel(350.0, 150.0, -40.0, W as f32, H as f32));
    let zoomed = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert_eq!(host.pan_cache_builds_for_test(), 1);
    assert_eq!(host.pan_cache_blits_for_test(), blits + 1);
    // The scaled blit must actually change on-screen pixels vs the
    // pre-zoom frame (the red rect grows/moves with the zoom).
    let y_probe = 300;
    assert!(leftmost_red_x(&zoomed, y_probe).is_some());
}

#[test]
fn progressive_restore_converges_to_direct_paint_quality() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    // A drop shadow is degrade-only content: gesture frames skip it,
    // the progressive restore must bring it back tile by tile.
    let json = r##"{"version":"1.0.0","children":[{"type":"rectangle","id":"r1","x":200,"y":200,"width":120,"height":120,"fill":[{"type":"solid","color":"#ff0000"}],"effects":[{"type":"shadow","offsetX":12,"offsetY":12,"blur":24,"spread":0,"color":"#000000cc"}]}]}"##;
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    let mut state = op_editor_core::EditorState::from_document(doc);
    state.chat.minimize();
    *host.editor_state_mut() = state;
    host.mark_paint_dirty_for_test();
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);

    // Build the (degraded, shadow-less) layer during a pan.
    assert!(host.apply_pan_gesture(350.0, 150.0, -10.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());
    assert!(!host.pan_cache_sharp_for_test());

    // Gesture ends: each frame restores one tile; the layer must reach
    // `sharp` within the tile budget without a single full repaint.
    host.set_now_ms(10_000);
    let mut last = Vec::new();
    for _ in 0..=super::canvas_pan_cache::PAN_CACHE_RESTORE_TILES {
        last = paint_frame(&mut host, &mut backend);
        if host.pan_cache_sharp_for_test() {
            break;
        }
    }
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_sharp_for_test());
    assert!(!host.pan_cache_restore_active_for_test());

    // The restored blit must be pixel-identical to a direct paint.
    host.mark_paint_dirty_for_test();
    let reference = paint_frame(&mut host, &mut backend);
    assert_eq!(
        last, reference,
        "progressively restored frame diverged from a direct paint"
    );
}

/// Image decodes finish on a worker, install pixels straight into the
/// backend's raster cache, and mutate no editor state — so no
/// `mark_dirty` fires. A restored (sharp) layer must still notice, or
/// it serves the blur-up placeholder until something unrelated
/// happens to invalidate it.
#[test]
fn a_landed_image_decode_reopens_the_progressive_restore() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);
    assert!(host.apply_pan_gesture(350.0, 150.0, -10.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());

    // Gesture ends: let the progressive restore finish.
    host.set_now_ms(10_000);
    for _ in 0..super::canvas_pan_cache::PAN_CACHE_RESTORE_TILES + 4 {
        let _ = paint_frame(&mut host, &mut backend);
        if host.pan_cache_sharp_for_test() && !host.pan_cache_restore_active_for_test() {
            break;
        }
    }
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_sharp_for_test());

    // A worker install lands: the layer's baked pixels are now stale.
    let mut decoded =
        skia_safe::surfaces::raster_n32_premul((4, 4)).expect("raster surface allocated");
    decoded.canvas().clear(skia_safe::Color::BLUE);
    backend.install_raster_image(1, decoded.image_snapshot(), 4);

    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(
        host.pan_cache_restore_active_for_test() || !host.pan_cache_sharp_for_test(),
        "a landed decode must reopen the restore instead of blitting stale pixels"
    );

    // And it converges again rather than restoring on every frame.
    for _ in 0..super::canvas_pan_cache::PAN_CACHE_RESTORE_TILES + 4 {
        let _ = paint_frame(&mut host, &mut backend);
        if host.pan_cache_sharp_for_test() && !host.pan_cache_restore_active_for_test() {
            break;
        }
    }
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_sharp_for_test());
}

#[test]
fn document_mutation_invalidates_the_pan_cache() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);
    assert!(host.apply_pan_gesture(350.0, 150.0, 20.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());

    // Any editor-state mutation drops the cache: the next frame must
    // repaint the scene rather than blit stale pixels.
    host.editor_state_mut().viewport.zoom = 2.0;
    host.mark_paint_dirty_for_test();
    assert!(!host.pan_cache_resident_for_test());
}

#[cfg(feature = "collab-host")]
#[test]
fn collaboration_dirty_invalidates_the_pan_cache() {
    let _indicator_guard = crate::agent_indicator_test_support::read();
    let fonts0 = jian_skia::font_generation();
    let mut host = WidgetHostNative::new();
    seed_red_rect(&mut host);
    let mut backend = NativeBackend::with_dpi(1.0);

    host.set_now_ms(1_000);
    let _ = paint_frame(&mut host, &mut backend);
    assert!(host.apply_pan_gesture(350.0, 150.0, 20.0, 0.0, W as f32, H as f32));
    let _ = paint_frame(&mut host, &mut backend);
    if !fonts_stable_since(fonts0) {
        return;
    }
    assert!(host.pan_cache_resident_for_test());

    host.mark_editor_state_dirty();
    assert!(
        host.pan_cache_resident_for_test(),
        "the general external dirty seam must keep its non-canvas cache policy"
    );
    op_collab_host::CollabHost::mark_editor_state_dirty(&mut host);

    assert!(
        !host.pan_cache_resident_for_test(),
        "presence/participant projection must not leave a stale canvas layer"
    );
}
