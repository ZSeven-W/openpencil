//! Raster / SVG / PDF export tests — split out of `export.rs` to
//! keep that file under the 800-line cap. Shared scene-builder
//! helpers live in the sibling `export::test_support` module.

use super::test_support::{filled_rect, scene_with};
use super::*;
use op_editor_ui::layout_scene::{NodeKind, SceneNode, SceneStroke};
use op_editor_ui::{Color, Rect};

#[test]
fn raster_format_extension_lookup() {
    assert_eq!(RasterFormat::from_extension("png"), Some(RasterFormat::Png));
    assert_eq!(
        RasterFormat::from_extension("jpg"),
        Some(RasterFormat::Jpeg)
    );
    assert_eq!(
        RasterFormat::from_extension("jpeg"),
        Some(RasterFormat::Jpeg)
    );
    assert_eq!(
        RasterFormat::from_extension("webp"),
        Some(RasterFormat::Webp)
    );
    assert_eq!(RasterFormat::from_extension("svg"), None);
    assert_eq!(RasterFormat::from_extension("gif"), None);
    assert_eq!(RasterFormat::from_extension(""), None);
}

#[test]
fn raster_format_jpeg_does_not_support_alpha() {
    assert!(RasterFormat::Png.supports_alpha());
    assert!(RasterFormat::Webp.supports_alpha());
    assert!(!RasterFormat::Jpeg.supports_alpha());
}

#[test]
fn raster_format_quality_matches_ts() {
    // TS export-section.tsx: quality = 100 for PNG, 92 for JPEG/WEBP.
    assert_eq!(RasterFormat::Png.quality(), 100);
    assert_eq!(RasterFormat::Jpeg.quality(), 92);
    assert_eq!(RasterFormat::Webp.quality(), 92);
}

