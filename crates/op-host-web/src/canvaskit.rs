//! CanvasKit rendering path for the Rust web shell.
//!
//! Replaces the from-scratch `wasm32-unknown-unknown` skia build (skia-safe-op
//! + hand-rolled libc++/GL shim) with the official CanvasKit skia WASM artifact.
//! The Rust side owns all widget/draw logic and drives CanvasKit through the
//! thin `op_ck_bridge.js` FFI. `CanvasKitBackend` implements the same
//! `RenderBackend` (`jian_widgets::painter::Painter`) the native desktop
//! backend implements, so all shell-core UI code is shared across platforms.

use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/op_ck_bridge.js")]
extern "C" {
    /// Async init: load CanvasKit, make a WebGL surface on `canvas_id`, build
    /// the bridge object. Text uses browser/system fonts via the JS bridge.
    #[wasm_bindgen(js_name = opCkInit, catch)]
    fn op_ck_init(canvas_id: &str) -> Result<js_sys::Promise, JsValue>;

    /// The bridge object: flat scalar-arg ops over a CanvasKit canvas.
    pub type OpCk;
    #[wasm_bindgen(method, js_name = beginFrame)]
    fn begin_frame(this: &OpCk);
    #[wasm_bindgen(method, js_name = endFrame)]
    fn end_frame(this: &OpCk);
    #[wasm_bindgen(method)]
    fn clear(this: &OpCk, r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = fillRect)]
    fn fill_rect(this: &OpCk, x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = strokeRect)]
    fn stroke_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRect)]
    fn fill_round_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = strokeRoundRect)]
    fn stroke_round_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillOval)]
    fn fill_oval(this: &OpCk, x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = strokeOval)]
    fn stroke_oval(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = strokeLine)]
    fn stroke_line(
        this: &OpCk,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillPolygon)]
    fn fill_polygon(this: &OpCk, pts: &[f32], r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = strokeSvgPath)]
    fn stroke_svg_path(
        this: &OpCk,
        d: &str,
        tx: f32,
        ty: f32,
        scale: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPath)]
    fn fill_svg_path(
        this: &OpCk,
        d: &str,
        tx: f32,
        ty: f32,
        scale: f32,
        even_odd: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRect)]
    fn fill_svg_path_in_rect(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = strokeSvgPathInRect)]
    fn stroke_svg_path_in_rect(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRectLinearGradient)]
    fn fill_svg_path_in_rect_linear_gradient(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        stops: &[f32],
        angle_deg: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRectRadialGradient)]
    fn fill_svg_path_in_rect_radial_gradient(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        stops: &[f32],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillInnerShadowSvgPath)]
    fn fill_inner_shadow_svg_path(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = drawText)]
    fn draw_text(
        this: &OpCk,
        t: &str,
        x: f32,
        y: f32,
        sz: f32,
        weight: i32,
        italic: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = measureText)]
    fn measure_text(this: &OpCk, t: &str, sz: f32) -> f32;
    #[wasm_bindgen(method, js_name = registerSystemFont)]
    fn register_system_font(this: &OpCk, family: &str, bytes: &[u8]) -> bool;
    #[wasm_bindgen(method, js_name = clipRect)]
    fn clip_rect(this: &OpCk, x: f32, y: f32, w: f32, h: f32);
    #[wasm_bindgen(method, js_name = clipRoundRect)]
    fn clip_round_rect(this: &OpCk, x: f32, y: f32, w: f32, h: f32, rad: f32);
    #[wasm_bindgen(method)]
    fn save(this: &OpCk);
    #[wasm_bindgen(method)]
    fn restore(this: &OpCk);
    #[wasm_bindgen(method)]
    fn translate(this: &OpCk, x: f32, y: f32);
    #[wasm_bindgen(method)]
    fn scale(this: &OpCk, sx: f32, sy: f32);
    #[wasm_bindgen(method)]
    fn rotate(this: &OpCk, deg: f32, px: f32, py: f32);
    #[wasm_bindgen(method)]
    fn resize(this: &OpCk, w: u32, h: u32);
}

