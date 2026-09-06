//! DOM video overlays for CanvasKit preview.
//!
//! CanvasKit owns the painted poster and all editor chrome. Preview playback
//! is a sibling DOM layer: each visible video node becomes one absolutely
//! positioned `<video>` element whose screen rect is refreshed after every
//! repaint. That keeps browser media decoding out of Rust/CanvasKit and makes
//! pan, zoom, resize, and device-frame presentation use the same frame as the
//! painted scene.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use op_editor_ui::{Point2D, Rect};
use op_preview_core::PreviewVideoOverlay;
use wasm_bindgen::JsCast;
use web_sys::{DomRect, HtmlCanvasElement, HtmlVideoElement, MouseEvent};

use crate::listener::{add_listener, Listener};

/// A preview video source plus its already-mapped logical screen rectangle.
#[derive(Debug, Clone)]
pub(crate) struct VideoOverlayPlacement {
    pub(crate) node_id: String,
    pub(crate) screen_rect: Rect,
    pub(crate) video: PreviewVideoOverlay,
}

/// Convert a logical CanvasKit screen rect into CSS viewport coordinates.
/// CanvasKit paints in the logical canvas size, while the DOM rect may be
/// scaled by CSS or device-pixel-ratio outside the Rust host.
pub(crate) fn css_video_rect(
    canvas_bounds: &DomRect,
    logical_w: f32,
    logical_h: f32,
    screen_rect: Rect,
) -> Rect {
    css_video_rect_from_metrics(
        canvas_bounds.left() as f32,
        canvas_bounds.top() as f32,
        canvas_bounds.width() as f32,
        canvas_bounds.height() as f32,
        logical_w,
        logical_h,
        screen_rect,
    )
}

pub(crate) fn css_video_rect_from_metrics(
    canvas_left: f32,
    canvas_top: f32,
    canvas_width: f32,
    canvas_height: f32,
    logical_w: f32,
    logical_h: f32,
    screen_rect: Rect,
) -> Rect {
    let scale_x = if logical_w > 0.0 {
        canvas_width / logical_w
    } else {
        1.0
    };
    let scale_y = if logical_h > 0.0 {
        canvas_height / logical_h
    } else {
        1.0
    };
    Rect {
        origin: Point2D::new(
            canvas_left + screen_rect.origin.x * scale_x,
            canvas_top + screen_rect.origin.y * scale_y,
        ),
        size: Point2D::new(screen_rect.size.x * scale_x, screen_rect.size.y * scale_y),
    }
}

/// Map a scene-space node rect through the same affine transform used by the
/// CanvasKit painter. This is kept pure so overlay geometry can be tested
/// without constructing browser DOM objects.
pub(crate) fn scene_video_rect(scene_rect: Rect, viewport_origin: Point2D, zoom: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            viewport_origin.x + scene_rect.origin.x * zoom,
            viewport_origin.y + scene_rect.origin.y * zoom,
        ),
        size: Point2D::new(scene_rect.size.x * zoom, scene_rect.size.y * zoom),
    }
}

struct VideoElementState {
    element: HtmlVideoElement,
    src: String,
    poster: Option<String>,
    autoplay: bool,
    loop_video: bool,
    muted: bool,
    click_to_replay: Rc<Cell<bool>>,
}

/// Retained DOM elements and event closures for one CanvasKit mount.
pub(crate) struct VideoOverlayLayer {
    document: web_sys::Document,
    elements: HashMap<String, VideoElementState>,
    listeners: Vec<Listener>,
}

impl VideoOverlayLayer {
    pub(crate) fn create(canvas: &HtmlCanvasElement) -> Option<Self> {
        let document = canvas
            .owner_document()
            .or_else(|| web_sys::window().and_then(|window| window.document()))?;
        Some(Self {
            document,
            elements: HashMap::new(),
            listeners: Vec::new(),
        })
    }