#[test]
fn export_raster_writes_png_for_minimal_scene() {
    let scene = scene_with(vec![filled_rect(
        "n10",
        0.0,
        0.0,
        100.0,
        50.0,
        Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    )]);
    let tmp = std::env::temp_dir().join(format!("op-export-test-{}.png", std::process::id()));
    let res = export_raster(&scene, &tmp, RasterFormat::Png, 2.0);
    assert!(res.is_ok(), "export_raster PNG failed: {res:?}");
    let bytes = std::fs::read(&tmp).unwrap();
    // PNG signature: 89 50 4E 47 0D 0A 1A 0A
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_raster_writes_jpeg_with_white_background() {
    let scene = scene_with(vec![filled_rect(
        "n10",
        0.0,
        0.0,
        80.0,
        40.0,
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    )]);
    let tmp = std::env::temp_dir().join(format!("op-export-test-{}.jpg", std::process::id()));
    let res = export_raster(&scene, &tmp, RasterFormat::Jpeg, 1.0);
    assert!(res.is_ok(), "export_raster JPEG failed: {res:?}");
    let bytes = std::fs::read(&tmp).unwrap();
    // JPEG SOI marker: FF D8 FF
    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_raster_scale_clamps_extreme_values() {
    let scene = scene_with(vec![filled_rect(
        "n10",
        0.0,
        0.0,
        10.0,
        10.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    )]);
    // Both extremes should succeed (clamped silently) rather than
    // allocating a gigapixel surface or zero-sized output.
    let tmp = std::env::temp_dir().join(format!("op-export-clamp-{}.png", std::process::id()));
    assert!(export_raster(&scene, &tmp, RasterFormat::Png, 0.001).is_ok());
    assert!(export_raster(&scene, &tmp, RasterFormat::Png, 1000.0).is_ok());
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_raster_fails_on_empty_scene() {
    let scene = scene_with(Vec::new());
    let tmp = std::env::temp_dir().join(format!("op-export-empty-{}.png", std::process::id()));
    let res = export_raster(&scene, &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_err(), "expected Err on empty scene, got {res:?}");
    assert_eq!(res.unwrap_err(), "nothing to export");
}

#[test]
fn export_raster_applies_flex_layout_from_editor_state() {
    // A vertical flex frame with a `fill_container`-width child:
    // the child's authored width is the collapsed flex token, so
    // the resolved width (375 px = the root frame width) only
    // appears after jian's flex pass. Export must render the
    // RESOLVED geometry — proven here by `page_bounds` over the
    // built `LayoutScene` covering the full 375 px root width.
    let src = r##"{
      "version":"1.0.0",
      "pages":[{
        "id":"p1","name":"Page 1",
        "children":[{
          "type":"frame","id":"root","width":375,"height":200,
          "layout":"vertical","gap":16,
          "children":[
            {"type":"rectangle","id":"r1","width":"fill_container","height":40,
             "fill":[{"type":"solid","color":"#3366FF"}]}
          ]
        }]
      }],
      "children":[]
    }"##;
    let parsed = jian_ops_schema::load_str(src).expect("parse .op fixture");
    let state = op_editor_core::EditorState::from_document(parsed.value);
    let scene = op_pen_loader::editor_state_to_layout_scene(&state);
    // Flex stretched the child to the 375 px root width.
    let child = &scene.pages[0].children[0].children[0];
    assert_eq!(child.id, "r1");
    assert_eq!(
        child.bounds.size.x, 375.0,
        "fill_container stretched via taffy"
    );
    // page_bounds covers the resolved 375 px-wide root.
    let b = page_bounds(scene.active_page().unwrap()).expect("paintable bounds");
    assert_eq!(b.size.x, 375.0, "page bounds reflect resolved layout width");
    // And the export succeeds against the layout-resolved scene.
    let tmp = std::env::temp_dir().join(format!("op-export-flex-{}.png", std::process::id()));
    let res = export_raster(&scene, &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "flex export failed: {res:?}");
    let bytes = std::fs::read(&tmp).unwrap();
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn page_bounds_covers_layout_resolved_child_geometry() {
    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::Rect;
    // A frame at (10,10) 200x100 with a child the layout pass
    // resolved to the frame's full width — page_bounds must cover
    // the resolved child bounds, not authored coords.
    let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(10.0, 10.0, 200.0, 100.0);
    frame.fill = Some(Color {
        r: 0.9,
        g: 0.9,
        b: 0.9,
        a: 1.0,
    });
    let mut child = filled_rect(
        "child",
        10.0,
        10.0,
        200.0,
        40.0,
        Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        },
    );
    child.bounds = Rect::xywh(10.0, 10.0, 200.0, 40.0);
    frame.children = vec![child];
    let scene = scene_with(vec![frame]);
    let page = scene.active_page().unwrap();
    let b = page_bounds(page).expect("page has paintable bounds");
    assert_eq!(b.origin.x, 10.0);
    assert_eq!(b.origin.y, 10.0);
    assert_eq!(b.size.x, 200.0);
    assert_eq!(b.size.y, 100.0);
}

#[test]
fn export_node_raster_crops_to_the_named_node() {
    // Two side-by-side rects: a 100×50 at origin and a 40×40 far
    // away. Exporting only the small node must produce a surface
    // cropped to ITS bounds, not the page union.
    let grey = Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };
    let scene = scene_with(vec![
        filled_rect("big", 0.0, 0.0, 100.0, 50.0, grey),
        filled_rect("small", 400.0, 400.0, 40.0, 40.0, grey),
    ]);
    let tmp = std::env::temp_dir().join(format!("op-export-node-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "small", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "export_node_raster failed: {res:?}");
    let bytes = std::fs::read(&tmp).unwrap();
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    );
    // The cropped surface is the 40×40 node + 2×MARGIN, far
    // smaller than the ~440 px page union the whole-page export
    // would have produced. PNG IHDR carries the dimensions as
    // big-endian u32s at byte offsets 16 (width) and 20 (height).
    let png_width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let png_height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let expected = (40.0 + MARGIN * 2.0) as u32;
    assert_eq!(png_width, expected);
    assert_eq!(png_height, expected);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_node_raster_paints_icon_font_glyphs() {
    let mut icon = SceneNode::leaf("home-icon", NodeKind::Other("icon_font".into()));
    icon.bounds = Rect::xywh(0.0, 0.0, 32.0, 32.0);
    icon.text = Some("home".into());
    icon.font_family = "lucide".into();
    icon.fill = Some(Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    let scene = scene_with(vec![icon]);
    let tmp = std::env::temp_dir().join(format!("op-export-icon-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "home-icon", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "export_node_raster icon failed: {res:?}");

    let bytes = std::fs::read(&tmp).unwrap();
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes))
        .expect("decode exported icon PNG");
    let width = image.width();
    let height = image.height();
    let info = skia_safe::ImageInfo::new(
        (width, height),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let stride = width as usize * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    let ok = image.read_pixels(
        &info,
        pixels.as_mut_slice(),
        stride,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    assert!(ok, "read exported icon pixels");
    let painted = pixels.chunks_exact(4).filter(|rgba| rgba[3] > 0).count();
    assert!(
        painted > 20,
        "expected icon export to contain painted glyph pixels, got {painted}"
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_node_raster_paints_svg_path_d_stroke() {
    let mut path = SceneNode::leaf("activity-line", NodeKind::Path);
    path.bounds = Rect::xywh(0.0, 0.0, 120.0, 60.0);
    path.svg_path = Some("M 0,50 L 40,20 L 80,35 L 120,5".into());
    path.stroke = Some(SceneStroke {
        color: Color {
            r: 0.0,
            g: 0.2,
            b: 1.0,
            a: 1.0,
        },
        width: 3.0,
        sides: None,
    });

    let scene = scene_with(vec![path]);
    let tmp = std::env::temp_dir().join(format!(
        "op-export-svg-path-stroke-{}.png",
        std::process::id()
    ));
    let res = export_node_raster(&scene, "activity-line", &tmp, RasterFormat::Png, 1.0);
    assert!(
        res.is_ok(),
        "export_node_raster svg path stroke failed: {res:?}"
    );

    let bytes = std::fs::read(&tmp).unwrap();
    let painted = visible_pixel_count(&bytes);
    assert!(
        painted > 20,
        "expected svg path stroke export to contain painted pixels, got {painted}"
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_raster_paints_only_authored_stroke_sides() {
    let mut node = SceneNode::leaf("bottom-border", NodeKind::Frame);
    node.bounds = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    node.stroke = Some(SceneStroke {
        color: Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
        width: 4.0,
        sides: Some([0.0, 0.0, 4.0, 0.0]),
    });
    let scene = scene_with(vec![node]);
    let tmp = std::env::temp_dir().join(format!("op-export-sided-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "bottom-border", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "sided-stroke export failed: {res:?}");
    let decoded = decode_rgba(&std::fs::read(&tmp).unwrap());

    // Bounds include a 2 px stroke pad, so doc (x,y) maps to
    // `(MARGIN + 2 + x, MARGIN + 2 + y)`.
    let top = pixel_at(&decoded, MARGIN as i32 + 22, MARGIN as i32 + 2);
    assert_eq!(top[3], 0, "top edge must remain transparent, got {top:?}");
    let left = pixel_at(&decoded, MARGIN as i32 + 2, MARGIN as i32 + 12);
    assert_eq!(
        left[3], 0,
        "left edge must remain transparent, got {left:?}"
    );
    let bottom = pixel_at(&decoded, MARGIN as i32 + 22, MARGIN as i32 + 20);
    assert!(
        bottom[3] > 200,
        "bottom edge should paint the authored stroke, got {bottom:?}"
    );
    let below = pixel_at(&decoded, MARGIN as i32 + 22, MARGIN as i32 + 23);
    assert_eq!(
        below[3], 0,
        "sided strokes should stay inside the authored bounds, got {below:?}"
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn export_node_raster_errors_on_unknown_id() {
    let scene = scene_with(vec![filled_rect(
        "n10",
        0.0,
        0.0,
        10.0,
        10.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    )]);
    let tmp = std::env::temp_dir().join(format!("op-export-node-miss-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "ghost", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_err(), "expected Err on unknown id, got {res:?}");
    assert!(res.unwrap_err().contains("not found"));
}

fn visible_pixel_count(bytes: &[u8]) -> usize {
    let (_, _, pixels) = decode_rgba(bytes);
    pixels.chunks_exact(4).filter(|rgba| rgba[3] > 0).count()
}

/// Decode an exported PNG into `(width, height, unpremul RGBA bytes)`.
fn decode_rgba(bytes: &[u8]) -> (i32, i32, Vec<u8>) {
    let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(bytes))
        .expect("decode exported PNG");
    let width = image.width();
    let height = image.height();
    let info = skia_safe::ImageInfo::new(
        (width, height),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let stride = width as usize * 4;
    let mut pixels = vec![0u8; stride * height as usize];
    let ok = image.read_pixels(
        &info,
        pixels.as_mut_slice(),
        stride,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    assert!(ok, "read exported PNG pixels");
    (width, height, pixels)
}

/// One RGBA pixel at `(x, y)` from a `decode_rgba` result.
fn pixel_at(decoded: &(i32, i32, Vec<u8>), x: i32, y: i32) -> [u8; 4] {
    let (w, h, pixels) = decoded;
    assert!(x < *w && y < *h, "probe ({x},{y}) outside {w}x{h}");
    let i = (y as usize * *w as usize + x as usize) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Encode a `w`×`h` solid-colour PNG and wrap it as a `data:` URL —
/// fixture bytes generated at test time so they can't rot.
fn solid_png_data_url(w: i32, h: i32, color: skia_safe::Color) -> String {
    use base64::Engine as _;
    let mut surface = skia_safe::surfaces::raster_n32_premul((w, h)).expect("fixture surface");
    surface.canvas().clear(color);
    let image = surface.image_snapshot();
    let data = image
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .expect("encode fixture PNG");
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(data.as_bytes())
    )
}

/// BLOCK fix probe: an image node (loader shape — `kind=rect` +
/// `image_src` + grey placeholder fill, `adapter.rs::image_to_payload`)
/// must export the decoded bitmap, not the placeholder rect. Routed
/// through the shared canvas painter (`canvas_viewport_image.rs`), so
/// data-URL decode + shared byte cache + fit semantics all apply.
#[test]
fn export_renders_data_url_image_bitmaps() {
    let mut node = SceneNode::leaf("img", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 20.0, 20.0);
    node.image_src = Some(solid_png_data_url(4, 4, skia_safe::Color::RED));
    // Loader placeholder fill — must NOT win over the bitmap.
    node.fill = Some(Color {
        r: 0.85,
        g: 0.86,
        b: 0.88,
        a: 1.0,
    });
    let scene = scene_with(vec![node]);
    let tmp = std::env::temp_dir().join(format!("op-export-img-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "img", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "image export failed: {res:?}");
    let decoded = decode_rgba(&std::fs::read(&tmp).unwrap());
    // Image centre = node centre offset by the export MARGIN.
    let c = pixel_at(&decoded, MARGIN as i32 + 10, MARGIN as i32 + 10);
    assert!(
        c[0] > 200 && c[1] < 60 && c[2] < 60 && c[3] > 200,
        "expected the red bitmap at the node centre, got {c:?}"
    );
    let _ = std::fs::remove_file(&tmp);
}

/// A remote (`https`) source whose bytes were never fetched paints
/// the SAME placeholder the canvas shows (grey fill + dashed border +
/// picture glyph) instead of a bare rect.
#[test]
fn export_remote_image_without_bytes_paints_the_canvas_placeholder() {
    let mut node = SceneNode::leaf("remote-img", NodeKind::Rect);
    node.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
    node.image_src = Some("https://example.invalid/op-export-remote-test.png".into());
    node.fill = Some(Color {
        r: 0.85,
        g: 0.86,
        b: 0.88,
        a: 1.0,
    });
    let scene = scene_with(vec![node]);
    let tmp = std::env::temp_dir().join(format!("op-export-remote-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "remote-img", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "remote-image export failed: {res:?}");
    let decoded = decode_rgba(&std::fs::read(&tmp).unwrap());
    // The placeholder fill shows where neither the dashed border nor
    // the centred picture glyph paints. The glyph is 16×16 centred
    // (min(40,40)×0.4, clamped ≥12 → doc 12..28), so probe doc (6,6):
    // inside the fill, clear of the glyph box and the 1 px dashed
    // perimeter.
    let c = pixel_at(&decoded, MARGIN as i32 + 6, MARGIN as i32 + 6);
    assert!(
        c[0] > 190 && c[1] > 190 && c[2] > 190 && c[3] > 200,
        "expected the grey placeholder fill, got {c:?}"
    );
    // …and the dashed-border + picture-glyph art paints on top.
    assert!(visible_pixel_count(&std::fs::read(&tmp).unwrap()) > 100);
    let _ = std::fs::remove_file(&tmp);
}

/// `clip_content` containers trim child overflow exactly like the
/// canvas (`push_clip_content`): the surface is sized to the clipped
/// container rect AND no overflowing child pixel paints outside it.
#[test]
fn export_clip_content_trims_child_overflow() {
    let blue = Color {
        r: 0.0,
        g: 0.2,
        b: 1.0,
        a: 1.0,
    };
    let mut frame = SceneNode::leaf("clipper", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 40.0, 40.0);
    frame.clip_content = true;
    // Child overflows the frame by 40 px vertically.
    frame.children = vec![filled_rect("ovf", 0.0, 0.0, 40.0, 80.0, blue)];
    let scene = scene_with(vec![frame]);
    let tmp = std::env::temp_dir().join(format!("op-export-clip-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "clipper", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "clip export failed: {res:?}");
    let bytes = std::fs::read(&tmp).unwrap();
    let decoded = decode_rgba(&bytes);
    // Surface sized to the clipped frame (40 + 2×MARGIN), not the
    // un-clipped 80 px child union.
    let expected = (40.0 + MARGIN * 2.0) as i32;
    assert_eq!((decoded.0, decoded.1), (expected, expected));
    // Inside the frame the child paints…
    let inside = pixel_at(&decoded, MARGIN as i32 + 20, MARGIN as i32 + 20);
    assert!(
        inside[2] > 200 && inside[3] > 200,
        "expected the blue child inside the frame, got {inside:?}"
    );
    // …below the frame (doc y > 40, inside the margin band) the
    // overflow is clipped away — transparent, like the canvas.
    let below = pixel_at(&decoded, MARGIN as i32 + 20, MARGIN as i32 + 44);
    assert_eq!(below[3], 0, "overflow must be clipped, got {below:?}");
    let _ = std::fs::remove_file(&tmp);
}

/// Output-size caps: per-side and total-pixel ceilings return a
/// structured error instead of allocating a giant surface. The MCP
/// screenshot path wraps this in "Renderer reported failure: …".
#[test]
fn render_raster_bytes_rejects_oversized_outputs() {
    // Per-side cap (width).
    let err = render_raster_bytes(
        Rect::xywh(0.0, 0.0, 20_000.0, 10.0),
        RasterFormat::Png,
        1.0,
        0.0,
        |_| {},
    )
    .unwrap_err();
    assert!(err.contains("exceeds the size cap"), "{err}");
    // Total-pixel cap with both sides under the per-side cap:
    // 10 000 × 10 000 = 100 MPx > 64 MPx.
    let err = render_raster_bytes(
        Rect::xywh(0.0, 0.0, 10_000.0, 10_000.0),
        RasterFormat::Png,
        1.0,
        0.0,
        |_| {},
    )
    .unwrap_err();
    assert!(err.contains("exceeds the size cap"), "{err}");
    // Sanity: a normal surface still renders.
    assert!(render_raster_bytes(
        Rect::xywh(0.0, 0.0, 64.0, 64.0),
        RasterFormat::Png,
        1.0,
        0.0,
        |_| {},
    )
    .is_ok());
}

/// Styled-run smoke: the shared text painter (canvas_viewport_text)
/// drives export text. "HELLO" has no descenders, so painted pixels in
/// the underline band below the baseline can only come from the styled
/// run's underline decoration — a path the old mirror painter (plain
/// `draw_str` loop) did not have at all.
#[test]
fn export_paints_styled_text_runs_with_decorations() {
    use op_editor_ui::layout_scene::SceneTextRun;
    let mut node = SceneNode::leaf("styled", NodeKind::Text);
    node.bounds = Rect::xywh(0.0, 0.0, 120.0, 30.0);
    node.text = Some("HELLO".into());
    node.font_size = 20.0;
    node.fill = Some(Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    node.text_runs = vec![SceneTextRun {
        start: 0,
        end: 5,
        font_size: 0.0,
        font_weight: 700,
        fill: None,
        italic: false,
        underline: true,
        strikethrough: false,
    }];
    let scene = scene_with(vec![node]);
    let tmp = std::env::temp_dir().join(format!("op-export-styled-{}.png", std::process::id()));
    let res = export_node_raster(&scene, "styled", &tmp, RasterFormat::Png, 1.0);
    assert!(res.is_ok(), "styled-text export failed: {res:?}");
    let decoded = decode_rgba(&std::fs::read(&tmp).unwrap());
    // Underline strokes at baseline (top + font_size = 20) + 0.12 ×
    // 20 ≈ doc y 22.4 → px rows ~38..40 after the MARGIN offset.
    let band_y0 = MARGIN as i32 + 22;
    let mut painted_in_band = 0;
    for y in band_y0..(band_y0 + 3) {
        for x in MARGIN as i32..(MARGIN as i32 + 90) {
            if pixel_at(&decoded, x, y)[3] > 0 {
                painted_in_band += 1;
            }
        }
    }
    assert!(
        painted_in_band > 20,
        "expected the styled run's underline below the baseline, found {painted_in_band} px"
    );
    let _ = std::fs::remove_file(&tmp);
}
