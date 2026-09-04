use crate::import_warning::ImportWarning;
use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::constraints::{Constraints, HConstraint, VConstraint};
use jian_ops_schema::node::base::{NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::{ContainerProps, CornerRadius, LayoutMode};
use jian_ops_schema::node::image::{ImageFitMode, ImageNode};
use jian_ops_schema::node::path::{PathFillRule, PathNode};
use jian_ops_schema::node::text::TextAlign;
use jian_ops_schema::node::{FrameNode, ImageSrc, PenNode};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{BlendMode, PenFill, SolidFillBody};
use serde_json::{Map, Value};

use crate::color::parse_css_color;
use crate::css::cascade::ComputedStyle;
use crate::mapper::{container_props_from, fill_glyph_color, MapCtx};
use crate::{
    wrap_imported_document, HtmlDocumentResult, HtmlImportOptions, HtmlImportResult,
    MAX_OUTPUT_NODES,
};

#[path = "snapshot_stack.rs"]
mod snapshot_stack;
#[path = "snapshot_text.rs"]
mod text_run;

const MAX_SNAPSHOT_BYTES: usize = 48 * 1024 * 1024;

pub const SNAPSHOT_EXTRACTOR_JS: &str = include_str!("../assets/snapshot-extractor.js");

/// A browser-snapshot payload that could not be imported. `import_snapshot`
/// surfaces the `Display` text as the import's single warning, so the
/// rendering must stay byte-identical to the strings this replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// Payload exceeds [`MAX_SNAPSHOT_BYTES`].
    TooLarge,
    /// The payload is not parseable JSON; carries the serde message
    /// (`serde_json::Error` is neither `Clone` nor `Eq`).
    InvalidJson(String),
    /// The payload parsed but is not a JSON object.
    NotAnObject,
    /// A `version` field that this importer does not understand.
    UnsupportedVersion(u64),
    /// No `version` field at all.
    MissingVersion,
    /// No `root` object.
    MissingRoot,
    /// The root node's `rect` is absent or non-finite.
    InvalidRootRect,
    /// The root element produced no `PenNode`.
    RootConversionFailed,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("snapshot JSON exceeds the 48 MiB input limit"),
            Self::InvalidJson(detail) => write!(formatter, "invalid snapshot JSON: {detail}"),
            Self::NotAnObject => formatter.write_str("snapshot JSON must be an object"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "unsupported snapshot version {version}; expected 1"
                )
            }
            Self::MissingVersion => {
                formatter.write_str("snapshot version is required and must equal 1")
            }
            Self::MissingRoot => formatter.write_str("snapshot root is required"),
            Self::InvalidRootRect => {
                formatter.write_str("snapshot root rect is missing or invalid")
            }
            Self::RootConversionFailed => {
                formatter.write_str("snapshot root could not be converted")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

pub fn import_snapshot(json: &str, opts: &HtmlImportOptions) -> HtmlImportResult {
    match import_snapshot_inner(json, opts) {
        Ok(result) => result,
        Err(error) => HtmlImportResult::new(
            Vec::new(),
            vec![ImportWarning::SnapshotRejected {
                reason: error.to_string(),
            }],
        ),
    }
}

pub fn import_snapshot_document(json: &str, opts: &HtmlImportOptions) -> HtmlDocumentResult {
    wrap_imported_document(import_snapshot(json, opts))
}

fn import_snapshot_inner(
    json: &str,
    opts: &HtmlImportOptions,
) -> Result<HtmlImportResult, SnapshotError> {
    if json.len() > MAX_SNAPSHOT_BYTES {
        return Err(SnapshotError::TooLarge);
    }
    let value: Value = serde_json::from_str(json)
        .map_err(|error| SnapshotError::InvalidJson(error.to_string()))?;
    let object = value.as_object().ok_or(SnapshotError::NotAnObject)?;
    match object.get("version").and_then(Value::as_u64) {
        Some(1) => {}
        Some(version) => return Err(SnapshotError::UnsupportedVersion(version)),
        None => return Err(SnapshotError::MissingVersion),
    }
    let root = object
        .get("root")
        .and_then(Value::as_object)
        .ok_or(SnapshotError::MissingRoot)?;
    let root_rect = Rect::from_node(root).ok_or(SnapshotError::InvalidRootRect)?;
    let title = object
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or("Web Snapshot");
    let mut context = SnapshotCtx::new(opts);
    // A picked subtree usually starts below the element carrying the page
    // backdrop, so the capture reports it separately. Without it a dark-theme
    // page lands on the importer's white default and every light-on-dark run
    // reads as washed out.
    context.page_background = object
        .get("background")
        .and_then(Value::as_str)
        .and_then(parse_css_color);
    let root_node = context
        .map_element(root, root_rect, None, Some(title))
        .ok_or(SnapshotError::RootConversionFailed)?;
    if object
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        context.warn_once(ImportWarning::SnapshotTruncated);
    }
    if context.output_truncated {
        context.warn_once(ImportWarning::SnapshotNodeLimit);
    }
    if context.tainted_images > 0 {
        context.warnings.push(ImportWarning::SnapshotTaintedImages {
            count: context.tainted_images,
        });
    }
    Ok(HtmlImportResult::new(vec![root_node], context.warnings))
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn from_node(node: &Map<String, Value>) -> Option<Self> {
        Self::from_field(node, "rect")
    }

    fn from_field(node: &Map<String, Value>, key: &str) -> Option<Self> {
        let rect = node.get(key)?.as_object()?;
        let value = Self {
            x: rect.get("x")?.as_f64()?,
            y: rect.get("y")?.as_f64()?,
            w: rect.get("w")?.as_f64()?,
            h: rect.get("h")?.as_f64()?,
        };
        (value.x.is_finite()
            && value.y.is_finite()
            && value.w.is_finite()
            && value.h.is_finite()
            && value.w >= 0.0
            && value.h >= 0.0)
            .then_some(value)
    }
}

