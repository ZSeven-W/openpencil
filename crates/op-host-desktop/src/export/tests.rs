//! Raster / SVG / PDF export tests — split out of `export.rs` to
//! keep that file under the 800-line cap. Shared scene-builder
//! helpers live in the sibling `export::test_support` module.

use super::test_support::{filled_rect, scene_with};
use super::*;
use op_editor_ui::layout_scene::{NodeKind, SceneNode, SceneStroke};
use op_editor_ui::Rect;

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
    pixels.chunks_exact(4).filter(|rgba| rgba[3] > 0).count()
}