const STEADY_CANVAS_PIXEL_BUDGET: f32 = 4_000_000.0;

fn flatten_gradient_stops(stops: &[(f32, Color)]) -> Vec<f32> {
    let mut flat = Vec::with_capacity(stops.len() * 5);
    for (offset, color) in stops {
        flat.extend([*offset, color.r, color.g, color.b, color.a]);
    }
    flat
}

fn svg_path_even_odd(d: &str) -> bool {
    d.matches(['Z', 'z']).count() > 1
}

/// `RenderBackend` over CanvasKit. Paints in logical (CSS) pixels; `begin_frame`
/// applies the device-pixel-ratio so output matches the native backend.
pub struct CanvasKitBackend {
    ck: OpCk,
    dpr: f32,
    /// Logical (CSS) viewport — what widget layout uses.
    logical_w: u32,
    logical_h: u32,
}

impl CanvasKitBackend {
    pub fn new(ck: OpCk, dpr: f32, logical_w: u32, logical_h: u32) -> Self {
        Self {
            ck,
            dpr,
            logical_w,
            logical_h,
        }
    }
    pub fn logical_size(&self) -> (f32, f32) {
        (self.logical_w as f32, self.logical_h as f32)
    }
    pub fn resize_for_display(&mut self, logical_w: u32, logical_h: u32, dpr: f32) {
        self.logical_w = logical_w.max(1);
        self.logical_h = logical_h.max(1);
        self.dpr = dpr.max(1.0);
        let pw = ((self.logical_w as f32) * self.dpr).round() as u32;
        let ph = ((self.logical_h as f32) * self.dpr).round() as u32;
        self.ck.resize(pw.max(1), ph.max(1));
    }
}

