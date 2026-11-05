//! Reduced Kiwi capture of tesla.fig's 40×40 `aux-扳手` instance.
//! The source component is 120×120; the captured background vector is
//! authored at (60, 60), 48×48 and uses vector-network blob 4674.

use super::tests::{captured_bytes, fresh_ctx, obj, solid_paint, LOOKUP_GUARD};
use super::*;
use crate::common::{clear_icon_lookup, set_icon_lookup};
use crate::figma_types::BlobOrString;
use crate::vector_decoder::compute_svg_path_bounds;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::text::TextContent;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::StrokeThickness;

const TESLA_AUX_BG_NETWORK: &str = concat!(
    "040000000400000001000000000000000000104100000000000000000000000000001041",
    "0000000000001041000090410000000000009041000010410000000000000000bcfe9ec0",
    "000000000100000000000000bcfe9ec0000000000100000000000000bcfe9e4002000000",
    "bcfe9ec0000000000000000002000000bcfe9e40000000000300000000000000bcfe9e40",
    "000000000300000000000000bcfe9ec00000000099789f40000000000100000001000000",
    "0400000000000000010000000200000003000000"
);
const MINIMAL_24PX_COMMAND_GEOMETRY: &str = concat!(
    "010000000000000000",
    "020000c04100000000",
    "020000c0410000c041",
    "02000000000000c041",
    "00"
);

fn guid(session_id: u32, local_id: u32) -> FigValue {
    obj(vec![
        ("sessionID", FigValue::Uint(session_id)),
        ("localID", FigValue::Uint(local_id)),
    ])
}

fn size(x: f32, y: f32) -> FigValue {
    obj(vec![("x", FigValue::Float(x)), ("y", FigValue::Float(y))])
}

fn transform(x: f32, y: f32) -> FigValue {
    obj(vec![
        ("m00", FigValue::Float(1.0)),
        ("m01", FigValue::Float(0.0)),
        ("m02", FigValue::Float(x)),
        ("m10", FigValue::Float(0.0)),
        ("m11", FigValue::Float(1.0)),
        ("m12", FigValue::Float(y)),
    ])
}

fn child<'a>(frame: &'a jian_ops_schema::node::FrameNode, name: &str) -> &'a PenNode {
    frame
        .children
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|node| match node {
            PenNode::Path(node) => node.base.name.as_deref() == Some(name),
            PenNode::Rectangle(node) => node.base.name.as_deref() == Some(name),
            PenNode::Text(node) => node.base.name.as_deref() == Some(name),
            _ => false,
        })
        .unwrap_or_else(|| panic!("missing converted child {name}"))
}

