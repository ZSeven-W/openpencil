use super::*;

fn load(src: &str) -> LoadedDoc {
    let r = jian_ops_schema::load_str(src).unwrap();
    pen_document_to_payload(&r.value)
}

#[test]
fn minimal_empty_doc() {
    let r = load(r#"{"version":"0.8.0","children":[]}"#);
    assert_eq!(r.payload.pages.len(), 1);
    assert!(r.payload.pages[0].children.is_empty());
}

#[test]
fn fill_container_resolves_via_taffy() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{
        "id":"p1","name":"Page 1",
        "children":[{
          "type":"frame","id":"root","width":375,"height":812,
          "layout":"vertical","gap":16,
          "children":[
            {"type":"rectangle","id":"r1","width":"fill_container","height":40,
             "fill":[{"type":"solid","color":"#000000"}]}
          ]
        }]
      }],
      "children":[]
    }"##;
    let r = load(src);
    let root = &r.payload.pages[0].children[0];
    let inner = &root.children[0];
    assert_eq!(inner.w, 375.0, "fill_container should stretch via taffy");
    assert_eq!(inner.h, 40.0);
}

#[test]
fn path_anchors_absolutize_to_canvas_coords() {
    // Canonical `PathNode.anchors` are authored local to the
    // path's `base.x`/`base.y` *and* scaled to its `width`/
    // `height` per `pen-renderer/node-renderer.ts::drawPath`.
    // The shell renderer treats `Node.points` as canvas-abs,
    // so the adapter must apply `(x + (anchor.x - min_x) * sx,
    // …)` — pure translate misses both non-zero anchor minima
    // and explicit-size scaling.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","x":-1000,"y":2000,"width":400,"height":200,
         "children":[
           {"type":"path","id":"p1","x":20,"y":30,"width":100,"height":50,
            "anchors":[
              {"x":0,"y":0,"handleIn":null,"handleOut":null},
              {"x":100,"y":50,"handleIn":null,"handleOut":null}
            ]}
         ]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let path_node = &r.payload.pages[0].children[0].children[0];
    assert_eq!(path_node.x, -980.0);
    assert_eq!(path_node.y, 2030.0);
    // anchor bbox (0..100, 0..50) == size (100, 50), so sx=sy=1.
    assert_eq!(path_node.points[0], [-980.0, 2030.0]);
    assert_eq!(path_node.points[1], [-880.0, 2080.0]);
}

#[test]
fn path_bounds_include_bezier_curve_extrema() {
    // Two anchors at y=0 with NEGATIVE-y handles (handle.y=-60).
    // The cubic Bezier reaches y=-45 at t=0.5, so native bbox is
    // min_y=-45, max_y=0, native_h=45. Authored height=90 →
    // sy = 90/45 = 2.0. Anchor at native y=0 shifts by
    // (0 - (-45)) = 45, scales by 2 = 90, plus path.y=100 = 190.
    //
    // Endpoint-only bounds would give native_h=0 → sy=1 →
    // anchor.y = path.y = 100. So asserting 190 forces the
    // curve-extrema path through `cubic_derivative_roots`.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"path","id":"curve","x":0,"y":100,"width":100,"height":90,
         "anchors":[
           {"x":0,"y":0,"handleIn":null,"handleOut":{"x":0,"y":-60}},
           {"x":100,"y":0,"handleIn":{"x":0,"y":-60},"handleOut":null}
         ]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let n = &r.payload.pages[0].children[0];
    // `points` stays 1:1 with the two schema anchors; the curve-aware
    // bounds come from `absolutize_path_anchors`'s native span.
    let p0 = n.points[0];
    let p1 = n.points[1];
    // x span [0, 100] → sx=1. Anchors keep their x.
    assert!((p0[0] - 0.0).abs() < 0.01, "anchor[0].x");
    assert!((p1[0] - 100.0).abs() < 0.01, "anchor[1].x");
    // y: 190 with extrema-aware bounds, 100 with endpoint-only.
    assert!(
        (p0[1] - 190.0).abs() < 0.5,
        "anchor[0].y expected ~190 (was {}). Endpoint-only bbox would give 100.",
        p0[1]
    );
    assert!(
        (p1[1] - 190.0).abs() < 0.5,
        "anchor[1].y expected ~190 (was {}). Endpoint-only bbox would give 100.",
        p1[1]
    );
}

#[test]
fn path_anchors_honor_nonzero_minima_and_scale() {
    // Anchors at (10, 20) → (30, 80) with authored width=200,
    // height=300. Native span = (20, 60), so sx=10, sy=5.
    // First anchor maps to path.x/y; second anchor maps to
    // path.x + 200, path.y + 300.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"path","id":"p1","x":50,"y":60,"width":200,"height":300,
         "anchors":[
           {"x":10,"y":20,"handleIn":null,"handleOut":null},
           {"x":30,"y":80,"handleIn":null,"handleOut":null}
         ]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let n = &r.payload.pages[0].children[0];
    assert_eq!(n.points[0], [50.0, 60.0], "first anchor maps to origin");
    assert_eq!(n.points[1], [250.0, 360.0], "scaled to width/height");
}

