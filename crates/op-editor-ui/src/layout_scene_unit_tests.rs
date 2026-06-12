//! Sibling unit tests for `layout_scene.rs` (800-line cap
//! convention).

use super::*;

#[test]
fn empty_scene_has_no_active_page() {
    let scene = LayoutScene::default();
    assert!(scene.pages.is_empty());
    assert!(scene.active_page().is_none());
}

#[test]
fn active_page_indexes_into_pages() {
    let scene = LayoutScene {
        pages: vec![
            ScenePage {
                id: "a".into(),
                name: "A".into(),
                children: Vec::new(),
            },
            ScenePage {
                id: "b".into(),
                name: "B".into(),
                children: Vec::new(),
            },
        ],
        active_page_index: 1,
    };
    assert_eq!(scene.active_page().map(|p| p.id.as_str()), Some("b"));
}

#[test]
fn find_locates_a_nested_node() {
    let mut leaf = SceneNode::leaf("deep", NodeKind::Rect);
    leaf.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
    let mut group = SceneNode::leaf("g", NodeKind::Group);
    group.children = vec![leaf];
    let page = ScenePage {
        id: "p".into(),
        name: "P".into(),
        children: vec![group],
    };
    assert_eq!(page.find("deep").map(|n| n.id.as_str()), Some("deep"));
    assert!(page.find("missing").is_none());
}

#[test]
fn aggregate_bounds_unions_children_for_unbounded_container() {
    let mut a = SceneNode::leaf("a", NodeKind::Rect);
    a.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
    let mut b = SceneNode::leaf("b", NodeKind::Rect);
    b.bounds = Rect::xywh(50.0, 5.0, 10.0, 40.0);
    let mut group = SceneNode::leaf("g", NodeKind::Group);
    group.children = vec![a, b];
    // Unbounded group → union of children: x 10..60, y 5..45.
    assert_eq!(group.aggregate_bounds(), Rect::xywh(10.0, 5.0, 50.0, 40.0));
}

#[test]
fn aggregate_bounds_keeps_own_bounds_when_bounded() {
    let mut frame = SceneNode::leaf("f", NodeKind::Frame);
    frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 200.0);
    let mut child = SceneNode::leaf("c", NodeKind::Rect);
    child.bounds = Rect::xywh(0.0, 0.0, 999.0, 999.0);
    frame.children = vec![child];
    assert_eq!(frame.aggregate_bounds(), Rect::xywh(0.0, 0.0, 100.0, 200.0));
}

#[test]
fn translate_nodes_moves_matching_subtree_once() {
    let mut child = SceneNode::leaf("child", NodeKind::Rect);
    child.bounds = Rect::xywh(10.0, 20.0, 30.0, 40.0);
    let mut parent = SceneNode::leaf("parent", NodeKind::Group);
    parent.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    parent.children = vec![child];
    let mut scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![parent],
        }],
        active_page_index: 0,
    };

    assert!(scene.translate_nodes(&["parent".into(), "child".into()], 5.0, 7.0));
    let page = scene.active_page().expect("active page");
    let parent = page.find("parent").expect("parent");
    let child = page.find("child").expect("child");
    assert_eq!(parent.bounds.origin, Point2D::new(5.0, 7.0));
    assert_eq!(child.bounds.origin, Point2D::new(15.0, 27.0));
}

#[test]
fn translate_nodes_moves_path_absolute_geometry() {
    let mut path = SceneNode::leaf("path", NodeKind::Path);
    path.bounds = Rect::xywh(1.0, 2.0, 30.0, 40.0);
    path.points = vec![Point2D::new(3.0, 4.0)];
    path.path_anchors = vec![SceneAnchor {
        pos: Point2D::new(5.0, 6.0),
        handle_in: Some(Point2D::new(7.0, 8.0)),
        handle_out: Some(Point2D::new(9.0, 10.0)),
        point_type: ScenePointType::Corner,
    }];
    let mut scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![path],
        }],
        active_page_index: 0,
    };

    assert!(scene.translate_nodes(&["path".into()], 11.0, 13.0));
    let path = scene.active_page().and_then(|p| p.find("path")).unwrap();
    assert_eq!(path.bounds.origin, Point2D::new(12.0, 15.0));
    assert_eq!(path.points[0], Point2D::new(14.0, 17.0));
    assert_eq!(path.path_anchors[0].pos, Point2D::new(16.0, 19.0));
    assert_eq!(
        path.path_anchors[0].handle_in,
        Some(Point2D::new(18.0, 21.0))
    );
    assert_eq!(
        path.path_anchors[0].handle_out,
        Some(Point2D::new(20.0, 23.0))
    );
}

#[test]
fn leaf_node_clears_paint_fields() {
    let n = SceneNode::leaf("n1", NodeKind::Rect);
    assert_eq!(n.bounds, Rect::ZERO);
    assert!(n.fill.is_none());
    assert!(n.stroke.is_none());
    assert!(n.children.is_empty());
    assert_eq!(n.fill_type, SceneFillType::Solid);
}

#[test]
fn content_bounds_unions_top_level_nodes() {
    let mut a = SceneNode::leaf("a", NodeKind::Rect);
    a.bounds = Rect::xywh(10.0, 20.0, 30.0, 40.0); // → x[10,40] y[20,60]
    let mut b = SceneNode::leaf("b", NodeKind::Rect);
    b.bounds = Rect::xywh(100.0, 0.0, 50.0, 10.0); // → x[100,150] y[0,10]
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![a, b],
        }],
        active_page_index: 0,
    };
    let bounds = scene.content_bounds().expect("non-empty page has bounds");
    // Union: x[10,150] y[0,60] → origin (10,0) size (140,60).
    assert_eq!(bounds, Rect::xywh(10.0, 0.0, 140.0, 60.0));
}

#[test]
fn content_bounds_none_for_empty_page() {
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "P".into(),
            children: vec![],
        }],
        active_page_index: 0,
    };
    assert!(scene.content_bounds().is_none());
}
