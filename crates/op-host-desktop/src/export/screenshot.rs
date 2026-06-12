//! `debug_screenshot` raster capture for the live-GUI MCP server.
//!
//! Renders the active page (or one node + subtree) of the live
//! `EditorState` through the SAME raster pipeline file exports use
//! (`render_raster_bytes` + `paint_node`), encodes PNG, and returns
//! base64 for the MCP image content block — mirroring the TS live
//! path (`use-mcp-sync.ts` → `SkiaEngine.captureRegion` → pngBase64).
//!
//! Runs on the UI thread (the `mcp_live` pump dispatches here) so the
//! layout-scene derive + skia raster never race the editor's own
//! skia usage.

use base64::Engine as _;
#[cfg(feature = "mcp-debug-tools")]
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_editor_ui::Rect;

use super::{collect_bounds, page_bounds, paint_node, paint_nodes, BoundsAcc, RasterFormat};

/// Capture parameters — decoupled from `op_mcp::ScreenshotRequest` so
/// this module compiles (and tests) without the `mcp-debug-tools`
/// feature; the feature-gated live glue converts between the two.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CaptureSpec {
    /// `None` captures the whole active-page content (TS target=root);
    /// `Some(id)` captures one node + subtree (TS target=node).
    pub node_id: Option<String>,
    /// Extra doc-px around the captured bounds (TS `opts.padding`,
    /// default 0).
    pub padding: f32,
    /// Render scale (TS `opts.dpr`, default 1 headlessly). Clamped to
    /// the export pipeline's 0.5..=8 range.
    pub scale: f32,
}

/// One captured screenshot — PNG bytes (base64) + the doc-space
/// content bounds that were rendered. `actual_bounds` is `None` for
/// whole-page captures, mirroring TS (`actualBounds` is undefined for
/// `bounds === 'root'`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScreenshotPng {
    pub png_base64: String,
    pub actual_bounds: Option<(f32, f32, f32, f32)>,
}

/// Render the screenshot off the live editor state. Derives a fresh
/// layout-resolved scene (same derive the canvas painter uses) so the
/// pixels match what the user sees at zoom 1. The only caller is the
/// feature-gated `mcp_live` UI pump, so gate it identically to keep
/// the default build warning-free.
#[cfg(feature = "mcp-debug-tools")]
pub(crate) fn capture(state: &EditorState, spec: &CaptureSpec) -> Result<ScreenshotPng, String> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    capture_scene(&scene, spec)
}

/// Scene-level capture — split out so tests can drive a hand-built
/// scene without an `EditorState`.
pub(crate) fn capture_scene(
    scene: &LayoutScene,
    spec: &CaptureSpec,
) -> Result<ScreenshotPng, String> {
    let Some(page) = scene.active_page() else {
        return Err("no active page".into());
    };
    let scale = if spec.scale.is_finite() {
        spec.scale.clamp(0.5, 8.0)
    } else {
        1.0
    };
    let padding = if spec.padding.is_finite() {
        spec.padding.max(0.0)
    } else {
        0.0
    };
    let (actual_bounds, png) = match &spec.node_id {
        None => {
            let bounds: Rect = page_bounds(page).ok_or("nothing to capture on the active page")?;
            let png = render(bounds, scale, padding, |canvas| {
                paint_nodes(canvas, &page.children);
            })?;
            (None, png)
        }
        Some(node_id) => {
            let node = page
                .find(node_id)
                .ok_or_else(|| format!("node {node_id} not found on the active page"))?;
            let mut acc = BoundsAcc::new();
            collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
            let bounds = acc
                .into_rect()
                .ok_or_else(|| format!("node {node_id} paints nothing"))?;
            let png = render(bounds, scale, padding, |canvas| paint_node(canvas, node))?;
            (
                Some((
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.x,
                    bounds.size.y,
                )),
                png,
            )
        }
    };
    Ok(ScreenshotPng {
        png_base64: base64::engine::general_purpose::STANDARD.encode(png),
        actual_bounds,
    })
}

fn render(
    bounds: Rect,
    scale: f32,
    padding: f32,
    paint: impl FnOnce(&skia_safe::Canvas),
) -> Result<Vec<u8>, String> {
    super::render_raster_bytes(bounds, RasterFormat::Png, scale, padding, paint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::test_support::{filled_rect, scene_with};
    use op_editor_ui::Color;

    const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G'];

    fn decode(b64: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("valid base64")
    }

    #[test]
    fn root_capture_renders_the_page_to_png_without_bounds() {
        let scene = scene_with(vec![filled_rect(
            "r1",
            10.0,
            20.0,
            40.0,
            30.0,
            Color::BLACK,
        )]);
        let shot = capture_scene(
            &scene,
            &CaptureSpec {
                node_id: None,
                padding: 0.0,
                scale: 1.0,
            },
        )
        .expect("capture");
        assert!(
            shot.actual_bounds.is_none(),
            "TS root capture has no actualBounds"
        );
        let png = decode(&shot.png_base64);
        assert_eq!(&png[..4], PNG_MAGIC, "must be a PNG payload");
    }

    #[test]
    fn node_capture_reports_the_node_bounds_and_errors_on_unknown_ids() {
        let scene = scene_with(vec![
            filled_rect("r1", 0.0, 0.0, 10.0, 10.0, Color::BLACK),
            filled_rect("r2", 100.0, 50.0, 40.0, 30.0, Color::BLACK),
        ]);
        let shot = capture_scene(
            &scene,
            &CaptureSpec {
                node_id: Some("r2".into()),
                padding: 4.0,
                scale: 2.0,
            },
        )
        .expect("capture");
        assert_eq!(shot.actual_bounds, Some((100.0, 50.0, 40.0, 30.0)));
        assert_eq!(&decode(&shot.png_base64)[..4], PNG_MAGIC);

        let err = capture_scene(
            &scene,
            &CaptureSpec {
                node_id: Some("ghost".into()),
                padding: 0.0,
                scale: 1.0,
            },
        )
        .unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }

    /// Output-size cap (export::MAX_RASTER_*) reaches the screenshot
    /// path: a huge padding must produce a structured error string —
    /// `mcp_live/screenshot.rs` wraps it as "Renderer reported
    /// failure: …" — instead of allocating a giant UI-thread surface.
    #[test]
    fn oversized_padding_is_rejected_with_a_structured_error() {
        let scene = scene_with(vec![filled_rect("r1", 0.0, 0.0, 10.0, 10.0, Color::BLACK)]);
        let err = capture_scene(
            &scene,
            &CaptureSpec {
                node_id: None,
                padding: 1.0e7,
                scale: 1.0,
            },
        )
        .unwrap_err();
        assert!(err.contains("exceeds the size cap"), "{err}");
    }

    #[test]
    fn empty_page_reports_nothing_to_capture() {
        let scene = scene_with(vec![]);
        let err = capture_scene(
            &scene,
            &CaptureSpec {
                node_id: None,
                padding: 0.0,
                scale: 1.0,
            },
        )
        .unwrap_err();
        assert!(err.contains("nothing to capture"), "{err}");
    }
}