    /// Reconcile the DOM layer with the current preview placements. Empty
    /// video sources are omitted so the image poster remains visible while a
    /// newly-added video is still being configured in the property panel.
    pub(crate) fn sync(
        &mut self,
        canvas: &HtmlCanvasElement,
        logical_w: f32,
        logical_h: f32,
        placements: &[VideoOverlayPlacement],
    ) {
        let canvas_bounds = canvas.get_bounding_client_rect();
        let mut seen = HashSet::new();
        for placement in placements {
            if placement.video.video.src.trim().is_empty() || !valid_rect(placement.screen_rect) {
                continue;
            }
            let node_id = placement.node_id.as_str();
            let css_rect =
                css_video_rect(&canvas_bounds, logical_w, logical_h, placement.screen_rect);
            let Some(state) = self.ensure_element(node_id) else {
                continue;
            };
            sync_video_attributes(state, &placement.video);
            let pointer_events = if state.click_to_replay.get() {
                "auto"
            } else {
                "none"
            };
            let _ = state.element.set_attribute(
                "style",
                &format!(
                    "position:fixed;left:{}px;top:{}px;width:{}px;height:{}px;object-fit:cover;border:0;padding:0;margin:0;z-index:1;pointer-events:{};",
                    css_rect.origin.x,
                    css_rect.origin.y,
                    css_rect.size.x,
                    css_rect.size.y,
                    pointer_events,
                ),
            );
            seen.insert(node_id.to_owned());
        }

        let stale: Vec<String> = self
            .elements
            .keys()
            .filter(|id| !seen.contains(*id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(state) = self.elements.remove(&id) {
                if let Some(parent) = state.element.parent_node() {
                    let _ = parent.remove_child(&state.element);
                }
            }
        }
    }

    fn ensure_element(&mut self, node_id: &str) -> Option<&mut VideoElementState> {
        if !self.elements.contains_key(node_id) {
            let element = self
                .document
                .create_element("video")
                .ok()?
                .dyn_into::<HtmlVideoElement>()
                .ok()?;
            let _ = element.set_attribute("playsinline", "");
            let _ = element.set_attribute("preload", "metadata");
            let _ = element.set_attribute("aria-hidden", "true");
            element.set_controls(false);

            let click_to_replay = Rc::new(Cell::new(false));
            let enabled = click_to_replay.clone();
            let replay_video = element.clone();
            let _ = add_listener::<MouseEvent, _, _>(
                &element,
                "click",
                &mut self.listeners,
                move |_event| {
                    if enabled.get() {
                        replay_video.set_current_time(0.0);
                        let _ = replay_video.play();
                    }
                },
            );
            if let Some(body) = self.document.body() {
                let _ = body.append_child(&element);
            }
            self.elements.insert(
                node_id.to_owned(),
                VideoElementState {
                    element,
                    src: String::new(),
                    poster: None,
                    autoplay: false,
                    loop_video: false,
                    muted: false,
                    click_to_replay,
                },
            );
        }
        self.elements.get_mut(node_id)
    }
}

fn sync_video_attributes(state: &mut VideoElementState, overlay: &PreviewVideoOverlay) {
    let video = &overlay.video;
    let src = video.src.to_string();
    if state.src != src {
        state.element.set_src(&src);
        let _ = state.element.set_attribute("src", &src);
        state.src = src;
    }
    let poster = overlay.poster.as_deref().map(str::to_owned);
    if state.poster != poster {
        match poster.as_deref() {
            Some(poster) if !poster.is_empty() => {
                state.element.set_poster(poster);
                let _ = state.element.set_attribute("poster", poster);
            }
            _ => {
                let _ = state.element.remove_attribute("poster");
            }
        }
        state.poster = poster;
    }
    let autoplay = video.autoplay;
    let loop_video = video.r#loop && !video.hold_last_frame;
    let muted = video.muted || autoplay;
    if state.autoplay != autoplay {
        state.element.set_autoplay(autoplay);
        set_presence_attribute(&state.element, "autoplay", autoplay);
        state.autoplay = autoplay;
    }
    if state.loop_video != loop_video {
        state.element.set_loop(loop_video);
        set_presence_attribute(&state.element, "loop", loop_video);
        state.loop_video = loop_video;
    }
    if state.muted != muted {
        state.element.set_muted(muted);
        set_presence_attribute(&state.element, "muted", muted);
        state.muted = muted;
    }
    state.click_to_replay.set(video.click_to_replay);
}

fn set_presence_attribute(element: &HtmlVideoElement, name: &str, present: bool) {
    if present {
        let _ = element.set_attribute(name, "");
    } else {
        let _ = element.remove_attribute(name);
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.origin.x.is_finite()
        && rect.origin.y.is_finite()
        && rect.size.x.is_finite()
        && rect.size.y.is_finite()
        && rect.size.x > 0.0
        && rect.size.y > 0.0
}

#[cfg(test)]
mod tests {
    use super::{css_video_rect_from_metrics, scene_video_rect};
    use op_editor_ui::{Point2D, Rect};

    #[test]
    fn css_rect_maps_logical_canvas_coordinates() {
        let rect = css_video_rect_from_metrics(
            100.0,
            50.0,
            800.0,
            600.0,
            400.0,
            300.0,
            Rect {
                origin: Point2D::new(20.0, 30.0),
                size: Point2D::new(100.0, 80.0),
            },
        );
        assert_eq!(rect.origin, Point2D::new(140.0, 110.0));
        assert_eq!(rect.size, Point2D::new(200.0, 160.0));
    }

    #[test]
    fn scene_rect_uses_the_canvas_transform() {
        let rect = scene_video_rect(
            Rect::xywh(20.0, 30.0, 100.0, 80.0),
            Point2D::new(40.0, 50.0),
            1.5,
        );
        assert_eq!(rect.origin, Point2D::new(70.0, 95.0));
        assert_eq!(rect.size, Point2D::new(150.0, 120.0));
    }
}