#[test]
fn shape_inside_offset_root_inherits_canvas_offset() {
    // The zero-size shape fallback used to keep `(x, y)` from
    // `base_payload` (= authored local coords inside the parent
    // frame), so an ellipse at local (20, 30) inside a root at
    // canvas (-1000, 2000) painted at world (20, 30) instead of
    // (-980, 2030) — detaching it from its parent design.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","x":-1000,"y":2000,"width":400,"height":200,
         "children":[
           {"type":"ellipse","id":"dot","x":20,"y":30,"width":40,"height":40}
         ]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let root = &r.payload.pages[0].children[0];
    assert_eq!((root.x, root.y), (-1000.0, 2000.0));
    let dot = &root.children[0];
    assert_eq!(dot.x, -980.0, "ellipse x must reflect root offset");
    assert_eq!(dot.y, 2030.0, "ellipse y must reflect root offset");
    assert_eq!((dot.w, dot.h), (40.0, 40.0), "authored size preserved");
}

#[test]
fn multi_root_designs_use_authored_canvas_offset() {
    // pencil-demo.op's 14 designs sit at distinct (x, y) on the
    // infinite canvas. Each root's taffy layout starts at (0, 0)
    // so we must offset the harvested rects by the root's
    // authored `base.x` / `base.y` — otherwise every design
    // collapses to origin and they all overlap.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"a","x":100,"y":50,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#FF0000"}]},
        {"type":"frame","id":"b","x":-500,"y":2000,"width":200,"height":100,
         "fill":[{"type":"solid","color":"#00FF00"}]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let kids = &r.payload.pages[0].children;
    assert_eq!(kids[0].x, 100.0);
    assert_eq!(kids[0].y, 50.0);
    assert_eq!(kids[1].x, -500.0);
    assert_eq!(kids[1].y, 2000.0);
}

#[test]
fn space_between_preserves_authored_positions() {
    // Mirror pencil-demo.op's sidebar: vertical flex with
    // space_between authored on the parent. Authored positions
    // on children (Top at y=0, Bottom at y=600) must survive
    // the layout pass — jian-core's `node_to_style` treats
    // explicit x/y as `Position::Absolute`.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"frame","id":"sidebar","width":240,"height":800,
        "layout":"vertical","justifyContent":"space_between",
        "children":[
          {"type":"frame","id":"top","width":240,"height":100,"x":0,"y":0},
          {"type":"frame","id":"bot","width":240,"height":100,"x":0,"y":700}
        ]
      }]}],"children":[]
    }"##;
    let r = load(src);
    let sidebar = &r.payload.pages[0].children[0];
    let top = &sidebar.children[0];
    let bot = &sidebar.children[1];
    assert_eq!(top.y, 0.0, "top authored at y=0");
    assert_eq!(
        bot.y, 700.0,
        "bottom authored at y=700 must NOT be restacked"
    );
}

#[test]
fn text_font_size_and_weight_flow_through() {
    // login.op-style headline: 28pt, weight 700.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"hd","content":"Welcome Back",
         "fontSize":28,"fontWeight":700,
         "fill":[{"type":"solid","color":"#0F172A"}]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let n = &r.payload.pages[0].children[0];
    assert_eq!(n.font_size, 28.0);
    assert_eq!(n.font_weight, 700);
}

#[test]
fn text_keyword_weight_resolves() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"text","id":"t","content":"hi","fontWeight":"semibold"}
      ]}],"children":[]
    }"##;
    let r = load(src);
    assert_eq!(r.payload.pages[0].children[0].font_weight, 600);
}

#[test]
fn numeric_string_font_weights_resolve() {
    // pencil-demo.op authors weights as JSON strings — `"fontWeight":"700"`
    // for headlines, `"600"` for navigation chrome, `"500"` etc. The
    // canonical schema picks `FontWeight::Keyword(String)` for every
    // JSON string, so the resolver has to parse numeric keywords back
    // into u16. Otherwise SkiaMeasure picks 400-px glyph advances and
    // "Focus Protocol" (700) measures as regular-weight, mis-sizing
    // every fit_content parent.
    for (encoded, expected) in [
        (r#""700""#, 700u16),
        (r#""600""#, 600),
        (r#""500""#, 500),
        (r#""800""#, 800),
        (r#""900""#, 900),
        (r#""300""#, 300),
        (r#""normal""#, 400),
        (r#""regular""#, 400),
        (r#""bold""#, 700),
        (r#""semibold""#, 600),
        (r#""black""#, 900),
        (r#""thin""#, 100),
    ] {
        let src = format!(
            r##"{{"version":"1.0.0","pages":[{{"id":"p","name":"P","children":[
                {{"type":"text","id":"t","content":"hi","fontWeight":{}}}
            ]}}],"children":[]}}"##,
            encoded
        );
        let r = load(&src);
        let n = &r.payload.pages[0].children[0];
        assert_eq!(
            n.font_weight, expected,
            "fontWeight {} should resolve to {}, got {}",
            encoded, expected, n.font_weight
        );
    }
}

#[test]
fn linear_gradient_falls_back_to_first_stop() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":100,"height":50,
        "fill":[{"type":"linear_gradient","angle":90,
                 "stops":[{"color":"#FF0000","offset":0},
                          {"color":"#0000FF","offset":1}]}]
      }]}],"children":[]
    }"##;
    let r = load(src);
    let n = &r.payload.pages[0].children[0];
    let fill = n.fill.unwrap();
    assert!(fill[0] > 0.99 && fill[1] < 0.01, "first stop is red");
    assert_eq!(n.fill_type, "linear");
}