impl RenderBackend for CanvasKitBackend {
    fn begin_frame(&mut self) {
        self.ck.begin_frame();
        if (self.dpr - 1.0).abs() > f32::EPSILON {
            self.ck.scale(self.dpr, self.dpr);
        }
    }
    fn end_frame(&mut self) {
        self.ck.end_frame();
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.ck.fill_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.ck.stroke_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            color.r,
            color.g,
            color.b,
            color.a,
            width,
        );
    }
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.ck.fill_round_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            radius,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }
    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.ck.stroke_round_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            radius,
            color.r,
            color.g,
            color.b,
            color.a,
            width,
        );
    }
    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        self.ck.fill_oval(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }
    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        self.ck.stroke_oval(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
            color.r,
            color.g,
            color.b,
            color.a,
            width,
        );
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.ck.stroke_line(
            from.x, from.y, to.x, to.y, color.r, color.g, color.b, color.a, width,
        );
    }
    fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
        if points.len() < 3 {
            return;
        }
        let mut flat: Vec<f32> = Vec::with_capacity(points.len() * 2);
        for p in points {
            flat.push(p.x);
            flat.push(p.y);
        }
        self.ck
            .fill_polygon(&flat, color.r, color.g, color.b, color.a);
    }
    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        // lucide d-strings use a 24x24 viewBox.
        self.ck.stroke_svg_path(
            d,
            top_left.x,
            top_left.y,
            size / 24.0,
            color.r,
            color.g,
            color.b,
            color.a,
            width,
        );
    }
    fn fill_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, viewbox: f32, color: Color) {
        let even_odd = svg_path_even_odd(d);
        self.ck.fill_svg_path(
            d,
            top_left.x,
            top_left.y,
            size / viewbox.max(1.0),
            even_odd,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }
    fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color) {
        let even_odd = svg_path_even_odd(d);
        self.ck.fill_svg_path_in_rect(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            even_odd,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }
    fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32) {
        self.ck.stroke_svg_path_in_rect(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            color.r,
            color.g,
            color.b,
            color.a,
            width,
        );
    }
    fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        if stops.is_empty() {
            return;
        }
        let flat = flatten_gradient_stops(stops);
        self.ck.fill_svg_path_in_rect_linear_gradient(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            svg_path_even_odd(d),
            &flat,
            angle_deg,
            opacity,
        );
    }
    fn fill_svg_path_in_rect_radial_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    ) {
        if stops.is_empty() {
            return;
        }
        let flat = flatten_gradient_stops(stops);
        self.ck.fill_svg_path_in_rect_radial_gradient(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            svg_path_even_odd(d),
            &flat,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
    }
    fn fill_inner_shadow_svg_path(
        &mut self,
        d: &str,
        rect: Rect,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: Color,
    ) {
        self.ck.fill_inner_shadow_svg_path(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            svg_path_even_odd(d),
            offset_x,
            offset_y,
            blur,
            color.r,
            color.g,
            color.b,
            color.a,
        );
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        let italic = layout.italic();
        for run in layout.runs() {
            let x = origin.x + run.origin.x;
            let y = origin.y + run.origin.y;
            // `TextRun.color` is `jian_core::scene::Color` (0-255 u8 channels),
            // unlike the f32-channel `Color` the fill ops take.
            let c = run.color;
            self.ck.draw_text(
                run.content.as_str(),
                x,
                y,
                run.font_size,
                run.font_weight as i32,
                italic,
                f32::from(c.r()) / 255.0,
                f32::from(c.g()) / 255.0,
                f32::from(c.b()) / 255.0,
                f32::from(c.a()) / 255.0,
            );
        }
    }
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.ck.measure_text(text, font_size)
    }
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, _weight: u16) -> f32 {
        self.ck.measure_text(text, font_size)
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.ck
            .clip_rect(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y);
    }
    fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
        self.ck.clip_round_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            radius,
        );
    }
    fn save(&mut self) {
        self.ck.save();
    }
    fn restore(&mut self) {
        self.ck.restore();
    }
    fn translate(&mut self, offset: Point2D) {
        self.ck.translate(offset.x, offset.y);
    }
    fn scale(&mut self, scale: Point2D, pivot: Point2D) {
        // Mirror the native backend: scale about a pivot.
        self.ck.translate(pivot.x, pivot.y);
        self.ck.scale(scale.x, scale.y);
        self.ck.translate(-pivot.x, -pivot.y);
    }
    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.ck.rotate(radians.to_degrees(), pivot.x, pivot.y);
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.logical_w = width.max(1);
        self.logical_h = height.max(1);
        let pw = ((width as f32) * self.dpr).round() as u32;
        let ph = ((height as f32) * self.dpr).round() as u32;
        self.ck.resize(pw.max(1), ph.max(1));
    }
    fn dpi_scale(&self) -> f32 {
        self.dpr
    }
}

/// Initialise CanvasKit on `canvas_id` and return a ready `CanvasKitBackend`.
pub(crate) async fn init_backend(
    canvas_id: &str,
    dpr: f32,
    logical_w: u32,
    logical_h: u32,
) -> Result<CanvasKitBackend, JsValue> {
    let promise = op_ck_init(canvas_id)?;
    let ck_val = wasm_bindgen_futures::JsFuture::from(promise).await?;
    let ck: OpCk = ck_val.unchecked_into();
    Ok(CanvasKitBackend::new(ck, dpr, logical_w, logical_h))
}

/// Live shell state for the CanvasKit host: the widget host + its backend.
struct CkInner {
    backend: CanvasKitBackend,
    host: crate::widget_host::WidgetHost,
    canvas: web_sys::HtmlCanvasElement,
    /// Hidden ARIA DOM mirror (#57) — kept in sync after every paint so a
    /// screen reader can read the opaque CanvasKit surface. `None` only if
    /// the DOM container couldn't be created (non-browser host).
    a11y: Option<crate::a11y_dom::A11yDomMirror>,
}

