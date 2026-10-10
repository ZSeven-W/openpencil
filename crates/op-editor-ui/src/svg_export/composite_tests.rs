use super::serialize_active_page_svg;
use crate::layout_scene::{
    LayoutScene, NodeKind, SceneNode, ScenePage, SceneWidget, SceneWidgetOption,
};
use crate::{Color, Rect};

fn export(children: Vec<SceneNode>) -> String {
    serialize_active_page_svg(&LayoutScene {
        pages: vec![ScenePage {
            id: "page".into(),
            name: "Page".into(),
            children,
        }],
        active_page_index: 0,
    })
    .unwrap()
}

fn widget(kind: &str, value: Option<&str>) -> SceneNode {
    let mut node = SceneNode::leaf("control", NodeKind::Text);
    node.bounds = Rect::xywh(20.0, 30.0, 180.0, 36.0);
    node.fill = Some(Color::WHITE);
    node.text = value.map(str::to_string);
    node.widget = Some(SceneWidget {
        kind: kind.into(),
        value_str: value.map(str::to_string),
        ..Default::default()
    });
    node
}

#[test]
fn svg_icon_uses_catalog_paths_at_canvas_size_and_opacity() {
    let mut node = SceneNode::leaf("icon", NodeKind::Other("icon_font".into()));
    node.bounds = Rect::xywh(20.0, 30.0, 18.0, 26.0);
    node.font_family = "lucide".into();
    node.text = Some("home".into());
    node.fill = Some(Color {
        a: 0.5,
        ..Color::BLACK
    });
    let svg = export(vec![node]);
    assert!(svg.contains("<path "), "{svg}");
    assert!(
        !svg.contains("<rect "),
        "icons must not become filled squares: {svg}"
    );
    assert!(svg.contains("translate(20 34) scale(0.75)"), "{svg}");
    assert!(svg.contains("stroke-opacity=\"0.5\""), "{svg}");
    assert!(svg.contains("stroke-linejoin=\"round\""));
}

#[test]
fn svg_unknown_icon_matches_the_unpainted_canvas_fallback() {
    let mut icon = SceneNode::leaf("unknown", NodeKind::Other("icon_font".into()));
    icon.bounds = Rect::xywh(10.0, 10.0, 20.0, 20.0);
    icon.fill = Some(Color::RED);
    icon.text = Some("not-an-op-icon".into());
    let mut background = SceneNode::leaf("background", NodeKind::Rect);
    background.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
    background.fill = Some(Color::WHITE);
    let svg = export(vec![background, icon]);
    assert!(!svg.contains("fill=\"rgb(255,0,0)\""), "{svg}");
}

#[test]
fn svg_select_exports_option_label_and_affordance_instead_of_internal_value() {
    for value in ["all", "__op_all"] {
        let mut node = widget("select", Some(value));
        node.widget.as_mut().unwrap().options = vec![SceneWidgetOption {
            value: value.into(),
            label: "All <roles> & teams".into(),
        }];
        let svg = export(vec![node]);
        assert!(svg.contains("All &lt;roles&gt; &amp; teams"), "{svg}");
        assert!(!svg.contains(&format!(">{value}</text>")), "{svg}");
        assert!(svg.contains("rx=\"6\""), "select surface missing: {svg}");
        assert_eq!(svg.matches("<line ").count(), 2, "chevron missing: {svg}");
    }
}

#[test]
fn svg_inputs_preserve_placeholder_and_leading_icon() {
    let mut node = widget("text_input", None);
    let props = node.widget.as_mut().unwrap();
    props.placeholder = Some("Search users…".into());
    props.leading_icon = Some("search".into());
    let svg = export(vec![node]);
    assert!(svg.contains("Search users…"), "{svg}");
    assert!(svg.contains("<path "), "search glyph missing: {svg}");
    assert!(
        svg.contains("<text x=\"56\""),
        "text must clear the leading icon: {svg}"
    );
}

#[test]
fn svg_checked_checkbox_keeps_indicator_and_label() {
    let mut node = widget("checkbox", None);
    node.kind = NodeKind::Rect;
    node.fill = Some(Color::BLACK);
    let props = node.widget.as_mut().unwrap();
    props.checked = Some(true);
    props.label = Some("Remember me".into());
    let svg = export(vec![node]);
    assert!(svg.contains("Remember me"), "{svg}");
    assert_eq!(
        svg.matches("<line ").count(),
        2,
        "checked mark missing: {svg}"
    );
}

#[test]
fn svg_tabs_only_export_the_visible_panel_like_the_canvas() {
    let mut node = widget("tabs", Some("second"));
    node.kind = NodeKind::Frame;
    node.bounds = Rect::xywh(0.0, 0.0, 180.0, 100.0);
    node.widget.as_mut().unwrap().options = ["first", "second"]
        .into_iter()
        .map(|value| SceneWidgetOption {
            value: value.into(),
            label: value.into(),
        })
        .collect();
    for (id, color) in [
        ("inactive-panel", Color::RED),
        ("active-panel", Color::BLACK),
    ] {
        let mut panel = SceneNode::leaf(id, NodeKind::Rect);
        panel.bounds = Rect::xywh(0.0, 40.0, 180.0, 60.0);
        if id == "inactive-panel" {
            panel.bounds.size.x = 1000.0;
        }
        panel.fill = Some(color);
        node.children.push(panel);
    }
    let svg = export(vec![node]);
    assert!(svg.contains("id=\"active-panel\""), "{svg}");
    assert!(!svg.contains("id=\"inactive-panel\""), "{svg}");
    assert!(svg.contains("viewBox=\"-16 -16 212 132\""), "{svg}");
    assert_eq!(svg.matches("id=\"control\"").count(), 1, "{svg}");
    assert!(svg.contains("fill=\"rgb(255,255,255)\""), "{svg}");
}
