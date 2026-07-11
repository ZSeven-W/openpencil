use super::*;
use crate::layout_scene::{
    SceneFillType, SceneGradient, SceneGradientStop, SceneStroke, SceneStrokeAlign,
};

fn scene_with(children: Vec<SceneNode>) -> LayoutScene {
    LayoutScene {
        pages: vec![ScenePage {
            id: "p1".into(),
            name: "Page 1".into(),
            children,
        }],
        active_page_index: 0,
    }
}

fn filled_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> SceneNode {
    let mut n = SceneNode::leaf(id, NodeKind::Rect);
    n.bounds = Rect::xywh(x, y, w, h);
    n.fill = Some(fill);
    n
}

fn view_box(svg: &str) -> [f32; 4] {
    let value = svg
        .split_once("viewBox=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
        .expect("viewBox attribute");
    let values = value
        .split_whitespace()
        .map(|part| part.parse::<f32>().expect("viewBox number"))
        .collect::<Vec<_>>();
    values.try_into().expect("four viewBox numbers")
}

#[test]
fn active_page_svg_contains_vector_markup() {
    let scene = scene_with(vec![filled_rect(
        "r1",
        5.0,
        5.0,
        120.0,
        60.0,
        Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 1.0,
        },
    )]);
    let body = serialize_active_page_svg(&scene).expect("svg");
    assert!(body.starts_with("<svg "), "missing svg root: {body}");
    assert!(body.contains("<rect "), "missing rect element: {body}");
    assert!(body.contains(r#"width="120""#), "rect width wrong: {body}");
    assert!(body.ends_with("</svg>"), "missing svg close: {body}");
}

#[test]
fn active_page_svg_fails_on_empty_scene() {
    assert_eq!(
        serialize_active_page_svg(&scene_with(Vec::new())).unwrap_err(),
        "nothing to export"
    );
}

#[test]
fn selected_node_svg_crops_to_subtree_and_excludes_siblings() {
    let mut group = SceneNode::leaf("group", NodeKind::Group);
    group
        .children
        .push(filled_rect("inside", 40.0, 50.0, 20.0, 10.0, Color::RED));
    let scene = scene_with(vec![
        group,
        filled_rect("outside", 500.0, 500.0, 20.0, 20.0, Color::BLACK),
    ]);
    let body = serialize_node_svg(&scene, "group").expect("selected node svg");
    assert!(body.contains(r#"viewBox="40 50 20 10""#), "{body}");
    assert!(body.contains("inside"), "selected subtree missing: {body}");
    assert!(
        !body.contains("outside"),
        "sibling leaked into export: {body}"
    );
}

#[test]
fn svg_emits_gradient_image_clip_and_original_path() {
    let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(10.0, 20.0, 120.0, 80.0);
    frame.corner_radius = 12.0;
    frame.clip_content = true;
    frame.fill_type = SceneFillType::LinearGradient;
    frame.gradient = Some(SceneGradient::Linear {
        angle_deg: 90.0,
        opacity: 1.0,
        stops: vec![
            SceneGradientStop {
                offset: 0.0,
                color: Color::RED,
            },
            SceneGradientStop {
                offset: 1.0,
                color: Color::BLACK,
            },
        ],
    });
    let mut image = SceneNode::leaf("image", NodeKind::Other("image".into()));
    image.bounds = frame.bounds;
    image.image_src = Some("data:image/png;base64,AAAA".into());
    let mut path = SceneNode::leaf("path", NodeKind::Path);
    path.bounds = Rect::xywh(10.0, 20.0, 20.0, 20.0);
    path.fill = Some(Color::RED);
    path.svg_path = Some("M0 0 C5 0 10 5 20 20 Z".into());
    frame.children = vec![image, path];
    let body = serialize_node_svg(&scene_with(vec![frame]), "frame").expect("svg");
    assert!(body.contains("<linearGradient"), "{body}");
    assert!(body.contains("clipPath"), "{body}");
    assert!(body.contains(r#"<image id="image""#), "{body}");
    assert!(body.contains(r#"d="M0 0 C5 0 10 5 20 20 Z""#), "{body}");
    assert!(body.contains(r#"rx="12""#), "{body}");
}

#[test]
fn svg_path_export_matches_canvas_tight_bounds_fitting() {
    let mut path = SceneNode::leaf("scaled-path", NodeKind::Path);
    path.bounds = Rect::xywh(100.0, 200.0, 1.0, 1.0);
    path.fill = Some(Color::RED);
    path.svg_path = Some("M0.2 0.1 L0.8 0.9".into());

    let body = serialize_node_svg(&scene_with(vec![path]), "scaled-path").expect("svg");

    // Intentional canvas parity: native and CanvasKit `fill_svg_path_in_rect`
    // fit the path's tight geometry bounds into the full destination rect.
    // Therefore a localized 0.2..0.8 inset is deliberately NOT preserved.
    assert!(body.contains(r#"d="M0.2 0.1 L0.8 0.9""#), "{body}");
    assert!(
        body.contains(r#"transform="matrix(1.6666666 0 0 1.25 99.666664 199.875)""#),
        "export diverged from canvas tight-bounds fitting: {body}"
    );
}

#[test]
fn fitted_svg_path_keeps_stroke_width_in_document_space() {
    let mut path = SceneNode::leaf("stroked-scaled-path", NodeKind::Path);
    path.bounds = Rect::xywh(100.0, 200.0, 80.0, 40.0);
    path.svg_path = Some("M10 20 L30 60".into());
    path.stroke = Some(SceneStroke {
        color: Color::BLACK,
        width: 4.0,
        sides: None,
        align: SceneStrokeAlign::Center,
    });

    let body = serialize_node_svg(&scene_with(vec![path]), "stroked-scaled-path").expect("svg");

    assert!(body.contains(r#"stroke-width="4""#), "{body}");
    assert!(
        body.contains(r#"vector-effect="non-scaling-stroke""#),
        "{body}"
    );
}

#[test]
fn linear_gradient_uses_document_space_endpoints_on_tall_bounds() {
    let mut rect = SceneNode::leaf("linear-tall", NodeKind::Rect);
    rect.bounds = Rect::xywh(10.0, 20.0, 100.0, 200.0);
    rect.gradient = Some(SceneGradient::Linear {
        angle_deg: 45.0,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });

    let body = serialize_node_svg(&scene_with(vec![rect]), "linear-tall").expect("svg");

    assert!(
        body.contains(r#"<linearGradient id="gradient-n-6c696e6561722d74616c6c" gradientUnits="userSpaceOnUse""#),
        "{body}"
    );
    assert!(
        !body.contains(r#"x1="0.""#),
        "fractional object-space endpoint leaked: {body}"
    );
}

#[test]
fn radial_gradient_uses_document_space_circle_on_tall_bounds() {
    let mut rect = SceneNode::leaf("radial-tall", NodeKind::Rect);
    rect.bounds = Rect::xywh(10.0, 20.0, 100.0, 200.0);
    rect.gradient = Some(SceneGradient::Radial {
        cx: 0.5,
        cy: 0.5,
        radius: 0.5,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });

    let body = serialize_node_svg(&scene_with(vec![rect]), "radial-tall").expect("svg");

    assert!(body.contains(r#"gradientUnits="userSpaceOnUse""#), "{body}");
    assert!(body.contains(r#"cx="60" cy="120" r="100""#), "{body}");
}

#[test]
fn gradient_only_frame_emits_its_background_rect() {
    let mut frame = SceneNode::leaf("gradient-frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(10.0, 20.0, 100.0, 60.0);
    frame.gradient = Some(SceneGradient::Linear {
        angle_deg: 90.0,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });

    let body = serialize_node_svg(&scene_with(vec![frame]), "gradient-frame").expect("svg");

    assert!(body.contains(r#"<rect id="gradient-frame""#), "{body}");
    assert!(body.contains(r#"fill="url(#gradient-"#), "{body}");
}

#[test]
fn resource_ids_do_not_collide_after_svg_encoding() {
    let gradient = || SceneGradient::Linear {
        angle_deg: 90.0,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    };
    let mut colon = SceneNode::leaf("a:b", NodeKind::Rect);
    colon.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
    colon.gradient = Some(gradient());
    let mut dash = SceneNode::leaf("a-b", NodeKind::Rect);
    dash.bounds = Rect::xywh(20.0, 0.0, 10.0, 10.0);
    dash.gradient = Some(gradient());

    let body = serialize_active_page_svg(&scene_with(vec![colon, dash])).expect("svg");

    assert!(body.contains(r#"id="gradient-n-613a62""#), "{body}");
    assert!(body.contains(r#"id="gradient-n-612d62""#), "{body}");
    assert!(body.contains(r#"fill="url(#gradient-n-613a62)""#), "{body}");
    assert!(body.contains(r#"fill="url(#gradient-n-612d62)""#), "{body}");
}

#[test]
fn open_point_path_without_stroke_uses_implicit_fill_color_stroke() {
    let mut path = SceneNode::leaf("open-path", NodeKind::Path);
    path.points = vec![Point2D::new(1.0, 2.0), Point2D::new(11.0, 12.0)];
    path.fill = Some(Color::RED);

    let body = serialize_node_svg(&scene_with(vec![path]), "open-path").expect("svg");

    assert!(body.contains(r#"fill="none""#), "{body}");
    assert!(body.contains(r#"stroke="rgb(255,0,0)""#), "{body}");
    assert!(body.contains(r#"stroke-width="1.5""#), "{body}");
}

#[test]
fn tile_image_uses_repeating_svg_pattern() {
    let mut image = SceneNode::leaf("tile:image", NodeKind::Other("image".into()));
    image.bounds = Rect::xywh(10.0, 20.0, 240.0, 160.0);
    // PNG signature + IHDR declaring a 32x18 intrinsic size.
    image.image_src = Some("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAS".into());
    image.image_fit = crate::layout_scene::SceneImageFit::Tile;

    let body = serialize_node_svg(&scene_with(vec![image]), "tile:image").expect("svg");

    assert!(
        body.contains(r#"<pattern id="image-pattern-n-74696c653a696d616765""#),
        "{body}"
    );
    assert!(body.contains(r#"patternUnits="userSpaceOnUse""#), "{body}");
    assert!(
        body.contains(r#"x="114" y="91" width="32" height="18""#),
        "tile phase or intrinsic dimensions are wrong: {body}"
    );
    assert!(body.contains(r#"<image width="32" height="18""#), "{body}");
    assert!(
        body.contains(r#"fill="url(#image-pattern-n-74696c653a696d616765)""#),
        "{body}"
    );
}

#[test]
fn tile_image_reads_jpeg_and_webp_intrinsic_dimensions() {
    for (id, src) in [
        (
            "jpeg-tile",
            "data:image/jpeg;base64,/9j/wAARCAASACADAREAAhEAAxEA/9k=",
        ),
        (
            "webp-tile",
            "data:image/webp;base64,UklGRh4AAABXRUJQVlA4WAoAAAAAAAAAHwAAEQAA",
        ),
    ] {
        let mut image = SceneNode::leaf(id, NodeKind::Other("image".into()));
        image.bounds = Rect::xywh(0.0, 0.0, 240.0, 160.0);
        image.image_src = Some(src.into());
        image.image_fit = crate::layout_scene::SceneImageFit::Tile;

        let body = serialize_node_svg(&scene_with(vec![image]), id).expect("svg");

        assert!(
            body.contains(r#"x="104" y="71" width="32" height="18""#),
            "{id} intrinsic dimensions or centered phase are wrong: {body}"
        );
    }
}

#[test]
fn rounded_regular_and_tile_images_reference_rounded_clip_paths() {
    let mut regular = SceneNode::leaf("rounded-image", NodeKind::Other("image".into()));
    regular.bounds = Rect::xywh(10.0, 20.0, 120.0, 80.0);
    regular.corner_radius = 12.0;
    regular.image_src = Some("data:image/png;base64,AAAA".into());

    let mut tile = SceneNode::leaf("rounded-tile", NodeKind::Other("image".into()));
    tile.bounds = Rect::xywh(150.0, 20.0, 120.0, 80.0);
    tile.corner_radius = 9.0;
    tile.image_src = Some("data:image/png;base64,BBBB".into());
    tile.image_fit = crate::layout_scene::SceneImageFit::Tile;

    let body = serialize_active_page_svg(&scene_with(vec![regular, tile])).expect("svg");

    assert!(
        body.contains(
            r#"<clipPath id="clip-n-726f756e6465642d696d616765"><rect x="10" y="20" width="120" height="80" rx="12"/></clipPath>"#
        ),
        "{body}"
    );
    assert!(
        body.contains(
            r#"<clipPath id="clip-n-726f756e6465642d74696c65"><rect x="150" y="20" width="120" height="80" rx="9"/></clipPath>"#
        ),
        "{body}"
    );
    assert!(
        body.contains(r#"<image id="rounded-image"#)
            && body.contains(r#"clip-path="url(#clip-n-726f756e6465642d696d616765)""#),
        "{body}"
    );
    assert!(
        body.contains(r#"<rect id="rounded-tile"#)
            && body.contains(r#"clip-path="url(#clip-n-726f756e6465642d74696c65)""#),
        "{body}"
    );
}

#[test]
fn multiline_text_emits_node_id_once() {
    let mut text = SceneNode::leaf("multiline", NodeKind::Text);
    text.bounds = Rect::xywh(0.0, 0.0, 120.0, 60.0);
    text.text = Some("first\nsecond".into());

    let body = serialize_node_svg(&scene_with(vec![text]), "multiline").expect("svg");

    assert_eq!(body.matches(r#"id="multiline""#).count(), 1, "{body}");
    assert!(body.contains("first"), "{body}");
    assert!(body.contains("second"), "{body}");
}

#[test]
fn gradient_only_svg_path_contributes_painted_bounds() {
    let mut path = SceneNode::leaf("gradient-path", NodeKind::Path);
    path.bounds = Rect::xywh(30.0, 40.0, 80.0, 60.0);
    path.svg_path = Some("M0 0 L10 0 L10 10 Z".into());
    path.gradient = Some(SceneGradient::Linear {
        angle_deg: 90.0,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });

    let body = serialize_node_svg(&scene_with(vec![path]), "gradient-path").expect("svg");

    assert!(body.contains(r#"viewBox="30 40 80 60""#), "{body}");
    assert!(body.contains(r#"fill="url(#gradient-"#), "{body}");
}

#[test]
fn clip_content_wraps_children_but_not_frame_stroke() {
    let mut frame = SceneNode::leaf("clipped-frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(10.0, 20.0, 100.0, 80.0);
    frame.clip_content = true;
    frame.stroke = Some(SceneStroke {
        color: Color::BLACK,
        width: 8.0,
        sides: None,
        align: SceneStrokeAlign::Center,
    });
    frame.children.push(filled_rect(
        "clipped-child",
        0.0,
        0.0,
        200.0,
        200.0,
        Color::RED,
    ));

    let body = serialize_node_svg(&scene_with(vec![frame]), "clipped-frame").expect("svg");
    let frame_rect = body
        .find(r#"<rect id="clipped-frame""#)
        .expect("frame rect");
    let clip_group = body
        .find(r#"<g clip-path="url(#clip-"#)
        .expect("clip group");
    let child = body
        .find(r#"<rect id="clipped-child""#)
        .expect("child rect");

    assert!(
        frame_rect < clip_group,
        "frame stroke is inside clip group: {body}"
    );
    assert!(clip_group < child, "child is outside clip group: {body}");
}

#[test]
fn selected_clipping_node_crops_view_box_to_visible_child_bounds() {
    let mut frame = SceneNode::leaf("selected-clip", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    frame.clip_content = true;
    frame.children.push(filled_rect(
        "partially-visible-child",
        80.0,
        20.0,
        60.0,
        30.0,
        Color::RED,
    ));

    let body = serialize_node_svg(&scene_with(vec![frame]), "selected-clip").expect("svg");

    assert!(body.contains(r#"viewBox="80 20 20 30""#), "{body}");
    assert!(
        body.contains(r#"clip-path="url(#clip-n-73656c65637465642d636c6970)""#),
        "{body}"
    );
}

#[test]
fn selected_node_svg_preserves_ancestor_rotation_without_exporting_ancestor_or_siblings() {
    let mut parent = SceneNode::leaf("rotated-parent", NodeKind::Frame);
    parent.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    parent.rotation = std::f32::consts::FRAC_PI_2;
    parent.children = vec![
        filled_rect("selected-child", 10.0, 20.0, 20.0, 10.0, Color::RED),
        filled_rect("sibling", 80.0, 80.0, 10.0, 10.0, Color::BLACK),
    ];

    let body = serialize_node_svg(&scene_with(vec![parent]), "selected-child").expect("svg");
    let [x, y, width, height] = view_box(&body);

    assert!(
        (x - 70.0).abs() < 0.001,
        "ancestor rotation missing: {body}"
    );
    assert!(
        (y - 10.0).abs() < 0.001,
        "ancestor rotation missing: {body}"
    );
    assert!((width - 10.0).abs() < 0.001, "rotated width wrong: {body}");
    assert!(
        (height - 20.0).abs() < 0.001,
        "rotated height wrong: {body}"
    );
    assert!(body.contains(r#"<g transform="matrix("#), "{body}");
    assert!(body.contains(r#"id="selected-child""#), "{body}");
    assert!(
        !body.contains("rotated-parent"),
        "ancestor paint leaked: {body}"
    );
    assert!(!body.contains("sibling"), "sibling leaked: {body}");
}

#[test]
fn selected_node_svg_preserves_ancestor_flip() {
    let mut parent = SceneNode::leaf("flipped-parent", NodeKind::Frame);
    parent.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    parent.flip_x = true;
    parent.children.push(filled_rect(
        "flipped-child",
        10.0,
        20.0,
        20.0,
        10.0,
        Color::RED,
    ));

    let body = serialize_node_svg(&scene_with(vec![parent]), "flipped-child").expect("svg");
    let [x, y, width, height] = view_box(&body);

    assert!((x - 70.0).abs() < 0.001, "ancestor flip missing: {body}");
    assert!((y - 20.0).abs() < 0.001, "flipped y changed: {body}");
    assert!((width - 20.0).abs() < 0.001, "flipped width wrong: {body}");
    assert!(
        (height - 10.0).abs() < 0.001,
        "flipped height wrong: {body}"
    );
    assert!(
        body.contains(r#"transform="matrix(-1 0 0 1 100 0)""#),
        "{body}"
    );
}

#[test]
fn selected_node_svg_preserves_ancestor_content_clip() {
    let mut parent = SceneNode::leaf("clip-parent", NodeKind::Frame);
    parent.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    parent.clip_content = true;
    parent.corner_radius = 12.0;
    parent.children.push(filled_rect(
        "clipped-selection",
        80.0,
        20.0,
        60.0,
        30.0,
        Color::RED,
    ));

    let body = serialize_node_svg(&scene_with(vec![parent]), "clipped-selection").expect("svg");

    assert!(body.contains(r#"viewBox="80 20 20 30""#), "{body}");
    assert!(
        body.contains(r#"<clipPath id="ancestor-clip-n-636c69702d706172656e74"#),
        "{body}"
    );
    assert!(
        body.contains(r#"clip-path="url(#ancestor-clip-n-636c69702d706172656e74)""#),
        "{body}"
    );
    assert!(body.contains(r#"rx="12""#), "{body}");
    assert!(body.contains(r#"id="clipped-selection""#), "{body}");
    assert!(!body.contains(r#"id="clip-parent""#), "{body}");
}

#[test]
fn gradient_only_other_node_contributes_bounds_and_emits_rect() {
    let mut other = SceneNode::leaf("gradient-other", NodeKind::Other("custom".into()));
    other.bounds = Rect::xywh(30.0, 40.0, 80.0, 60.0);
    other.gradient = Some(SceneGradient::Linear {
        angle_deg: 90.0,
        opacity: 1.0,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });

    let body = serialize_node_svg(&scene_with(vec![other]), "gradient-other").expect("svg");

    assert!(body.contains(r#"viewBox="30 40 80 60""#), "{body}");
    assert!(body.contains(r#"<rect id="gradient-other""#), "{body}");
    assert!(body.contains(r#"fill="url(#gradient-"#), "{body}");
}

#[test]
fn resolved_layer_opacity_is_not_applied_twice_to_gradient_stroke_or_descendants() {
    let mut frame = SceneNode::leaf("opacity-frame", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    // LayoutScene stores cumulative opacity here for raster images.
    frame.opacity = 0.5;
    frame.gradient = Some(SceneGradient::Linear {
        angle_deg: 90.0,
        // The loader has already folded the same cumulative node opacity
        // into the gradient multiplier used by the canvas shader.
        opacity: 0.5,
        stops: vec![SceneGradientStop {
            offset: 0.0,
            color: Color::RED,
        }],
    });
    frame.stroke = Some(SceneStroke {
        color: Color::BLACK.with_alpha(0.5),
        width: 2.0,
        sides: None,
        align: SceneStrokeAlign::Center,
    });
    frame.children.push(filled_rect(
        "opacity-child",
        10.0,
        10.0,
        20.0,
        20.0,
        Color::RED.with_alpha(0.5),
    ));

    let body = serialize_node_svg(&scene_with(vec![frame]), "opacity-frame").expect("svg");

    assert!(body.contains(r#"stop-opacity="0.5""#), "{body}");
    assert!(body.contains(r#"stroke-opacity="0.5""#), "{body}");
    assert!(body.contains(r#"id="opacity-child""#), "{body}");
    assert!(body.contains(r#"fill-opacity="0.5""#), "{body}");
    assert!(
        !body.contains(r#"<g opacity="0.5""#),
        "cumulative layer opacity was applied twice: {body}"
    );
}