struct SnapshotCtx<'a> {
    opts: &'a HtmlImportOptions,
    warnings: Vec<ImportWarning>,
    /// Rendered text of every warning already in `warnings` — see
    /// [`crate::mapper::MapCtx::warned`]. Keeps `warn_once` a hash probe
    /// instead of re-rendering the stored list on every call.
    warned: BTreeSet<String>,
    next_id: usize,
    node_count: usize,
    output_truncated: bool,
    tainted_images: usize,
    /// The page's own backdrop colour, used for the root frame when the
    /// captured root has no background of its own.
    page_background: Option<String>,
    /// Glyph colour inherited from a `background-clip: text` ancestor. The
    /// gradient-text idiom paints the ancestor's background *through* the
    /// glyphs and makes the glyph colour itself transparent; descendant text
    /// whose own colour is transparent picks this up instead (see
    /// `map_element`).
    text_fill_override: Option<String>,
}

impl<'a> SnapshotCtx<'a> {
    fn new(opts: &'a HtmlImportOptions) -> Self {
        Self {
            opts,
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            output_truncated: false,
            tainted_images: 0,
            page_background: None,
            text_fill_override: None,
        }
    }

    fn allocate_id(&mut self) -> Option<String> {
        if self.node_count >= MAX_OUTPUT_NODES {
            self.output_truncated = true;
            return None;
        }
        let id = format!("snapshot_{}", self.next_id);
        self.next_id += 1;
        self.node_count += 1;
        Some(id)
    }

    /// Record `warning` unless an identical rendered message is already
    /// stored. Dedupe semantics are rendered-string equality, exactly as
    /// before; the set keeps it O(1) per call instead of O(n).
    fn warn_once(&mut self, warning: ImportWarning) {
        if self.warned.insert(warning.to_string()) {
            self.warnings.push(warning);
        }
    }