#[test]
fn shape_nodes_keep_authored_size() {
    // Ellipse + polygon + path don't appear in jian-core's
    // `leaf_size` resolver, so taffy returns Size::ZERO for
    // them. The adapter must fall back to authored width /
    // height — otherwise shapes vanish.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"ellipse","id":"e","x":10,"y":20,"width":80,"height":40,
         "fill":[{"type":"solid","color":"#FF0000"}]},
        {"type":"polygon","id":"poly","x":100,"y":50,"width":60,"height":60,
         "polygonCount":6,"fill":[{"type":"solid","color":"#00FF00"}]},
        {"type":"path","id":"pa","x":200,"y":300,"width":120,"height":50,
         "anchors":[{"x":0,"y":0,"handleIn":null,"handleOut":null}]}
      ]}],"children":[]
    }"##;
    let r = load(src);
    let kids = &r.payload.pages[0].children;
    let e = &kids[0];
    let poly = &kids[1];
    let pa = &kids[2];
    assert_eq!((e.w, e.h), (80.0, 40.0), "ellipse keeps authored size");
    assert_eq!(
        (poly.w, poly.h),
        (60.0, 60.0),
        "polygon keeps authored size"
    );
    assert_eq!((pa.w, pa.h), (120.0, 50.0), "path keeps authored size");
}

#[test]
fn line_uses_signed_bounds() {
    // Vertical line: x2=0, y2=100. Encoded as bounds size
    // (0, 100); pure-axis lines pass `aggregate_bounds`' size>0
    // guard because the y axis is non-zero.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"line","id":"l","x":10,"y":20,"x2":0,"y2":100
      }]}],"children":[]
    }"##;
    let r = load(src);
    let n = &r.payload.pages[0].children[0];
    assert_eq!(n.x, 10.0);
    assert_eq!(n.y, 20.0);
    assert_eq!(n.w, 0.0);
    assert_eq!(n.h, 100.0);
}

#[test]
fn login_op_fixture_loads() {
    let path = "/Users/kayshen/Desktop/login.op";
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let src = std::str::from_utf8(&bytes).unwrap();
    let parsed = jian_ops_schema::load_str(src).unwrap();
    let adapted = pen_document_to_payload(&parsed.value);
    assert_eq!(adapted.payload.pages.len(), 1, "expected 1 page");
    let root = &adapted.payload.pages[0].children[0];
    assert_eq!(root.w, 375.0);
    assert_eq!(root.h, 812.0);
    // The 3 vertical sections (brand / form / social) get
    // stretched by taffy to fill the root width (375 px).
    assert_eq!(root.children.len(), 3);
    for section in &root.children {
        assert!(
            section.w > 300.0,
            "fill_container child should be near root width, got {}",
            section.w
        );
    }
}

#[test]
fn pencil_demo_op_fixture_loads() {
    let path = "/Users/kayshen/Desktop/pencil-demo.op";
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let src = std::str::from_utf8(&bytes).unwrap();
    let parsed = crate::payload::load_canonical(src).expect("canonical load");
    let adapted = pen_document_to_payload(&parsed.value);
    assert!(
        !adapted.payload.pages.is_empty(),
        "expected at least one page"
    );
    let total_nodes: usize = adapted
        .payload
        .pages
        .iter()
        .map(|p| count_nodes(&p.children))
        .sum();
    assert!(
        total_nodes > 10,
        "pencil-demo.op should yield a substantial node tree, got {}",
        total_nodes
    );
    // Any ellipse in the fixture must come out non-zero —
    // taffy's `Size::ZERO` for unmeasured leaves used to wipe
    // authored width/height on shapes.
    let mut ellipse_zero = 0usize;
    let mut ellipse_total = 0usize;
    fn walk_ellipses(nodes: &[crate::payload::NodePayload], total: &mut usize, zero: &mut usize) {
        for n in nodes {
            if n.kind == "ellipse" {
                *total += 1;
                if n.w == 0.0 && n.h == 0.0 {
                    *zero += 1;
                }
            }
            walk_ellipses(&n.children, total, zero);
        }
    }
    for page in &adapted.payload.pages {
        walk_ellipses(&page.children, &mut ellipse_total, &mut ellipse_zero);
    }
    if ellipse_total > 0 {
        assert_eq!(
            ellipse_zero, 0,
            "{}/{} ellipses lost their authored size",
            ellipse_zero, ellipse_total
        );
    }
    fn count_nodes(nodes: &[crate::payload::NodePayload]) -> usize {
        nodes.iter().map(|n| 1 + count_nodes(&n.children)).sum()
    }
}