impl CkInner {
    fn repaint(&mut self) {
        let (w, h) = self.backend.logical_size();
        self.backend.begin_frame();
        self.host.paint_dyn(&mut self.backend, w, h);
        self.backend.end_frame();
        self.sync_a11y();
    }

    /// Rebuild the hidden ARIA DOM mirror from a freshly assembled tree.
    /// Called after each paint so the mirror tracks the painted frame
    /// (cheap: ~8 always-present region nodes). A diff-or-rebuild refinement
    /// can replace the full rebuild later; v1 rebuilds.
    fn sync_a11y(&mut self) {
        if let Some(mirror) = self.a11y.as_mut() {
            let (w, h) = self.backend.logical_size();
            let tree = self.host.accessibility_tree_update(w, h);
            mirror.update(&tree);
        }
    }

    fn resize_to_window(&mut self, window: &web_sys::Window) -> Result<bool, JsValue> {
        let css_w = window
            .inner_width()?
            .as_f64()
            .unwrap_or_else(|| self.canvas.client_width().max(1) as f64)
            .round()
            .max(1.0) as u32;
        let css_h = window
            .inner_height()?
            .as_f64()
            .unwrap_or_else(|| self.canvas.client_height().max(1) as f64)
            .round()
            .max(1.0) as u32;
        let native_dpr = (window.device_pixel_ratio() as f32).max(1.0);
        let one_x_dots = ((css_w as f32) * (css_h as f32)).max(1.0);
        let capped_dpr = (STEADY_CANVAS_PIXEL_BUDGET / one_x_dots).sqrt().max(1.0);
        let dpr = native_dpr.min(capped_dpr).max(1.0);
        let dev_w = ((css_w as f32) * dpr).round().max(1.0) as u32;
        let dev_h = ((css_h as f32) * dpr).round().max(1.0) as u32;

        let style = format!("width: {css_w}px; height: {css_h}px;");
        let mut changed = self.canvas.get_attribute("style").as_deref() != Some(style.as_str());
        if changed {
            self.canvas.set_attribute("style", &style)?;
        }
        if self.canvas.width() != dev_w {
            self.canvas.set_width(dev_w);
            changed = true;
        }
        if self.canvas.height() != dev_h {
            self.canvas.set_height(dev_h);
            changed = true;
        }
        let (logical_w, logical_h) = self.backend.logical_size();
        let backend_changed = logical_w.round() as u32 != css_w
            || logical_h.round() as u32 != css_h
            || (self.backend.dpr - dpr).abs() > f32::EPSILON;
        if changed || backend_changed {
            self.backend.resize_for_display(css_w, css_h, dpr);
            return Ok(true);
        }
        Ok(false)
    }

    fn event_offset_to_logical(&self, offset_x: f32, offset_y: f32) -> (f32, f32) {
        let (logical_w, logical_h) = self.backend.logical_size();
        crate::event::pointer::map_offset_to_logical(
            offset_x,
            offset_y,
            self.canvas.client_width().max(1) as f32,
            self.canvas.client_height().max(1) as f32,
            logical_w,
            logical_h,
        )
    }
}

impl crate::repaint_ctx::RepaintContext for CkInner {
    fn host(&self) -> &crate::widget_host::WidgetHost {
        &self.host
    }
    fn host_mut(&mut self) -> &mut crate::widget_host::WidgetHost {
        &mut self.host
    }
    fn viewport_size(&self) -> (f32, f32) {
        self.backend.logical_size()
    }
    fn canvas_data_url(&self, mime: &str) -> Result<String, JsValue> {
        self.canvas.to_data_url_with_type(mime)
    }
    fn register_system_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.backend.ck.register_system_font(family, bytes)
    }
    fn repaint(&mut self) -> Result<(), JsValue> {
        // CanvasKit present is infallible (GPU flush, no pixel round-trip).
        CkInner::repaint(self);
        Ok(())
    }
}