    fn map_child(&mut self, value: &Value, parent_rect: Rect) -> Option<PenNode> {
        let object = value.as_object()?;
        let Some(rect) = Rect::from_node(object) else {
            self.warn_once(ImportWarning::SnapshotInvalidRect);
            return None;
        };
        match object.get("kind").and_then(Value::as_str) {
            Some("element") => self.map_element(object, rect, Some(parent_rect), None),
            Some("text") => self.map_text(object, rect, parent_rect),
            // A captured `<svg>` carries both shapes: the image serialization
            // every importer understands and, when the capture could reduce it
            // to one flat fill, native path data. `"vector"` is an interim
            // capture shape that carried the path data alone.
            Some("image") | Some("vector") => self.map_vector_or_image(object, rect, parent_rect),
            _ => {
                self.warn_once(ImportWarning::SnapshotUnknownKind);
                None
            }
        }
    }

    /// Prefer native path geometry, fall back to the image serialization.
    ///
    /// The fallback is what keeps a missing or empty `d` from dropping the
    /// node: a captured `<svg>` always carries `src` too, so "no usable path
    /// data" degrades to exactly the image node this importer produced before
    /// vectorization existed rather than silently losing the icon.
    fn map_vector_or_image(
        &mut self,
        object: &Map<String, Value>,
        rect: Rect,
        parent_rect: Rect,
    ) -> Option<PenNode> {
        self.map_vector(object, rect, parent_rect)
            .or_else(|| self.map_image(object, rect, parent_rect))
    }