#[test]
fn derived_instance_scales_geometry_and_visual_metrics_into_40px_frame() {
    let _guard = LOOKUP_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    set_icon_lookup(|_| None);

    let vector = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("VECTOR".into())),
            ("guid", guid(238, 1665)),
            ("name", FigValue::Str("aux-bg".into())),
            ("size", size(48.0, 48.0)),
            ("transform", transform(60.0, 60.0)),
            ("strokeWeight", FigValue::Float(0.9)),
            (
                "fillPaints",
                FigValue::Array(vec![solid_paint(1.0, 0.95, 0.85)]),
            ),
            (
                "strokePaints",
                FigValue::Array(vec![solid_paint(1.0, 0.8, 0.45)]),
            ),
            (
                "vectorData",
                obj(vec![
                    ("vectorNetworkBlob", FigValue::Uint(0)),
                    ("normalizedSize", size(18.0, 18.0)),
                ]),
            ),
        ]),
        children: vec![],
    };
    let rounded = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("ROUNDED_RECTANGLE".into())),
            ("guid", guid(238, 1666)),
            ("name", FigValue::Str("scaled-corners".into())),
            ("size", size(30.0, 30.0)),
            ("transform", transform(12.0, 12.0)),
            ("rectangleCornerRadiiIndependent", FigValue::Bool(true)),
            ("rectangleTopLeftCornerRadius", FigValue::Float(6.0)),
            ("rectangleTopRightCornerRadius", FigValue::Float(9.0)),
            ("rectangleBottomRightCornerRadius", FigValue::Float(12.0)),
            ("rectangleBottomLeftCornerRadius", FigValue::Float(15.0)),
            ("borderStrokeWeightsIndependent", FigValue::Bool(true)),
            ("borderTopWeight", FigValue::Float(3.0)),
            ("borderRightWeight", FigValue::Float(6.0)),
            ("borderBottomWeight", FigValue::Float(9.0)),
            ("borderLeftWeight", FigValue::Float(12.0)),
            (
                "strokePaints",
                FigValue::Array(vec![solid_paint(0.0, 0.0, 0.0)]),
            ),
        ]),
        children: vec![],
    };
    let direct_geometry = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("VECTOR".into())),
            ("guid", guid(238, 1668)),
            ("name", FigValue::Str("direct-geometry".into())),
            ("size", size(24.0, 24.0)),
            ("transform", transform(90.0, 0.0)),
            (
                "fillPaints",
                FigValue::Array(vec![solid_paint(1.0, 0.0, 0.0)]),
            ),
            (
                "fillGeometry",
                FigValue::Array(vec![obj(vec![("commandsBlob", FigValue::Uint(1))])]),
            ),
        ]),
        children: vec![],
    };
    let text = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("guid", guid(238, 1667)),
            ("name", FigValue::Str("scaled-label".into())),
            ("size", size(60.0, 18.0)),
            ("transform", transform(30.0, 90.0)),
            ("fontSize", FigValue::Float(18.0)),
            (
                "lineHeight",
                obj(vec![
                    ("units", FigValue::Str("PIXELS".into())),
                    ("value", FigValue::Float(24.0)),
                ]),
            ),
            (
                "letterSpacing",
                obj(vec![
                    ("units", FigValue::Str("PIXELS".into())),
                    ("value", FigValue::Float(3.0)),
                ]),
            ),
            (
                "textData",
                obj(vec![
                    ("characters", FigValue::Str("A".into())),
                    ("characterStyleIDs", FigValue::Array(vec![FigValue::Int(1)])),
                    (
                        "styleOverrideTable",
                        FigValue::Array(vec![
                            obj(vec![]),
                            obj(vec![("fontSize", FigValue::Float(12.0))]),
                        ]),
                    ),
                ]),
            ),
        ]),
        children: vec![],
    };
    let symbol = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("SYMBOL".into())),
            ("guid", guid(238, 1664)),
            ("name", FigValue::Str("aux-扳手".into())),
            ("size", size(120.0, 120.0)),
        ]),
        children: vec![vector, rounded, direct_geometry, text],
    };
    let instance = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("name", FigValue::Str("aux-扳手".into())),
            ("size", size(40.0, 40.0)),
            ("transform", transform(0.0, 0.0)),
            ("symbolData", obj(vec![("symbolID", guid(238, 1664))])),
            // A real derived entry forces the override/materialisation
            // path that previously skipped the instance-size scale.
            (
                "derivedSymbolData",
                FigValue::Array(vec![obj(vec![(
                    "guidPath",
                    obj(vec![("guids", FigValue::Array(vec![guid(238, 1665)]))]),
                )])]),
            ),
            ("uniformScaleFactor", FigValue::Float(1.0 / 3.0)),
        ]),
        children: vec![],
    };

    let mut ctx = fresh_ctx();
    ctx.layout_mode = FigLayoutMode::Preserve;
    ctx.blobs = vec![
        BlobOrString::Bytes(captured_bytes(TESLA_AUX_BG_NETWORK)),
        BlobOrString::Bytes(captured_bytes(MINIMAL_24PX_COMMAND_GEOMETRY)),
    ];
    ctx.symbol_tree.insert("238:1664".into(), &symbol);

    let PenNode::Frame(frame) = convert_instance(&instance, None, &mut ctx) else {
        panic!("instance must materialise as a frame");
    };
    assert_eq!(frame.container.width, Some(SizingBehavior::Number(40.0)));
    assert_eq!(frame.container.height, Some(SizingBehavior::Number(40.0)));

    let PenNode::Path(path) = child(&frame, "aux-bg") else {
        panic!("aux-bg must stay a path");
    };
    assert_eq!((path.base.x, path.base.y), (Some(20.0), Some(20.0)));
    assert_eq!(path.width, Some(SizingBehavior::Number(16.0)));
    assert_eq!(path.height, Some(SizingBehavior::Number(16.0)));
    let bounds = compute_svg_path_bounds(path.d.as_deref().expect("vector d")).expect("bounds");
    assert!(
        bounds.max_x <= 16.001 && bounds.max_y <= 16.001,
        "{bounds:?}"
    );
    let StrokeThickness::Uniform(stroke) = path.stroke.as_ref().expect("stroke").thickness else {
        panic!("uniform vector stroke expected");
    };
    assert!((stroke - 0.3).abs() < 0.001, "stroke={stroke}");

    let PenNode::Path(direct) = child(&frame, "direct-geometry") else {
        panic!("direct command geometry must stay a path");
    };
    assert_eq!((direct.base.x, direct.base.y), (Some(30.0), Some(0.0)));
    assert_eq!(direct.width, Some(SizingBehavior::Number(8.0)));
    assert_eq!(direct.height, Some(SizingBehavior::Number(8.0)));
    let direct_bounds =
        compute_svg_path_bounds(direct.d.as_deref().expect("direct vector d")).expect("bounds");
    assert_eq!((direct_bounds.min_x, direct_bounds.min_y), (0.0, 0.0));
    assert!(
        (direct_bounds.max_x - 8.0).abs() < 0.001 && (direct_bounds.max_y - 8.0).abs() < 0.001,
        "{direct_bounds:?}"
    );

    let PenNode::Rectangle(rect) = child(&frame, "scaled-corners") else {
        panic!("rounded child must stay a rectangle");
    };
    assert_eq!((rect.base.x, rect.base.y), (Some(4.0), Some(4.0)));
    assert_eq!(rect.container.width, Some(SizingBehavior::Number(10.0)));
    assert_eq!(rect.container.height, Some(SizingBehavior::Number(10.0)));
    assert_eq!(
        rect.container.corner_radius,
        Some(CornerRadius::PerCorner([2.0, 3.0, 4.0, 5.0]))
    );
    assert_eq!(
        rect.container
            .stroke
            .as_ref()
            .expect("per-side stroke")
            .thickness,
        StrokeThickness::PerSide([1.0, 2.0, 3.0, 4.0])
    );

    let PenNode::Text(text) = child(&frame, "scaled-label") else {
        panic!("text child must stay text");
    };
    assert_eq!((text.base.x, text.base.y), (Some(10.0), Some(30.0)));
    assert_eq!(text.width, Some(SizingBehavior::Number(20.0)));
    assert_eq!(text.height, Some(SizingBehavior::Number(6.0)));
    // Text metrics stay as authored: resizing an instance reflows the
    // text box in Figma, it does not restyle the type. Only the box
    // (x/y/width/height above) follows the instance scale.
    assert_eq!(text.font_size_px(), Some(18.0));
    assert_eq!(text.line_height, Some(1.333));
    assert_eq!(text.letter_spacing, Some(3.0));
    let TextContent::Styled(runs) = &text.content else {
        panic!("style-run fixture must stay styled");
    };
    assert_eq!(runs[0].font_size, Some(12.0));

    clear_icon_lookup();
}