/// Resolve an accessibility DOM event's target to its `accesskit::NodeId`
/// and route it into the host (#57). `is_focus` distinguishes `focusin`
/// from `click`. Repaints on a state change so the canvas + the mirror
/// re-sync with the screen-reader-driven focus / activation.
fn dispatch_a11y_dom_event(
    inner: &std::rc::Rc<std::cell::RefCell<CkInner>>,
    target: Option<web_sys::EventTarget>,
    is_focus: bool,
) {
    use wasm_bindgen::JsCast;
    let Some(element) = target.and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
        return;
    };
    let Some(node_id) = crate::a11y_dom::A11yDomMirror::node_id_for_target(&element) else {
        return;
    };
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host.set_clocks(
        crate::listener::now_ms_perf(),
        crate::listener::now_unix_secs(),
    );
    if b.host.apply_a11y_action(node_id.0, is_focus) {
        b.repaint();
    }
}

/// Mount the full editor chrome on `canvas_id`, rendered via CanvasKit on the
/// GPU, with mouse / wheel / keyboard interactivity. Builds the shared
/// `WidgetHost` (skia-free under this feature) and drives it through
/// `CanvasKitBackend`, behind the same `RenderBackend` the desktop host uses.
#[wasm_bindgen]
pub async fn mount_ck(canvas_id: String) -> Result<(), JsValue> {
    use crate::listener::{add_listener, now_ms_perf, now_unix_secs, Listener};
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::{KeyboardEvent, MouseEvent, WheelEvent};

    console_error_panic_hook::set_once();

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("mount_ck: no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("mount_ck: no document"))?;
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str("mount_ck: canvas not found"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("mount_ck: element is not a <canvas>"))?;

    // Device backing store vs CSS size → device-pixel-ratio.
    let dev_w = canvas.width().max(1) as f32;
    let dev_h = canvas.height().max(1) as f32;
    let css_w = (canvas.client_width().max(1)) as f32;
    let dpr = (dev_w / css_w).max(1.0);
    let logical_w = (dev_w / dpr).round().max(1.0) as u32;
    let logical_h = (dev_h / dpr).round().max(1.0) as u32;

    let backend = init_backend(&canvas_id, dpr, logical_w, logical_h).await?;
    let host = crate::widget_host::WidgetHost::new();
    // Hidden ARIA DOM mirror (#57) — created next to the canvas, refreshed
    // after every paint so screen readers can read the opaque GPU surface.
    let a11y = crate::a11y_dom::A11yDomMirror::create(&canvas);
    let inner = Rc::new(RefCell::new(CkInner {
        backend,
        host,
        canvas: canvas.clone(),
        a11y,
    }));
    {
        let mut b = inner.borrow_mut();
        let _ = b.resize_to_window(&window)?;
        b.repaint();
    }
    crate::web_fonts::drain_font_requests(&inner);

    // Populate the chat model picker from the daemon's `/api/ai/models`
    // catalog (best-effort; async, repaints when the response lands).
    crate::web_chat::fetch_models(&inner);
    // Bidirectional live-canvas sync with the daemon (pull on version bump,
    // push local edits + selection) — same loops the skia mount wires.
    crate::live_sync_glue::start(&inner);

    let mut listeners: Vec<Listener> = Vec::new();
    let canvas_target: web_sys::EventTarget = canvas.clone().into();
    let win_target: web_sys::EventTarget = window.clone().into();

    // Accessibility DOM mirror (#57): delegated `focus` / `click` on the
    // hidden mirror container map a focused/activated mirror node back to a
    // host action (focus chat input, blur it on canvas/panel focus, …) then
    // repaint so the canvas reflects the screen-reader-driven change.
    let mirror_target = inner.try_borrow().ok().and_then(|b| {
        b.a11y
            .as_ref()
            .map(|m| -> web_sys::EventTarget { m.container().clone().into() })
    });
    if let Some(mirror_target) = mirror_target {
        // `focusin` bubbles (unlike `focus`), so a single delegated listener
        // on the container catches focus landing on any descendant node.
        {
            let inner = inner.clone();
            add_listener::<web_sys::FocusEvent, _, _>(
                &mirror_target,
                "focusin",
                &mut listeners,
                move |evt| {
                    dispatch_a11y_dom_event(&inner, evt.target(), true);
                },
            )?;
        }
        {
            let inner = inner.clone();
            add_listener::<MouseEvent, _, _>(
                &mirror_target,
                "click",
                &mut listeners,
                move |evt| {
                    dispatch_a11y_dom_event(&inner, evt.target(), false);
                },
            )?;
        }
    }

    // mousedown → press / right-press
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "mousedown",
            &mut listeners,
            move |evt| {
                use crate::event::pointer::{classify_mouse_press_button, MousePressAction};

                let action = classify_mouse_press_button(evt.button());
                if matches!(action, MousePressAction::Ignore) {
                    return;
                }
                if matches!(action, MousePressAction::MiddlePan) {
                    evt.prevent_default();
                }
                let Ok(mut b) = inner.try_borrow_mut() else {
                    return;
                };
                b.host.set_modifier_shift(evt.shift_key());
                b.host.set_clocks(now_ms_perf(), now_unix_secs());
                let (w, h) = b.backend.logical_size();
                let (x, y) =
                    b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
                let consumed = match action {
                    MousePressAction::PrimaryPress => b.host.apply_press(x, y, w, h),
                    MousePressAction::MiddlePan => {
                        let started = b.host.apply_pan_press(x, y, w, h);
                        b.host.set_space_pan(started);
                        started
                    }
                    MousePressAction::ContextPress => b.host.apply_right_press(x, y, w, h),
                    MousePressAction::Ignore => false,
                };
                if consumed {
                    b.repaint();
                }
                // Release the borrow before draining: a Send / Stop / New Chat
                // button press raised a chat flag; an icon-picker Load-more
                // press queued a remote search. Both drains re-borrow `inner`
                // (mirrors the skia mount's post-press drain points).
                drop(b);
                crate::web_chat::drain_chat_flags(&inner);
                crate::iconify_web::drain_iconify_request(&inner);
                crate::codegen_web::drain_codegen_flags(&inner);
                crate::dom_io::drain_pending_file_action(&inner);
                crate::dom_io::drain_pending_attachment_pick(&inner);
                crate::dom_io::drain_pending_kit_io(&inner);
                crate::web_agent_connect::drain_pending_provider_connect(&inner);
                crate::web_acp_connect::drain_pending_acp_agent_connect(&inner);
                crate::web_fonts::drain_font_requests(&inner);
            },
        )?;
    }
    // Suppress browser's native context menu over the canvas so right-click
    // stays reserved for editor context menus.
    {
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "contextmenu",
            &mut listeners,
            move |evt| {
                evt.prevent_default();
            },
        )?;
    }
    // mousemove → cursor move / drag
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(
            &canvas_target,
            "mousemove",
            &mut listeners,
            move |evt| {
                let Ok(mut b) = inner.try_borrow_mut() else {
                    return;
                };
                b.host.set_clocks(now_ms_perf(), now_unix_secs());
                let (x, y) =
                    b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
                if b.host.apply_cursor_move(x, y) {
                    b.repaint();
                }
            },
        )?;
    }
    // mouseup → release
    {
        let inner = inner.clone();
        add_listener::<MouseEvent, _, _>(&canvas_target, "mouseup", &mut listeners, move |evt| {
            if evt.button() != 0 && evt.button() != 1 {
                return;
            }
            if evt.button() == 1 {
                evt.prevent_default();
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_clocks(now_ms_perf(), now_unix_secs());
            let (w, h) = b.backend.logical_size();
            let was_middle = evt.button() == 1;
            if b.host.apply_release_with_viewport(w, h) {
                b.repaint();
            }
            if was_middle {
                b.host.set_space_pan(false);
            }
        })?;
    }
    // wheel → pan / zoom
    {
        let inner = inner.clone();
        add_listener::<WheelEvent, _, _>(&canvas_target, "wheel", &mut listeners, move |evt| {
            use crate::event::pointer::{classify_wheel_intent, WheelIntent};

            evt.prevent_default();
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            let (w, h) = b.backend.logical_size();
            let (x, y) = b.event_offset_to_logical(evt.offset_x() as f32, evt.offset_y() as f32);
            let consumed = match classify_wheel_intent(
                evt.delta_x() as f32,
                evt.delta_y() as f32,
                evt.shift_key(),
                evt.ctrl_key(),
                evt.meta_key(),
                evt.alt_key(),
            ) {
                WheelIntent::Zoom { delta_y } => b.host.apply_wheel(x, y, delta_y, w, h),
                WheelIntent::Pan { dx, dy } => b.host.apply_pan_gesture(x, y, dx, dy, w, h),
            };
            if consumed {
                b.repaint();
            }
        })?;
    }
    // keydown → text input + editor shortcuts. `apply_key` is a stub on this
    // host; real input is dispatched per-key to apply_text / apply_backspace /
    // apply_send / nudge / reorder / clipboard / undo etc. (mirrors the skia
    // mount in lib.rs). Mod = Cmd/Ctrl; named-key shortcuts gate on `!is_mod`.
    {
        let inner = inner.clone();
        add_listener::<KeyboardEvent, _, _>(&win_target, "keydown", &mut listeners, move |evt| {
            use op_editor_core::ReorderDirection;
            // In-flight IME composition owns its keystrokes (handled on commit).
            if evt.is_composing() {
                return;
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_clocks(now_ms_perf(), now_unix_secs());
            let key = evt.key();
            let starts_space_pan = evt.code() == "Space"
                && !evt.repeat()
                && !evt.meta_key()
                && !evt.ctrl_key()
                && !evt.alt_key();
            let is_mod = evt.meta_key() || evt.ctrl_key();
            let shift = evt.shift_key();
            let nudge = if shift { 10.0 } else { 1.0 };
            let mut consumed = false;
            if starts_space_pan && !b.host.input_active() {
                b.host.set_space_pan(true);
                evt.prevent_default();
            }
            match key.as_str() {
                "Backspace" if !is_mod => consumed = b.host.apply_backspace(),
                "Delete" if !is_mod => consumed = b.host.apply_delete(),
                "Enter" if !is_mod => consumed = b.host.apply_send(),
                "Escape" if !is_mod => consumed = b.host.apply_escape(),
                "ArrowUp" if !is_mod => {
                    consumed =
                        b.host.apply_text_edit_vertical(false) || b.host.apply_nudge(0.0, -nudge)
                }
                "ArrowDown" if !is_mod => {
                    consumed =
                        b.host.apply_text_edit_vertical(true) || b.host.apply_nudge(0.0, nudge)
                }
                "ArrowLeft" if is_mod => consumed = b.host.apply_text_edit_line_edge(false),
                "ArrowRight" if is_mod => consumed = b.host.apply_text_edit_line_edge(true),
                "ArrowLeft" if !is_mod => {
                    consumed = b.host.apply_settings_caret(false)
                        || b.host.apply_chat_model_picker_caret(false)
                        || b.host.apply_rename_caret(false)
                        || b.host.apply_text_edit_caret(false)
                        || b.host.apply_property_caret(false)
                        || b.host.apply_nudge(-nudge, 0.0);
                }
                "ArrowRight" if !is_mod => {
                    consumed = b.host.apply_settings_caret(true)
                        || b.host.apply_chat_model_picker_caret(true)
                        || b.host.apply_rename_caret(true)
                        || b.host.apply_text_edit_caret(true)
                        || b.host.apply_property_caret(true)
                        || b.host.apply_nudge(nudge, 0.0);
                }
                "[" if !is_mod => consumed = b.host.apply_reorder(ReorderDirection::Down),
                "]" if !is_mod => consumed = b.host.apply_reorder(ReorderDirection::Up),
                "d" if is_mod && !shift => consumed = b.host.apply_duplicate(),
                "a" if is_mod && !shift => consumed = b.host.apply_select_all(),
                "c" if is_mod && !shift => consumed = b.host.apply_copy(),
                "x" if is_mod && !shift => consumed = b.host.apply_cut(),
                // Cmd/Ctrl+V is owned by the DOM `paste` listener
                // (`dom_io::register_io_listeners` → `handle_paste_event`): it
                // routes the system clipboard first (Figma HTML / image / text),
                // then falls back to the internal node clipboard. Calling
                // `apply_paste` here would preempt that with the STALE internal
                // clipboard + double-paste, so leave Cmd+V unconsumed → the
                // browser's native paste fires the `paste` event. (Mirrors the
                // skia codegen build, lib.rs.)
                "v" if is_mod && !shift => {}
                "z" if is_mod && !shift => consumed = b.host.apply_undo(),
                "Z" if is_mod && shift => consumed = b.host.apply_redo(),
                "y" if is_mod && !shift => consumed = b.host.apply_redo(),
                _ => {
                    // Printable char → text input (only when no Cmd/Ctrl held,
                    // so unbound chords don't type into a focused field).
                    if !is_mod {
                        let mut chars = key.chars();
                        if let (Some(c), None) = (chars.next(), chars.next()) {
                            if !c.is_control() && b.host.apply_text(c) {
                                consumed = true;
                            }
                        }
                    }
                }
            }
            if consumed {
                evt.prevent_default();
                b.repaint();
            }
            // Release the borrow before draining: Enter may have queued a chat
            // send (apply_send → pending_send); the drain launches the turn.
            drop(b);
            crate::web_chat::drain_chat_flags(&inner);
        })?;
    }

    {
        let inner = inner.clone();
        add_listener::<KeyboardEvent, _, _>(&win_target, "keyup", &mut listeners, move |evt| {
            if evt.code() != "Space" {
                return;
            }
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            b.host.set_space_pan(false);
            evt.prevent_default();
        })?;
    }

    {
        let inner = inner.clone();
        let window_for_resize = window.clone();
        add_listener::<web_sys::Event, _, _>(&win_target, "resize", &mut listeners, move |_evt| {
            let Ok(mut b) = inner.try_borrow_mut() else {
                return;
            };
            match b.resize_to_window(&window_for_resize) {
                Ok(true) => b.repaint(),
                Ok(false) => {}
                Err(err) => web_sys::console::error_1(&err),
            }
        })?;
    }

    crate::dom_io::register_io_listeners(&inner, &canvas, &win_target, &mut listeners)?;

    // Retain the shell + its listeners for the page lifetime. (A future
    // WebShell-style handle can own these for explicit teardown; leaking keeps
    // the CanvasKit surface + DOM closures alive for now.)
    std::mem::forget(inner);
    std::mem::forget(listeners);
    Ok(())
}

/// Smoke entry retained for FFI validation (renders AA text + a fill).
#[wasm_bindgen]
pub async fn ck_smoke(canvas_id: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let mut be = init_backend(&canvas_id, 1.0, 800, 300).await?;
    be.ck.clear(1.0, 1.0, 1.0, 1.0);
    be.fill_rect(
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(800.0, 60.0),
        },
        Color {
            r: 0.06,
            g: 0.06,
            b: 0.06,
            a: 1.0,
        },
    );
    be.ck.draw_text(
        "OpenPencil Rust -> CanvasKit GPU",
        20.0,
        40.0,
        28.0,
        400,
        false,
        0.9,
        0.9,
        0.95,
        1.0,
    );
    be.end_frame();
    Ok(())
}