    fn map_element(
        &mut self,
        object: &Map<String, Value>,
        rect: Rect,
        parent_rect: Option<Rect>,
        root_name: Option<&str>,
    ) -> Option<PenNode> {
        let id = self.allocate_id()?;
        let styles = style_map(object);
        let mut container = self.container_from_styles(&styles, rect, true);
        container.layout = Some(LayoutMode::None);
        container.width = Some(SizingBehavior::Number(rect.w));
        container.height = Some(SizingBehavior::Number(rect.h));
        // The gradient-text idiom: `background-clip: text` paints this box's
        // background *through* its glyphs, and the glyph colour itself is
        // transparent. Painting the background as an ordinary fill produced a
        // solid gradient bar over invisible text — so the fill moves off the
        // box and becomes the descendants' glyph colour instead (the first
        // gradient stop; per-glyph gradients are not paintable text fills).
        let saved_text_fill = self.text_fill_override.clone();
        if styles.get("background-clip").map(String::as_str) == Some("text") {
            if let Some(color) = container.fill.as_deref().and_then(fill_glyph_color) {
                container.fill = None;
                self.text_fill_override = Some(color);
            }
        }
        if container.clip_content == Some(true)
            && !has_corner_radius(&container)
            && clip_is_inert(object, rect)
        {
            // `overflow: hidden` on a box that nothing overflowed clips
            // nothing in the browser — it is the standard truncation wrapper
            // (`overflow: hidden` + `text-overflow: ellipsis`) sitting exactly
            // around text that fit. Keeping it means the one thing that does
            // differ here — a fallback font a few percent wider — gets its
            // tail cut off, which is how "openpencil" imported as
            // "openpenci". A box the capture *did* overflow keeps its clip:
            // there the browser truncated too, and a scroll container must
            // not spill its contents.
            //
            // A rounded box always keeps its clip, however contained its
            // children are: `overflow: hidden` + `border-radius` is the
            // rounded-card idiom, where the clip is not there to truncate but
            // to round off a child that fills the box (a header image, a
            // coloured strip). Dropping it there paints square corners over
            // the parent's radius — geometry the child rects cannot reveal,
            // because the child is inside the box and only its *corners*
            // stick out.
            container.clip_content = None;
        }
        if parent_rect.is_none() && container.fill.is_none() {
            let backdrop = self
                .page_background
                .clone()
                .unwrap_or_else(|| "#ffffff".into());
            container.fill = Some(vec![solid_fill(backdrop)]);
        }
        let name = root_name.map(str::to_string).or_else(|| {
            object
                .get("tag")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        let mut base = self.base(id, name, rect, parent_rect);
        self.apply_base_styles(&mut base, &styles);
        // `z-index` takes effect on a flex / grid item even while it stays
        // `position: static`, so the children's stacking depends on *this*
        // element's `display`.
        let flex_or_grid_parent = snapshot_stack::is_flex_or_grid(styles.get("display"));
        // Map in DOM order (id allocation and the node budget must follow the
        // capture), then re-lay the survivors into canonical front-to-back
        // order. Snapshot frames position every child explicitly, so this
        // moves paint only.
        let ordered = object
            .get("children")
            .and_then(Value::as_array)
            .map(|children| {
                children
                    .iter()
                    .filter_map(|child| {
                        let bucket = snapshot_stack::paint_bucket(child, flex_or_grid_parent);
                        self.map_child(child, rect).map(|node| (bucket, node))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let children = snapshot_stack::to_front_to_back(ordered);
        self.text_fill_override = saved_text_fill;
        Some(PenNode::Frame(FrameNode {
            base,
            container,
            breakpoint: None,
            children: Some(children),
            image_search_query: None,
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
            screen: None,
        }))
    }

    fn map_image(
        &mut self,
        object: &Map<String, Value>,
        rect: Rect,
        parent_rect: Rect,
    ) -> Option<PenNode> {
        let id = self.allocate_id()?;
        let styles = style_map(object);
        let visual = self.container_from_styles(&styles, rect, false);
        let object_fit = match styles.get("object-fit").map(String::as_str) {
            Some("cover") => Some(ImageFitMode::Crop),
            Some("contain") => Some(ImageFitMode::Fit),
            // A snapshot only records `object-fit` when it differs from the
            // CSS initial value, and an `<svg>` / `<canvas>` node has no such
            // property at all: both mean "paint into the whole box".
            _ => Some(ImageFitMode::Fill),
        };
        let blend_mode = match styles
            .get("mix-blend-mode")
            .and_then(|value| crate::mapper::map_blend_mode(value))
        {
            Some(BlendMode::Normal) | None => None,
            Some(mode) => Some(mode),
        };
        if styles
            .get("mix-blend-mode")
            .is_some_and(|value| crate::mapper::map_blend_mode(value).is_none())
        {
            self.warn_once(ImportWarning::ImageMixBlendModeUnsupported);
        }
        if object
            .get("tainted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            self.tainted_images += 1;
        }
        let mut base = self.base(
            id,
            object
                .get("tag")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| Some("img".into())),
            rect,
            Some(parent_rect),
        );
        base.blend_mode = blend_mode;
        Some(PenNode::Image(ImageNode {
            limits: Default::default(),
            base,
            src: ImageSrc::from(
                object
                    .get("src")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            object_fit,
            width: Some(SizingBehavior::Number(rect.w)),
            height: Some(SizingBehavior::Number(rect.h)),
            corner_radius: visual.corner_radius,
            effects: visual.effects,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }))
    }

    /// An inline `<svg>` the capture reduced to a single filled path.
    ///
    /// Skia has no SVG codec, so the data-URI image the capture falls back to
    /// paints as an undecodable placeholder — which is what turned every
    /// icon button into an empty box. A `path` node is painted natively, and
    /// the renderer fits path data to the node box *by its bounds*, so the
    /// node has to be the artwork's own bounding box (`vectorRect`) rather
    /// than the element's used box (`rect`, which is what the image fallback
    /// needs). An `<svg>` sized larger than its art — an icon in a padded
    /// button, or Material's full-viewBox sizing rect — differs between the
    /// two by exactly the amount the glyph would be stretched.
    fn map_vector(
        &mut self,
        object: &Map<String, Value>,
        rect: Rect,
        parent_rect: Rect,
    ) -> Option<PenNode> {
        let data = object.get("d").and_then(Value::as_str)?.trim().to_string();
        if data.is_empty() {
            return None;
        }
        let rect = Rect::from_field(object, "vectorRect")
            .filter(|rect| rect.w > 0.0 && rect.h > 0.0)
            .unwrap_or(rect);
        let id = self.allocate_id()?;
        let styles = style_map(object);
        let mut base = self.base(id, Some("svg".into()), rect, Some(parent_rect));
        // Opacity only, deliberately: `rect` came out of the capture's root
        // CTM, which already carries every transform between the artwork and
        // the page. Re-applying the element's CSS `transform` on top would
        // rotate geometry that is already in its final place. (The capture
        // only vectorizes an axis-aligned CTM, so there is no rotation left to
        // carry either way.)
        self.apply_opacity(&mut base, &styles);
        let fill = object
            .get("fill")
            .and_then(Value::as_str)
            .and_then(parse_css_color)
            .map(|color| vec![solid_fill(color)]);
        let fill_rule = match object.get("fillRule").and_then(Value::as_str) {
            Some("evenodd") => Some(PathFillRule::Evenodd),
            _ => None,
        };
        Some(PenNode::Path(PathNode {
            base,
            icon_id: None,
            d: Some(data),
            anchors: None,
            closed: None,
            fill_rule,
            mask: None,
            width: Some(SizingBehavior::Number(rect.w)),
            height: Some(SizingBehavior::Number(rect.h)),
            limits: Default::default(),
            fill,
            stroke: None,
            effects: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }))
    }

    fn base(
        &self,
        id: String,
        name: Option<String>,
        rect: Rect,
        parent_rect: Option<Rect>,
    ) -> PenNodeBase {
        let (x, y, constraints) = parent_rect
            .map(|parent| {
                (
                    Some(rect.x - parent.x),
                    Some(rect.y - parent.y),
                    Some(Constraints {
                        h: HConstraint::Left,
                        v: VConstraint::Top,
                    }),
                )
            })
            .unwrap_or((None, None, None));
        PenNodeBase {
            id,
            name,
            x,
            y,
            // Computed snapshots carry browser-resolved rectangles. Mark
            // every non-root node as explicitly positioned so the downstream
            // fit-content repair cannot pull it back into flex flow or expand
            // its fixed parent around visible overflow.
            constraints,
            ..PenNodeBase::default()
        }
    }

    /// Resolve the shared visual mapping (fills, radii, shadows, stroke) for
    /// one captured node.
    ///
    /// `rect` is the browser's used box, and it is threaded in as the node's
    /// own size *and* as the containing block. Percentage-relative visuals —
    /// `border-radius: 50%` on an avatar, `background-size: cover`,
    /// `background-position: center` on a sprite — resolve against the used
    /// box in CSS; resolving them against the viewport (the previous
    /// behaviour, since a snapshot carries no `width` declaration) turned a
    /// circular avatar into a square and cropped every sprite at the wrong
    /// offset.
    fn container_from_styles(
        &mut self,
        styles: &BTreeMap<String, String>,
        rect: Rect,
        text_clip_will_transfer: bool,
    ) -> ContainerProps {
        let mut styles = styles.clone();
        styles.insert("width".into(), format!("{}px", rect.w));
        styles.insert("height".into(), format!("{}px", rect.h));
        let computed = computed_style(&styles, self.opts.base_font_size);
        let rules = [];
        let mut map_context = MapCtx {
            opts: self.opts,
            rules: &rules,
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            containing_width: rect.w,
            containing_height: rect.h,
            containing_width_is_definite: true,
            positioned_width: rect.w,
            positioned_height: rect.h,
            auto_margin_handled_by_parent: false,
            pending_base_outcome: Default::default(),
        };
        let container = if text_clip_will_transfer {
            crate::mapper::text_scope::snapshot_container_props(&computed, &mut map_context)
        } else {
            container_props_from(&computed, &mut map_context)
        };
        for warning in map_context.warnings {
            self.warn_once(warning);
        }
        container
    }

    fn apply_opacity(&mut self, base: &mut PenNodeBase, styles: &BTreeMap<String, String>) {
        base.opacity = styles
            .get("opacity")
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(NumberOrExpression::Number);
    }

    fn apply_base_styles(&mut self, base: &mut PenNodeBase, styles: &BTreeMap<String, String>) {
        self.apply_opacity(base, styles);
        let Some(transform) = styles.get("transform") else {
            return;
        };
        if transform == "none" {
            return;
        }
        if let Some(rotation) = matrix_rotation(transform) {
            base.rotation = Some(rotation);
        } else {
            self.warn_once(ImportWarning::SnapshotUnsupportedTransform);
        }
    }
}

/// Half a CSS pixel: the rounding the capture applies to every rect, so
/// anything smaller is noise rather than real overflow.
const OVERFLOW_EPSILON: f64 = 0.5;

/// Does this container round any of its corners?
fn has_corner_radius(container: &ContainerProps) -> bool {
    match &container.corner_radius {
        Some(CornerRadius::Uniform(radius)) => *radius != 0.0,
        Some(CornerRadius::PerCorner(radii)) => radii.iter().any(|radius| *radius != 0.0),
        None => false,
    }
}

/// Does this element's captured subtree stay inside `rect`?
///
/// Only text-shaped content qualifies: an image or vector descendant may be
/// relying on the clip for its crop (a circular avatar behind an
/// `overflow: hidden` wrapper), so those boxes keep clipping whatever their
/// geometry says.
fn clip_is_inert(object: &Map<String, Value>, rect: Rect) -> bool {
    let Some(children) = object.get("children").and_then(Value::as_array) else {
        return true;
    };
    children.iter().all(|child| {
        let Some(child) = child.as_object() else {
            return true;
        };
        if matches!(
            child.get("kind").and_then(Value::as_str),
            Some("image" | "vector")
        ) {
            return false;
        }
        let Some(child_rect) = Rect::from_node(child) else {
            return true;
        };
        if child_rect.x < rect.x - OVERFLOW_EPSILON
            || child_rect.y < rect.y - OVERFLOW_EPSILON
            || child_rect.x + child_rect.w > rect.x + rect.w + OVERFLOW_EPSILON
            || child_rect.y + child_rect.h > rect.y + rect.h + OVERFLOW_EPSILON
        {
            return false;
        }
        clip_is_inert(child, rect)
    })
}

fn style_map(object: &Map<String, Value>) -> BTreeMap<String, String> {
    object
        .get("styles")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|styles| styles.iter())
        .filter_map(|(name, value)| {
            value
                .as_str()
                .map(|value| (name.clone(), value.to_string()))
        })
        .collect()
}

fn computed_style(styles: &BTreeMap<String, String>, default_font_size: f64) -> ComputedStyle {
    let mut props = styles.clone();
    if let Some(border) = styles.get("border") {
        if let Some(width) = border
            .split_whitespace()
            .find(|part| parse_px(part).is_some())
        {
            props.insert("border-width".into(), width.to_string());
        }
        if let Some(style) = border.split_whitespace().find(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "none"
                    | "hidden"
                    | "dotted"
                    | "dashed"
                    | "solid"
                    | "double"
                    | "groove"
                    | "ridge"
                    | "inset"
                    | "outset"
            )
        }) {
            props.insert("border-style".into(), style.to_string());
        }
        if let Some(color) = color_from_border(border) {
            props.insert("border-color".into(), color);
        }
    }
    let font_size = props
        .get("font-size")
        .and_then(|value| parse_px(value))
        .unwrap_or(default_font_size);
    ComputedStyle { props, font_size }
}

fn color_from_border(border: &str) -> Option<String> {
    for prefix in ["rgba(", "rgb(", "hsla(", "hsl("] {
        if let Some(start) = border.find(prefix) {
            let end = border[start..].find(')')? + start + 1;
            return parse_css_color(&border[start..end]);
        }
    }
    border.split_whitespace().find_map(parse_css_color)
}

fn parse_px(value: &str) -> Option<f64> {
    value
        .trim()
        .strip_suffix("px")
        .unwrap_or(value.trim())
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_text_align(value: &str) -> Option<TextAlign> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" | "start" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" | "end" => Some(TextAlign::Right),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    }
}

fn matrix_rotation(value: &str) -> Option<f64> {
    let body = value.trim().strip_prefix("matrix(")?.strip_suffix(')')?;
    let values: Vec<f64> = body
        .split(',')
        .map(|part| part.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    if values.len() != 6 || !values.iter().all(|value| value.is_finite()) {
        return None;
    }
    Some(values[1].atan2(values[0]).to_degrees())
}

fn solid_fill(color: String) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color,
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "snapshot_inline_tests.rs"]
mod inline_tests;
